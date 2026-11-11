#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/.." && pwd)
bundle=${1:-"$repo_root/dist/macos/Matrix Screensaver.saver"}
checker=$(mktemp -t matrix-saver-options.XXXXXX)
trap 'rm -f "$checker"' EXIT

xcrun clang -fobjc-arc -fmodules -Wall -Wextra -framework AppKit -framework ScreenSaver \
    "$repo_root/macos/check_options.m" -o "$checker"
"$checker" "$bundle"
