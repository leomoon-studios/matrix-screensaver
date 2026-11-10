# Matrix Screensaver

A native Rust Matrix animation with modern 3D and classic 2D rain. It renders rain, bloom, and color entirely on the GPU. It runs in a resizable window by default or opens full-screen windows with `--screensaver`.

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

The default `--type modern` renders the 3D effect internally at 50% of the window's physical size and targets 30 FPS. This setting added about 3–4 W of GPU board power over idle in the single-monitor benchmark. Run `--type classic` for flat Matrix rain with a black, staggered startup. For a sharper image and smoother animation, use `./target/release/matrix-screensaver --type classic --fps 60 --render-scale 0.75`. The same quality settings work with modern:

```bash
./target/release/matrix-screensaver --fps 60 --render-scale 0.75
```

Build first with `cargo build --release`. Select the effect with `--type modern|classic`; override either quality setting with `--render-scale 0.25..1` and `--fps 1..60`; for example, `./target/release/matrix-screensaver --fps 60 --render-scale 1` uses full internal resolution. Run `./target/release/matrix-screensaver --help` for usage. The `MATRIX_RENDER_SCALE` and `MATRIX_FPS` environment variables remain supported for existing scripts; command-line values take precedence. Integrated graphics are supported when the driver provides a compatible adapter. The app exits with a clear error on a CPU software adapter because that path has not met the screensaver's low-power target.

To try the full-screen mode, run `./target/release/matrix-screensaver --screensaver --type modern` (or `--type classic`). It creates one window per active monitor and exits on a key press, mouse click, scroll, touch, or pointer movement past a small threshold. A session idle launcher should also stop the process on resume or lock; installation of that launcher is planned in Task 5. The binary does not lock the session. Temporary-output add and removal passed on the tested Hyprland session; physical unplug and suspend still need validation. See [Step 4 validation](docs/validation.md#step-4-linux-screensaver-mode).

For an Omarchy launcher, `--app-id org.omarchy.screensaver` gives the Wayland windows the class that its idle service tracks. This sets the window class only; the Task 5 adapter must still launch the binary and stop it on resume or lock.

For a 60-second NVIDIA benchmark, build the release binary and run `python scripts/benchmark.py target/release/matrix-screensaver --type classic` (or `--type modern`). It samples idle and running CPU and GPU counters and process memory, and enables 10-second FPS and frame-interval reports. On Hyprland, add `--float --size 1280x720` to measure that physical window size; otherwise tiling can override the requested size. See [validation results](docs/validation.md).

For two connected Hyprland monitors, run `python scripts/benchmark_dual.py target/release/matrix-screensaver --seconds 60`. It starts one instance per monitor and measures both simultaneously. The current default is the measured 30 FPS preset.

For sampled GPU pass timings on an adapter with timestamp-query support, run `MATRIX_PROFILE_GPU=1 target/release/matrix-screensaver`. It reports simulation, rain, bloom, palette, and presentation times every 300 frames.

## Checks

```bash
cargo fmt --check
cargo check
cargo test
```

## Attribution

The bundled glyph atlas and adapted shader design come from [Rezmason/matrix](https://github.com/Rezmason/matrix). See [third-party notices](docs/third-party-notices.md) before redistributing them.
