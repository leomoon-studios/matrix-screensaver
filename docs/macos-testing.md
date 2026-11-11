# macOS screensaver tests

Run these checks on a Mac from the repository root after downloading or building the universal bundle. They cover the native `ScreenSaverView`, Metal rendering, Retina resize, lifecycle cleanup, installation, multiple displays, and system-controlled dismissal.

## Build and structural checks

Install the pinned Rust toolchain if needed, then build and check the bundle:

```bash
rustup toolchain install 1.99.1 --profile minimal --component rustfmt,clippy
scripts/package-macos.sh
scripts/check-macos-bundle.sh
```

The final line must report both `arm64` and `x86_64`. The checker verifies the property list, principal class, resources, ad hoc signature, architectures, and that macOS can load the bundle as a `ScreenSaverView` plugin.

## Interactive preview lifecycle

Run:

```bash
scripts/test-macos-interactive.sh
```

Two seven-second preview windows open, first modern and then classic. For each, confirm Matrix rain appears without a separate application window, fills the preview, animates smoothly, and remains correctly sized when the host changes from 960 by 540 points to 720 by 720 points. The script calls `startAnimation`, renders, resizes, calls `stopAnimation`, closes the host, and restores modern as the selected type.

## System Screensaver tests

1. Open `dist/macos/Matrix Screensaver.saver`, install it for the current user, and select it in **System Settings > Screen Saver**.
2. Open the System Settings preview. Confirm rain appears inside the system preview and that no unrelated top-level window opens.
3. Start the full-screen preview. Confirm the animation fills every connected display with correct aspect ratio and sharp Retina output.
4. Move the pointer and press a key in separate runs. Confirm macOS closes the preview immediately. The plugin itself must not capture global input or perform authentication.
5. Connect or disconnect an external display, reopen the full-screen preview, and confirm each display gets an independently sized view without stretching or a black display.
6. Change `type` and `color` with the commands in `docs/macos-install.md`, reopen the preview after each change, and confirm modern, classic, and all six colors render correctly.
7. Repeat start and dismissal ten times. In Activity Monitor, confirm the screensaver process disappears after each dismissal and does not leave increasing GPU or memory use.
8. Put the Mac to sleep while the screensaver is active, wake it, and confirm macOS owns the lock and authentication flow. Confirm the next preview still renders.

Report the Mac model, macOS version, display resolutions and scaling, bundle SHA-256, and any failed item. Screenshots of modern and classic in both the small preview and full-screen mode are useful for visual comparison.
