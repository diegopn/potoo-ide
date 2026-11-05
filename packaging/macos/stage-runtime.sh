#!/usr/bin/env bash

set -euo pipefail
shopt -s nullglob

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
prefix="${GTK_PREFIX:-$(brew --prefix)}"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$repo_root/Cargo.toml" | head -n1)"
arch="x86_64"
target="x86_64-apple-darwin"
package_root="$repo_root/target/packages/macos-${arch}"
stage_root="$package_root/stage"
app_bundle="$stage_root/Potoo IDE.app"
contents_dir="$app_bundle/Contents"
macos_dir="$contents_dir/MacOS"
frameworks_dir="$contents_dir/Frameworks"
resources_dir="$contents_dir/Resources"
resource_bin_dir="$macos_dir"
bundle_binary="$macos_dir/PotooIDE"
release_binary="$repo_root/target/x86_64-apple-darwin/release/potoo-ide"
deployment_target="${MACOSX_DEPLOYMENT_TARGET:-13.0}"

if [[ "$(uname -m)" != "x86_64" ]]; then
    echo "The macOS Intel bundle must be built on an x86_64 runner with matching GTK libraries." >&2
    exit 1
fi

copy_tree() {
    local source="$1"
    local destination="$2"
    if [[ ! -e "$source" ]]; then
        echo "Required GTK runtime path not found: $source" >&2
        exit 1
    fi
    mkdir -p "$destination"
    cp -RL "$source"/. "$destination"/
}

copy_tree_if_present() {
    local source="$1"
    local destination="$2"
    if [[ -e "$source" ]]; then
        copy_tree "$source" "$destination"
    fi
}

is_system_dependency() {
    [[ "$1" == /System/Library/* || "$1" == /usr/lib/* || "$1" == /Library/Apple/System/* ]]
}

resolve_dependency() {
    local source="$1"
    local dependency="$2"
    local source_dir="$(dirname "$source")"
    local candidate

    case "$dependency" in
        @loader_path/*)
            candidate="$source_dir/${dependency#@loader_path/}"
            [[ -e "$candidate" ]] && { printf '%s\n' "$candidate"; return 0; }
            ;;
        @executable_path/*)
            candidate="$macos_dir/${dependency#@executable_path/}"
            [[ -e "$candidate" ]] && { printf '%s\n' "$candidate"; return 0; }
            ;;
        @rpath/*)
            candidate="$prefix/Library/Frameworks/${dependency#@rpath/}"
            [[ -e "$candidate" ]] && { printf '%s\n' "$candidate"; return 0; }
            ;;
        /*)
            [[ -e "$dependency" ]] && { printf '%s\n' "$dependency"; return 0; }
            ;;
    esac

    local dependency_name="$(basename "$dependency")"
    for candidate in "$prefix/lib/$dependency_name" "$prefix/opt/"*/lib/"$dependency_name"; do
        [[ -e "$candidate" ]] && { printf '%s\n' "$candidate"; return 0; }
    done
    if [[ -d "$prefix/Cellar" ]]; then
        candidate="$(find "$prefix/Cellar" -name "$dependency_name" -print -quit 2>/dev/null || true)"
        [[ -n "$candidate" ]] && { printf '%s\n' "$candidate"; return 0; }
    fi
    return 1
}

bundle_dependency_closure() {
    local queue=("$@")
    local -A seen=()

    while ((${#queue[@]})); do
        local current="${queue[0]}"
        queue=("${queue[@]:1}")
        local real_current="$(python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$current")"
        [[ -n "${seen[$real_current]:-}" ]] && continue
        seen["$real_current"]=1

        chmod u+w "$current" 2>/dev/null || true
        if [[ "$current" == *.dylib ]]; then
            install_name_tool -id "@executable_path/../Frameworks/$(basename "$current")" "$current"
        fi

        while IFS= read -r dependency; do
            [[ -z "$dependency" ]] && continue
            is_system_dependency "$dependency" && continue
            [[ "$dependency" == @executable_path/../Frameworks/* ]] && continue

            local resolved
            if ! resolved="$(resolve_dependency "$current" "$dependency")"; then
                echo "Unable to resolve macOS runtime dependency '$dependency' used by '$current'." >&2
                exit 1
            fi

            local name="$(basename "$resolved")"
            local destination="$frameworks_dir/$name"
            if [[ ! -e "$destination" ]]; then
                cp -L "$resolved" "$destination"
                chmod u+w "$destination" 2>/dev/null || true
                queue+=("$destination")
            fi
            install_name_tool -change "$dependency" "@executable_path/../Frameworks/$name" "$current"
        done < <(otool -L "$current" | tail -n +2 | awk '{print $1}')
    done
}

create_info_plist() {
    cat > "$contents_dir/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key><string>en</string>
    <key>CFBundleDisplayName</key><string>Potoo IDE</string>
    <key>CFBundleExecutable</key><string>PotooIDE</string>
    <key>CFBundleIconFile</key><string>io.github.diegopn.potooide.icns</string>
    <key>CFBundleIdentifier</key><string>io.github.diegopn.potooide</string>
    <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
    <key>CFBundleName</key><string>Potoo IDE</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$version</string>
    <key>CFBundleVersion</key><string>$version</string>
    <key>LSMinimumSystemVersion</key><string>$deployment_target</string>
    <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF
}

rustflags="${RUSTFLAGS:-} -C link-arg=-Wl,-headerpad_max_install_names"
MACOSX_DEPLOYMENT_TARGET="$deployment_target" RUSTFLAGS="$rustflags" \
    cargo build --locked --release --bin potoo-ide --target "$target"

if [[ ! -x "$release_binary" ]]; then
    echo "Release executable not found after the macOS build: $release_binary" >&2
    exit 1
fi

rm -rf "$package_root"
mkdir -p "$macos_dir" "$frameworks_dir" "$resources_dir"
cp "$release_binary" "$bundle_binary"
chmod 0755 "$bundle_binary"
if [[ "$(lipo -archs "$bundle_binary")" != "x86_64" ]]; then
    echo "The macOS release executable is not an Intel x86_64 binary." >&2
    exit 1
fi
create_info_plist

cp "$repo_root/data/icons/macos/io.github.diegopn.potooide.icns" "$resources_dir/"
copy_tree "$repo_root/data/fonts" "$resources_dir/fonts"
copy_tree "$repo_root/data/icons" "$resources_dir/icons"
copy_tree "$prefix/share/glib-2.0" "$resources_dir/share/glib-2.0"
copy_tree "$prefix/share/gtk-4.0" "$resources_dir/share/gtk-4.0"
copy_tree "$prefix/share/gtksourceview-5" "$resources_dir/share/gtksourceview-5"
copy_tree_if_present "$prefix/share/icons/Adwaita" "$resources_dir/share/icons/Adwaita"
copy_tree_if_present "$prefix/share/icons/hicolor" "$resources_dir/share/icons/hicolor"
copy_tree_if_present "$prefix/share/themes/Default" "$resources_dir/share/themes/Default"
copy_tree "$prefix/lib/gdk-pixbuf-2.0" "$resources_dir/lib/gdk-pixbuf-2.0"
copy_tree_if_present "$prefix/lib/gio/modules" "$resources_dir/lib/gio/modules"
copy_tree_if_present "$prefix/lib/gtk-4.0" "$resources_dir/lib/gtk-4.0"
copy_tree_if_present "$prefix/lib/gstreamer-1.0" "$resources_dir/lib/gstreamer-1.0"
copy_tree_if_present "$prefix/share/gstreamer-1.0" "$resources_dir/share/gstreamer-1.0"
copy_tree_if_present "$prefix/share/licenses" "$resources_dir/share/licenses"

pixbuf_helper="$prefix/bin/gdk-pixbuf-query-loaders"
if [[ ! -x "$pixbuf_helper" ]]; then
    echo "Required GdkPixbuf loader helper not found: $pixbuf_helper" >&2
    exit 1
fi
cp -L "$pixbuf_helper" "$resource_bin_dir/gdk-pixbuf-query-loaders"
chmod 0755 "$resource_bin_dir/gdk-pixbuf-query-loaders"
gst_scanner="$prefix/libexec/gstreamer-1.0/gst-plugin-scanner"
if [[ ! -x "$gst_scanner" ]]; then
    gst_scanner="$prefix/lib/gstreamer-1.0/gst-plugin-scanner"
fi
if [[ ! -x "$gst_scanner" ]]; then
    echo "Required GStreamer plugin scanner not found." >&2
    exit 1
fi
cp -L "$gst_scanner" "$resource_bin_dir/gst-plugin-scanner"
chmod 0755 "$resource_bin_dir/gst-plugin-scanner"

bundle_targets=("$bundle_binary")
[[ -x "$resource_bin_dir/gdk-pixbuf-query-loaders" ]] && bundle_targets+=("$resource_bin_dir/gdk-pixbuf-query-loaders")
[[ -x "$resource_bin_dir/gst-plugin-scanner" ]] && bundle_targets+=("$resource_bin_dir/gst-plugin-scanner")
while IFS= read -r file; do bundle_targets+=("$file"); done < <(
    find "$resources_dir/lib" -type f \( -name '*.dylib' -o -name '*.so' \) 2>/dev/null | sort
)
bundle_dependency_closure "${bundle_targets[@]}"

echo "Staged macOS $arch runtime: $app_bundle"
