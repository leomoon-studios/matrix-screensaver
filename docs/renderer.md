# Renderer

The app embeds its WGSL shaders from `src/shaders/` and its 512 × 512 MSDF glyph atlas from `assets/matrixcode_msdf.png`. It reads no files or network resources at runtime. Each frame updates an 80 × 80 rain state, renders glyph brightness and a glow mask, blurs the glow mask, applies a 512-color palette, and presents the result.

## Modes

| Setting | Modern (default) | Classic |
| --- | --- | --- |
| Geometry | 6,400 quads, 38,400 vertices, 90° perspective | One full-screen quad, six vertices, no perspective |
| Fall speed / raindrop length | 0.5 / 0.3 | 0.3 / 0.75 |
| Base brightness / contrast | −0.9 / 1.5 | −0.5 / 1.1 |
| Intro | Starts with active rain | Starts black; columns reveal from top in a staggered sequence |
| Bloom strength | 0.7 | 0.45 |

Both modes use the same 57-symbol glyph atlas, 80 × 80 cell state, selected palette, four bloom levels, and bright isolated cursor color. Classic's lower bloom strength is an intentional visual adjustment. The default power setting is 30 FPS at 50% internal resolution. `--fps` and `--render-scale` adjust those values without changing the mode.

`--color` rotates the 512-color palette and bright cursor hue together. Available values are `green` (default), `blue`, `red`, `yellow`, `purple`, and `orange`; geometry, timing, and bloom strength are unchanged.

The palette's per-pixel dither strength follows `--render-scale`: it is 0.05 at scale 1, 0.025 at the default scale 0.5, and 0.0125 at scale 0.25. Lower internal resolutions therefore show gentler speckles instead of enlarging equally strong noise. This changes only a uniform set when the renderer starts; the shader performs the same work per pixel.

## GPU resources

| Resource | Size and format |
| --- | --- |
| Config, Time, Scene uniforms | 192, 16, and 144 bytes |
| Cell and intro storage | 6,400 × 48 = 307,200 bytes; 80 × 16 = 1,280 bytes |
| Glyph atlas | Embedded RGBA8 unorm texture, sampled linearly |
| Rain brightness and glow mask | Two full-size RGBA8 unorm render targets |
| Bloom pyramid | Four horizontal and four vertical RGBA8 unorm storage textures; first level is 40% of internal size |
| Bloom combine | RGBA8 unorm storage texture at the first bloom level's size |
| Palette lookup | 512 colors in an 8,192-byte uniform buffer |
| Palette output | Full-size RGBA8 unorm storage texture |
| Window surface | Available sRGB format and FIFO presentation preferred |

Resizing recreates targets and updates the scene while preserving animation state. The app rejects a software CPU adapter to avoid expensive rendering on that path.

## Host boundary

`src/lib.rs` contains the GPU simulation, embedded assets, shaders, mode settings, and presentation code. `src/main.rs` owns the standalone window, monitor tracking, input dismissal, and idle-session behavior. A native host can construct a `wgpu::Surface` for its window or view and pass it, its physical pixel size, and a `wgpu::Instance` to `Renderer::new_for_surface`. The host must keep the window or view alive until the renderer is dropped. For multiple windows, pass the first renderer as `shared` and create each surface from the same instance; this shares one GPU device.

The host calls `resize` when its backing pixel size changes, `render` on a frame tick, and `pause`/`resume` when animation is suspended. `RenderStatus::SurfaceLost` asks the host to create another surface and call `replace_surface`. `stop` consumes the renderer and releases its resources. The standalone `winit` host uses `new_for_window` and `recreate_surface_for_window` as conveniences. Windows and macOS screensaver hosts are separate later steps.

The standalone host selects Vulkan or GL on Linux, Direct3D 12 on Windows, and Metal on macOS. A supplied native surface must be compatible with its instance and adapter. The macOS ScreenSaver bundle gives the renderer a `CAMetalLayer` through a small C ABI. Each system-created `ScreenSaverView` owns one renderer, supplies Retina pixel dimensions on resize, pauses it with `stopAnimation`, and destroys it before the view is released.

## Automated checks

`.github/workflows/ci.yml` builds and runs non-GPU tests on Ubuntu 24.04, Windows Server 2025, macOS 15 Apple Silicon, and macOS 15 Intel. It compiles the GPU tests but leaves them ignored on these standard hosted runners. The test log and test list make the distinction visible. A green standard-runner job proves compilation, pure logic, CLI validation, and embedded assets; it does not prove a GPU render or screensaver integration.

On a verified native GPU, run the render tests explicitly:

```bash
cargo test --locked --lib -- --ignored --skip capture_native_frame
```

The capture test needs the environment variables below and runs separately.

## Capture check

An ignored test can create a PNG without a visible window:

```bash
MATRIX_CAPTURE_SIZE=1280x720 MATRIX_CAPTURE_FRAME=121 \
  MATRIX_CAPTURE_FILE=/tmp/matrix-1280x720-f121.png \
  cargo test capture_native_frame -- --ignored
```

The test advances the GPU state at 60 Hz; frame 121 represents approximately two seconds of animation.
