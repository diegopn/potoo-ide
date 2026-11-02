mod app;
mod components;

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
    register_app_fonts();

    let application = gtk::Application::builder()
        .application_id("io.github.diegopn.potooide")
        .build();

    application.connect_startup(|_| {
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
