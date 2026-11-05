#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$repo_root/Cargo.toml" | head -n1)"
arch="x86_64"
package_root="$repo_root/target/packages/macos-${arch}"
app_bundle="$package_root/stage/Potoo IDE.app"
dmg_root="$package_root/dmg"
dmg_path="$package_root/Potoo-IDE-${version}-macos-${arch}.dmg"

"$repo_root/packaging/macos/stage-runtime.sh"

if [[ ! -d "$app_bundle" ]]; then
    echo "Application bundle not found: $app_bundle" >&2
    exit 1
fi

rm -rf "$dmg_root"
mkdir -p "$dmg_root"
cp -R "$app_bundle" "$dmg_root/"
cp "$repo_root/LICENSE" "$dmg_root/LICENSE.txt"
cat > "$dmg_root/Install Potoo IDE.txt" <<'EOF'
Drag Potoo IDE.app to the Applications folder.

This build targets Intel Macs. It is not signed or notarized. macOS may ask you
to confirm the first launch in System Settings > Privacy & Security.
EOF
ln -s /Applications "$dmg_root/Applications"
rm -f "$dmg_path"

hdiutil create \
    -volname "Potoo IDE" \
    -srcfolder "$dmg_root" \
    -format UDZO \
    "$dmg_path"

echo "Built macOS installer: $dmg_path"
