# Renderer

The app embeds its WGSL shaders from `src/shaders/` and its 512 × 512 MSDF glyph atlas from `assets/matrixcode_msdf.png`. It reads no files or network resources at runtime. Each frame updates an 80 × 80 rain state, renders glyph brightness and a glow mask, blurs the glow mask, applies a 512-color palette, and presents the result.

## Modes

| Setting | Modern (default) | Classic |
| --- | --- | --- |
| Geometry | 6,400 quads, 38,400 vertices, 90° perspective | One full-screen quad, six vertices, no perspective |
| Fall speed / raindrop length | 0.5 / 0.3 | 0.3 / 0.75 |
| Base brightness / contrast | −0.9 / 1.5 | −0.5 / 1.1 |
| Intro | Starts with active rain | Starts black; columns reveal from top in a staggered sequence |
| Bloom strength | 0.7 | 0.55 |

Both modes use the same 57-symbol glyph atlas, 80 × 80 cell state, green palette, four bloom levels, and bright isolated cursor color. Classic's lower bloom strength is an intentional visual adjustment. The default power setting is 30 FPS at 50% internal resolution. `--fps` and `--render-scale` adjust those values without changing the mode.

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

## Capture check

An ignored test can create a PNG without a visible window:

```bash
MATRIX_CAPTURE_SIZE=1280x720 MATRIX_CAPTURE_FRAME=121 \
  MATRIX_CAPTURE_FILE=/tmp/matrix-1280x720-f121.png \
  cargo test capture_native_frame -- --ignored
```

The test advances the GPU state at 60 Hz; frame 121 represents approximately two seconds of animation.
