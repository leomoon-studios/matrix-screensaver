# Matrix Screensaver

A native Rust Matrix 3D animation for Omarchy. It opens a resizable GPU window and renders the copied reference rain, bloom, and palette pipeline. Screensaver and idle integration are still planned.

## Build and run

On Arch/Omarchy, install a Rust toolchain and native build prerequisites:

```bash
sudo pacman -S rustup base-devel
rustup default stable
cargo run
```

The project enables Wayland and X11 backends in `winit`; on Omarchy it will use the Wayland session. It requests a native `wgpu` adapter and reports a readable failure if no compatible GPU surface or device is available.

The glyph atlas and WGSL shaders are compiled into the executable. Running the binary does not fetch assets or require a browser, network connection, or working-directory-relative asset path. The active settings and GPU resource formats are documented in [renderer details](docs/renderer.md).

For visual regression checks, choose a starting window size without changing the renderer:

```bash
MATRIX_WINDOW_SIZE=1280x720 cargo run
MATRIX_WINDOW_SIZE=720x1280 cargo run
MATRIX_WINDOW_SIZE=1024x1024 cargo run
```

The default renders internally at 75% of the window's physical size, matching the hosted reference's resolution setting, and targets 60 FPS. For lower power use:

```bash
MATRIX_LOW_POWER=1 cargo run
```

That preset renders at 50% scale and targets 30 FPS. Override either setting with `MATRIX_RENDER_SCALE=0.25..1` and `MATRIX_FPS=1..60`; for example, `MATRIX_RENDER_SCALE=1 MATRIX_FPS=60 cargo run` restores full internal resolution. Integrated graphics are supported when the driver provides a compatible adapter. The app exits with a clear error on a CPU software adapter because that path has not met the screensaver's low-power target.

For a 60-second NVIDIA benchmark, build the release binary and run `python scripts/benchmark.py target/release/matrix-screensaver`. It samples idle and running CPU and GPU counters and process memory, and enables 10-second FPS and frame-interval reports. On Hyprland, add `--float --size 1280x720` to measure that physical window size; otherwise tiling can override the requested size. See [validation results](docs/validation.md).

For two connected Hyprland monitors, run `python scripts/benchmark_dual.py target/release/matrix-screensaver --seconds 60`. It starts one instance per monitor and measures both simultaneously. Add `--low-power` to check the 30 FPS preset.

## Checks

```bash
cargo fmt --check
cargo check
cargo test
```

## Attribution

The bundled glyph atlas originates in the copied `Rezmason/matrix` reference. See [third-party notices](docs/third-party-notices.md) before redistributing it.
