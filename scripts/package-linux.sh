#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
binary=${1:-$repo_root/target/release/matrix-screensaver}
destination=${2:-$repo_root/dist}
version=$(sed -n '/^\[package\]/,/^\[/s/^version = "\([^"]*\)"/\1/p' "$repo_root/Cargo.toml" | head -n 1)
architecture=$(uname -m)
name="matrix-screensaver-${version}-linux-${architecture}"

[[ -f $binary && -x $binary ]] || { echo "Executable not found: $binary" >&2; exit 1; }
[[ -n $version ]] || { echo 'Cannot read package version' >&2; exit 1; }
mkdir -p -- "$destination"
staging=$(mktemp -d)
trap 'rm -rf -- "$staging"' EXIT
mkdir -p -- "$staging/$name"
install -m 755 -- "$binary" "$staging/$name/matrix-screensaver"
install -m 644 -- "$repo_root/LICENSE" "$staging/$name/LICENSE"
install -m 644 -- "$repo_root/docs/third-party-notices.md" "$staging/$name/THIRD-PARTY-NOTICES.md"
install -m 644 -- "$repo_root/docs/linux-install.md" "$staging/$name/INSTALL.md"

tar -C "$staging" -czf "$destination/$name.tar.gz" "$name"
(cd "$destination" && sha256sum "$name.tar.gz" > "$name.tar.gz.sha256")
printf '%s\n' "$destination/$name.tar.gz"
