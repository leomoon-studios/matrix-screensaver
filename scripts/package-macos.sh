#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/.." && pwd)
if [[ $(uname -s) != Darwin ]]; then
    echo 'package-macos.sh must run on macOS.' >&2
    exit 1
fi

version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$repo_root/Cargo.toml" | head -1)
output=${1:-$repo_root/dist/macos}
architectures=${MATRIX_MACOS_ARCHS:-"arm64 x86_64"}
rust_toolchain=${MATRIX_RUST_TOOLCHAIN:-}
toolchain_args=()
if [[ -n "$rust_toolchain" ]]; then
    toolchain_args+=("$rust_toolchain")
fi
build_root=$repo_root/target/macos-saver
bundle="$output/Matrix Screensaver.saver"
executable_name=MatrixScreenSaver

rm -rf "$build_root" "$bundle"
mkdir -p "$build_root" "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
cp "$repo_root/macos/Info.plist" "$bundle/Contents/Info.plist"
cp "$repo_root/LICENSE" "$bundle/Contents/Resources/LICENSE"
cp "$repo_root/docs/third-party-notices.md" "$bundle/Contents/Resources/THIRD-PARTY-NOTICES.md"
cp "$repo_root/docs/macos-install.md" "$bundle/Contents/Resources/INSTALL.md"

iconset="$build_root/MatrixScreensaver.iconset"
mkdir -p "$iconset"
for spec in '16 icon_16x16.png' '32 icon_16x16@2x.png' '32 icon_32x32.png' '64 icon_32x32@2x.png' '128 icon_128x128.png' '256 icon_128x128@2x.png' '256 icon_256x256.png' '512 icon_256x256@2x.png' '512 icon_512x512.png' '1024 icon_512x512@2x.png'; do
    read -r size name <<< "$spec"
    sips -z "$size" "$size" "$repo_root/assets/matrix-screensaver.png" --out "$iconset/$name" >/dev/null
done
iconutil -c icns "$iconset" -o "$bundle/Contents/Resources/MatrixScreensaver.icns"

frameworks=(
    -framework AppKit
    -framework Carbon
    -framework CoreFoundation
    -framework CoreGraphics
    -framework CoreServices
    -framework CoreVideo
    -framework Foundation
    -framework IOKit
    -framework IOSurface
    -framework Metal
    -framework QuartzCore
    -framework ScreenSaver
    -framework Security
    -framework SystemConfiguration
)

binaries=()
for architecture in $architectures; do
    case "$architecture" in
        arm64) target=aarch64-apple-darwin ;;
        x86_64) target=x86_64-apple-darwin ;;
        *) echo "Unsupported macOS architecture: $architecture" >&2; exit 1 ;;
    esac
    if [[ -n "$rust_toolchain" ]]; then
        rustup target add --toolchain "${rust_toolchain#+}" "$target"
    else
        rustup target add "$target"
    fi
    cargo "${toolchain_args[@]}" build --manifest-path "$repo_root/Cargo.toml" --release --locked --lib --target "$target"
    object="$build_root/MatrixScreenSaver-$architecture.o"
    binary="$build_root/$executable_name-$architecture"
    xcrun clang -fobjc-arc -fmodules -Wall -Wextra -arch "$architecture" -mmacosx-version-min=12.0 \
        -c "$repo_root/macos/MatrixScreenSaverView.m" -o "$object"
    xcrun clang -bundle -arch "$architecture" -mmacosx-version-min=12.0 "$object" \
        "$repo_root/target/$target/release/libmatrix_screensaver.a" "${frameworks[@]}" -lc++ -liconv -lobjc \
        -Wl,-dead_strip -o "$binary"
    binaries+=("$binary")
done

if (( ${#binaries[@]} == 1 )); then
    cp "${binaries[0]}" "$bundle/Contents/MacOS/$executable_name"
else
    xcrun lipo -create "${binaries[@]}" -output "$bundle/Contents/MacOS/$executable_name"
fi
xcrun strip -x "$bundle/Contents/MacOS/$executable_name"
codesign --force --deep --sign - "$bundle"

archive="$output/matrix-screensaver-$version-macos.zip"
rm -f "$archive" "$archive.sha256"
ditto -c -k --sequesterRsrc --keepParent "$bundle" "$archive"
(
    cd "$output"
    shasum -a 256 "$(basename "$archive")" > "$(basename "$archive").sha256"
)
echo "Created $bundle"
echo "Created $archive"
