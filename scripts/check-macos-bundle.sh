#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/.." && pwd)
bundle=${1:-"$repo_root/dist/macos/Matrix Screensaver.saver"}
executable="$bundle/Contents/MacOS/MatrixScreenSaver"
expected_arches=${MATRIX_EXPECT_MACOS_ARCHS:-"arm64 x86_64"}

test -d "$bundle"
test -x "$executable"
test -s "$bundle/Contents/Resources/MatrixScreensaver.icns"
test -s "$bundle/Contents/Resources/LICENSE"
test -s "$bundle/Contents/Resources/THIRD-PARTY-NOTICES.md"
test -s "$bundle/Contents/Resources/INSTALL.md"
plutil -lint "$bundle/Contents/Info.plist"
test "$(/usr/libexec/PlistBuddy -c 'Print :CFBundlePackageType' "$bundle/Contents/Info.plist")" = BNDL
test "$(/usr/libexec/PlistBuddy -c 'Print :NSPrincipalClass' "$bundle/Contents/Info.plist")" = MatrixScreenSaverView
codesign --verify --deep --strict "$bundle"

architectures=$(xcrun lipo -archs "$executable")
for architecture in $expected_arches; do
    [[ " $architectures " == *" $architecture "* ]] || { echo "Missing architecture $architecture in $architectures" >&2; exit 1; }
done

checker=$(mktemp -t matrix-saver-check.XXXXXX)
trap 'rm -f "$checker"' EXIT
xcrun clang -fobjc-arc -fmodules -Wall -Wextra -framework AppKit -framework ScreenSaver \
    "$repo_root/macos/check_bundle.m" -o "$checker"
"$checker" "$bundle"
echo "Bundle checks passed for: $architectures"
