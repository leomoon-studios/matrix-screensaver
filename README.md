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

## Checks

```bash
cargo fmt --check
cargo check
cargo test
```

## Attribution

The bundled glyph atlas originates in the copied `Rezmason/matrix` reference. See [third-party notices](docs/third-party-notices.md) before redistributing it.
