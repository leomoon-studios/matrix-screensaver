# Matrix Screensaver

A native Rust foundation for an Omarchy Matrix 3D screensaver. It currently opens a resizable native GPU window and clears it to black. The volumetric rain renderer is planned for Task 3.

## Build and run

On Arch/Omarchy, install a Rust toolchain and native build prerequisites:

```bash
sudo pacman -S rustup base-devel
rustup default stable
cargo run
```

The project enables Wayland and X11 backends in `winit`; on Omarchy it will use the Wayland session. It requests a native `wgpu` adapter and reports a readable failure if no compatible GPU surface or device is available.

`reference/matrix/assets/matrixcode_msdf.png` is compiled into the executable. Running the binary therefore does not fetch assets or require a browser, network connection, or working-directory-relative asset path.

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
```

## Attribution

The bundled glyph atlas originates in the copied `Rezmason/matrix` reference. See [third-party notices](docs/third-party-notices.md) before redistributing it.
