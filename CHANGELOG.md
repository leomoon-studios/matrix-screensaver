# Changelog

## 1.0.0 (2026-11-10)

- Initial release of the native Rust Matrix screensaver.
- Modern 3D Matrix rain with perspective depth, forward motion, bright cursor heads, fading trails, glints, palette mapping, and multi-level bloom.
- Classic 2D Matrix rain with a black staggered intro, flat falling columns, bright heads, glyph cycling, fading trails, and tuned bloom.
- Green, blue, red, yellow, purple, and orange color themes for both rain types.
- Configurable frame rate from 1 to 60 FPS and internal render scale from 0.25 to 1.0.
- Low-power defaults of 30 FPS and 50% render scale, with full-resolution and 60 FPS modes available.
- Native GPU rendering through Vulkan on Linux, Direct3D 12 on Windows, and Metal on macOS.
- Embedded MSDF Matrix glyph atlas and shaders with no browser, web view, runtime network access, or working-directory asset dependency.
- Resizable standalone application for development and manual use.
- Linux full-screen screensaver mode with one borderless surface per active monitor, live monitor changes, display scaling, cursor hiding, and input dismissal.
- Linux Wayland and X11 support with documented Omarchy, hypridle, swayidle, and standalone X11 integration.
- Windows `.scr` host with full-screen `/s`, embedded `/p <HWND>` preview, and native `/c` settings modes.
- Windows settings for rain type, color, FPS, and render scale, saved per user and shared by full-screen and preview modes.
- Windows multi-monitor full-screen rendering, startup input filtering, pointer movement threshold, keyboard and mouse dismissal, and first-frame presentation before showing the saver.
- Windows package with an embedded Matrix glyph icon, reusable PNG icon, attribution, installation instructions, and a separate console development executable.
- Native macOS `.saver` bundle with a `ScreenSaverView` host, Rust FFI renderer lifecycle, preview support, Retina scaling, and system-managed dismissal and authentication.
- Universal macOS package for Apple Silicon and Intel with installation and manual testing instructions.
- Automatic pause and clean resume while rendering is occluded or the display is unavailable, without animation-time jumps.
- Surface-loss recovery, resize handling, native adapter selection, and a clear error when only an unsuitable CPU software adapter is available.
- GPU pass profiling for simulation, rain, bloom, palette, and presentation.
- Repeatable CPU, memory, GPU, power, frame-rate, frame-time, resize, and multi-monitor benchmark tools and validation records.
- Portable Linux archive with checksums, license, third-party notices, installation documentation, and embedded runtime assets.
- Cross-platform GitHub Actions checks on Ubuntu 24.04, Windows Server 2025, macOS 15 Apple Silicon, and macOS 15 Intel.
- Automated formatting, Clippy, release-build, pure-test, command-line, embedded-asset, package-layout, Windows host, Linux archive, and macOS bundle checks.
- Separate ignored GPU rendering and manual capture tests for execution on native interactive graphics hardware.
- MIT licensing and attribution for the glyph atlas and renderer design adapted from Rezmason/matrix.
