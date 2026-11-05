#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
manifest="$repo_root/io.github.diegopn.potooide.json"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$repo_root/Cargo.toml" | head -n1)"
arch="${FLATPAK_ARCH:-x86_64}"
package_root="$repo_root/target/packages/linux-${arch}"
build_root="$package_root/build"
repo="$package_root/repo"
bundle="$package_root/Potoo-IDE-${version}-linux-${arch}.flatpak"

flatpak-builder \
    --user \
    --force-clean \
    --disable-rofiles-fuse \
    --default-branch=stable \
    --repo="$repo" \
    "$build_root" \
    "$manifest"

flatpak build-bundle \
    --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo \
    "$repo" \
    "$bundle" \
    io.github.diegopn.potooide \
    stable

if [[ ! -s "$bundle" ]]; then
    echo "Flatpak bundle was not created: $bundle" >&2
    exit 1
fi

echo "Built Linux installer: $bundle"
