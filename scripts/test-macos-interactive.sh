#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/.." && pwd)
bundle=${1:-"$repo_root/dist/macos/Matrix Screensaver.saver"}
preference_domain=com.leomoon-studios.matrix-screensaver

"$repo_root/scripts/check-macos-bundle.sh" "$bundle"
host=$(mktemp -t matrix-saver-preview.XXXXXX)
trap 'rm -f "$host"' EXIT
xcrun clang -fobjc-arc -fmodules -Wall -Wextra -framework AppKit -framework ScreenSaver \
    "$repo_root/macos/preview_host.m" -o "$host"

for rain_type in modern classic; do
    defaults write "$preference_domain" type "$rain_type"
    defaults write "$preference_domain" color green
    echo "Testing $rain_type preview. Confirm that rain appears, fills the window, and remains sharp after the square resize."
    "$host" "$bundle"
done
defaults write "$preference_domain" type modern
echo 'Automated preview lifecycle checks passed. Complete docs/macos-testing.md in System Settings.'
