#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod features;
mod languages;
mod ui;

use gtk::prelude::*;
use relm4::prelude::*;
use std::{
    ffi::c_void,
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(target_os = "linux")]
#[link(name = "fontconfig")]
unsafe extern "C" {
    fn FcInit() -> i32;
    fn FcConfigAppFontAddDir(config: *mut c_void, dir: *const u8) -> i32;
    fn FcConfigAppFontAddFile(config: *mut c_void, file: *const u8) -> i32;
    fn FcConfigBuildFonts(config: *mut c_void) -> i32;
}

#[cfg(target_os = "windows")]
#[link(name = "gdi32")]
unsafe extern "system" {
    fn AddFontResourceExW(name: *const u16, flags: u32, data: *mut c_void) -> i32;
}

#[cfg(target_os = "macos")]
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFURLCreateFromFileSystemRepresentation(
        allocator: *const c_void,
        buffer: *const u8,
        buffer_length: isize,
        is_directory: u8,
    ) -> *const c_void;
    fn CFRelease(value: *const c_void);
}

#[cfg(target_os = "macos")]
#[link(name = "CoreText", kind = "framework")]
unsafe extern "C" {
    fn CTFontManagerRegisterFontsForURL(
        font_url: *const c_void,
        scope: u32,
        error: *mut *const c_void,
    ) -> u8;
}

#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
fn app_font_directory() -> Option<PathBuf> {
    const FONT_FILES: [&str; 3] = [
        "InterVariable.ttf",
        "JetBrainsMono-Regular.ttf",
        "JetBrainsMono-Bold.ttf",
    ];

    let mut directories = Vec::new();
    if let Ok(executable) = std::env::current_exe()
        && let Some(directory) = executable.parent()
    {
        directories.extend([
            directory.join("fonts"),
            directory.join("data/fonts"),
            directory.join("../share/potoo-ide/fonts"),
            directory.join("../Resources/fonts"),
        ]);
    }
    directories.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/fonts"));

    directories.into_iter().find(|directory| {
        directory.is_dir() && FONT_FILES.iter().all(|font| directory.join(font).is_file())
    })
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn configure_bundled_runtime() {
    let Some(executable_directory) = std::env::current_exe()
        .ok()
        .and_then(|executable| executable.parent().map(Path::to_path_buf))
    else {
        return;
    };

    #[cfg(target_os = "windows")]
    let resource_directory = executable_directory
        .parent()
        .unwrap_or(&executable_directory)
        .to_path_buf();
    #[cfg(target_os = "macos")]
    let resource_directory = executable_directory.join("../Resources");

    let share_directory = resource_directory.join("share");
    if !share_directory.is_dir() {
        return;
    }

    let runtime_key = bundled_runtime_cache_key(&resource_directory);
    let mut data_directories = vec![share_directory.clone()];
    if let Some(existing) = std::env::var_os("XDG_DATA_DIRS") {
        data_directories.extend(std::env::split_paths(&existing));
    }
    #[cfg(target_os = "macos")]
    if std::env::var_os("XDG_DATA_DIRS").is_none() {
        data_directories.extend([
            PathBuf::from("/usr/local/share"),
            PathBuf::from("/usr/share"),
        ]);
    }
    if let Ok(data_directories) = std::env::join_paths(data_directories) {
        set_bundled_environment("XDG_DATA_DIRS", data_directories);
    }

    let schema_directory = share_directory.join("glib-2.0/schemas");
    if schema_directory.is_dir() {
        set_bundled_environment("GSETTINGS_SCHEMA_DIR", schema_directory.into_os_string());
    }

    let library_directory = resource_directory.join("lib");
    set_bundled_environment(
        "GTK_DATA_PREFIX",
        resource_directory.clone().into_os_string(),
    );
    set_bundled_environment(
        "GTK_EXE_PREFIX",
        resource_directory.clone().into_os_string(),
    );

    let gtk_module_directory = library_directory.join("gtk-4.0");
    if gtk_module_directory.is_dir() {
        set_bundled_environment("GTK_PATH", gtk_module_directory.into_os_string());
    }
    let gio_module_directory = library_directory.join("gio/modules");
    if gio_module_directory.is_dir() {
        set_bundled_environment("GIO_EXTRA_MODULES", gio_module_directory.into_os_string());
    }

    let gst_plugin_directory = library_directory.join("gstreamer-1.0");
    if gst_plugin_directory.is_dir() {
        set_bundled_environment(
            "GST_PLUGIN_PATH_1_0",
            gst_plugin_directory.clone().into_os_string(),
        );
        set_bundled_environment(
            "GST_PLUGIN_SYSTEM_PATH_1_0",
            gst_plugin_directory.into_os_string(),
        );
        let scanner = executable_directory.join(if cfg!(target_os = "windows") {
            "gst-plugin-scanner.exe"
        } else {
            "gst-plugin-scanner"
        });
        if scanner.is_file() {
            set_bundled_environment("GST_PLUGIN_SCANNER", scanner.into_os_string());
        }
        let cache_directory = bundled_cache_directory();
        if std::fs::create_dir_all(&cache_directory).is_ok() {
            set_bundled_environment(
                "GST_REGISTRY",
                cache_directory
                    .join(format!(
                        "gstreamer-registry-{}-{runtime_key}.bin",
                        env!("CARGO_PKG_VERSION"),
                    ))
                    .into_os_string(),
            );
        }
    }

    let pixbuf_root = library_directory.join("gdk-pixbuf-2.0/2.10.0");
    let pixbuf_loaders = pixbuf_root.join("loaders");
    if pixbuf_loaders.is_dir() {
        let cache_directory = bundled_cache_directory();
        let cache_file = cache_directory.join(format!(
            "gdk-pixbuf-loaders-{}-{runtime_key}.cache",
            env!("CARGO_PKG_VERSION"),
        ));
        if !cache_file.is_file() {
            let helper = executable_directory.join(if cfg!(target_os = "windows") {
                "gdk-pixbuf-query-loaders.exe"
            } else {
                "gdk-pixbuf-query-loaders"
            });
            let mut loaders = match std::fs::read_dir(&pixbuf_loaders) {
                Ok(entries) => entries
                    .flatten()
                    .map(|entry| entry.path())
                    .filter(|path| {
                        matches!(
                            path.extension().and_then(|extension| extension.to_str()),
                            Some("dll" | "dylib" | "so")
                        )
                    })
                    .collect::<Vec<_>>(),
                Err(_) => Vec::new(),
            };
            loaders.sort();

            if helper.is_file() && !loaders.is_empty() {
                if let Ok(output) = Command::new(helper).args(loaders).output()
                    && output.status.success()
                    && !output.stdout.is_empty()
                    && std::fs::create_dir_all(&cache_directory).is_ok()
                {
                    let _ = std::fs::write(&cache_file, output.stdout);
                }
            }
        }
        if cache_file.is_file() {
            set_bundled_environment("GDK_PIXBUF_MODULE_FILE", cache_file.into_os_string());
        } else {
            let packaged_cache = pixbuf_root.join("loaders.cache");
            if packaged_cache.is_file() {
                set_bundled_environment("GDK_PIXBUF_MODULE_FILE", packaged_cache.into_os_string());
            }
        }
    }
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn bundled_runtime_cache_key(path: &Path) -> u64 {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn set_bundled_environment(name: &str, value: std::ffi::OsString) {
    // Startup is single-threaded, before GTK or worker threads are initialized.
    unsafe { std::env::set_var(name, value) };
}

#[cfg(target_os = "windows")]
fn bundled_cache_directory() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Potoo IDE/cache")
}

#[cfg(target_os = "macos")]
fn bundled_cache_directory() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Library/Caches/io.github.diegopn.potooide")
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn configure_bundled_runtime() {}

#[cfg(target_os = "linux")]
fn register_app_fonts() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let Some(directory) = app_font_directory() else {
        return;
    };
    let Ok(directory) = CString::new(directory.as_os_str().as_bytes()) else {
        return;
    };

    unsafe {
        if FcInit() == 0 {
            return;
        }

        let config = std::ptr::null_mut();
        if FcConfigAppFontAddDir(config, directory.as_ptr().cast()) != 0 {
            FcConfigBuildFonts(config);
        }
    }
}

#[cfg(target_os = "windows")]
fn register_app_fonts() {
    use std::os::windows::ffi::OsStrExt;

    let Some(directory) = app_font_directory() else {
        return;
    };
    for font in [
        "InterVariable.ttf",
        "JetBrainsMono-Regular.ttf",
        "JetBrainsMono-Bold.ttf",
    ] {
        let font_path = directory.join(font);
        let mut path = font_path.as_os_str().encode_wide().collect::<Vec<_>>();
        path.push(0);
        unsafe {
            AddFontResourceExW(path.as_ptr(), 0x10, std::ptr::null_mut());
        }
    }
}

#[cfg(target_os = "macos")]
fn register_app_fonts() {
    use std::os::unix::ffi::OsStrExt;

    let Some(directory) = app_font_directory() else {
        return;
    };
    for font in [
        "InterVariable.ttf",
        "JetBrainsMono-Regular.ttf",
        "JetBrainsMono-Bold.ttf",
    ] {
        let path = directory.join(font);
        let bytes = path.as_os_str().as_bytes();
        let url = unsafe {
            CFURLCreateFromFileSystemRepresentation(
                std::ptr::null(),
                bytes.as_ptr(),
                bytes.len() as isize,
                0,
            )
        };
        if url.is_null() {
            continue;
        }

        let mut error = std::ptr::null();
        unsafe {
            CTFontManagerRegisterFontsForURL(url, 1, &mut error);
            CFRelease(url);
            if !error.is_null() {
                CFRelease(error);
            }
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
fn register_app_fonts() {}

fn register_program_font(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let path = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| "o caminho da fonte contém um caractere inválido".to_owned())?;
        let registered = unsafe {
            FcInit() != 0 && FcConfigAppFontAddFile(std::ptr::null_mut(), path.as_ptr().cast()) != 0
        };
        if registered {
            unsafe {
                FcConfigBuildFonts(std::ptr::null_mut());
            }
            Ok(())
        } else {
            Err("não foi possível registrar a fonte".to_owned())
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;

        let mut path = path.as_os_str().encode_wide().collect::<Vec<_>>();
        path.push(0);
        let registered = unsafe { AddFontResourceExW(path.as_ptr(), 0x10, std::ptr::null_mut()) };
        if registered > 0 {
            Ok(())
        } else {
            Err("não foi possível registrar a fonte".to_owned())
        }
    }

    #[cfg(target_os = "macos")]
    {
        use std::os::unix::ffi::OsStrExt;

        let bytes = path.as_os_str().as_bytes();
        let url = unsafe {
            CFURLCreateFromFileSystemRepresentation(
                std::ptr::null(),
                bytes.as_ptr(),
                bytes.len() as isize,
                0,
            )
        };
        if url.is_null() {
            return Err("não foi possível abrir a fonte".to_owned());
        }
        let mut error = std::ptr::null();
        let registered = unsafe {
            let registered = CTFontManagerRegisterFontsForURL(url, 1, &mut error) != 0;
            CFRelease(url);
            if !error.is_null() {
                CFRelease(error);
            }
            registered
        };
        if registered {
            Ok(())
        } else {
            Err("não foi possível registrar a fonte".to_owned())
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        let _ = path;
        Err("carregamento de fontes não é compatível com esta plataforma".to_owned())
    }
}

fn register_app_icons() {
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };

    let icon_theme = gtk::IconTheme::for_display(&display);
    let executable_directory = std::env::current_exe()
        .ok()
        .and_then(|executable| executable.parent().map(Path::to_path_buf));
    for path in app_icon_search_paths(executable_directory.as_deref()) {
        if path.is_dir() {
            icon_theme.add_search_path(&path);
        }
    }
}

fn app_icon_search_paths(executable_directory: Option<&Path>) -> Vec<PathBuf> {
    let mut paths = vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/icons")];
    if let Some(directory) = executable_directory {
        paths.extend([
            directory.join("data/icons"),
            directory.join("icons"),
            directory.join("share/icons"),
            directory.join("../share/icons"),
            directory.join("../Resources/icons"),
        ]);
    }
    paths
}

#[cfg(unix)]
pub(crate) fn restart_application() -> Result<(), String> {
    use std::os::unix::process::CommandExt;

    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let error = Command::new(executable)
        .args(std::env::args_os().skip(1))
        .exec();
    Err(error.to_string())
}

#[cfg(windows)]
pub(crate) fn restart_application() -> Result<(), String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    Command::new(executable)
        .args(std::env::args_os().skip(1))
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[cfg(not(any(unix, windows)))]
pub(crate) fn restart_application() -> Result<(), String> {
    Err("reinicialização não suportada nesta plataforma".to_string())
}

fn main() {
    configure_bundled_runtime();
    register_app_fonts();

    let application = gtk::Application::builder()
        .application_id("io.github.diegopn.potooide")
        .build();

    application.connect_startup(|_| {
        sourceview5::init();
        register_app_icons();
        let provider = gtk::CssProvider::new();
        provider.load_from_string(include_str!("../data/potoo.css"));
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    });

    RelmApp::from_app(application).run::<app::MainWindow>(());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_icon_search_paths_point_to_icon_theme_roots() {
        let paths = app_icon_search_paths(None);
        for icon in [
            "potoo-window-minimize-symbolic.svg",
            "potoo-window-maximize-symbolic.svg",
            "potoo-window-restore-symbolic.svg",
            "potoo-close-symbolic.svg",
        ] {
            assert!(paths.iter().any(|path| {
                path.join(format!("hicolor/scalable/actions/{icon}"))
                    .is_file()
            }));
        }
    }
}
