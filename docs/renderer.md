# Reference renderer settings

The native window runs the copied WebGPU `version=3d` pipeline in this order: `computeIntro`, `computeMain`, `vertMain`/`fragMain`, four horizontal and vertical bloom levels, bloom combine, palette compute, and final presentation. The shaders are embedded from `src/shaders/reference-*.wgsl`; the Matrix code MSDF atlas is embedded from `reference/matrix/assets/matrixcode_msdf.png`. Runtime drawing uses no browser, file lookup, or network request.

## Resolved `version=3d` values

| Group | Values |
| --- | --- |
| Grid and glyphs | 80 × 80 cells; 57 glyphs in an 8 × 8 atlas; height/width 1; vertical spacing 1; edge crop 0; density 1; no flip or rotation |
| State | animation speed 1; fall speed 0.5; raindrop length 0.3; brightness threshold 0; override 0; decay 1; cursor brightness 0; cycle speed 0.03; cycle frame skip 1; intro skipped |
| Geometry | volumetric on; forward speed 0.25; 90° perspective; near 0.0001, far 1000; camera aspect = window width/height; z = -1 translation; no isometric, polar, slant, loops, thunder, ripple, or base/glint textures |
| Rain color channels | MSDF range 4; base brightness -0.9, contrast 1.5; glint brightness -1.5, contrast 2.5; cursor isolated; glint not isolated; high-pass threshold 0.1 |
| Bloom | four levels; first level `floor(window size × 0.4)` then each level halves; Gaussian radius 2; strength 0.7; source is the rain high-pass target |
| Palette | 512 colors from HSL hue 0.3, saturation 0.9, lightness 0/0.2/0.7/0.8 at positions 0/0.2/0.7/0.8; dither magnitude 0.05; black background; cursor HSL (0.242, 1, 0.73) at intensity 2; white glint at intensity 1 |

The rain pass uses 6,400 quads, or 38,400 vertices. Each frame dispatches the intro pass in 3 × 1 workgroups and the main pass in 3 × 80 workgroups, then draws those quads. The event loop schedules frames about 16.67 ms apart. `MATRIX_WINDOW_SIZE=WIDTHxHEIGHT` sets the initial window size; resizing updates the camera and render targets.

## Comparison capture

The ignored `capture_native_reference_frame` test creates reproducible native PNGs without a visible window. For example:

```bash
MATRIX_CAPTURE_SIZE=1280x720 MATRIX_CAPTURE_FRAME=121 \
  MATRIX_CAPTURE_FILE=/tmp/native-1280x720-f121.png \
  cargo test capture_native_reference_frame -- --ignored
```

The test steps the reference pipeline at 60 Hz, so frame 121 is the two-second frame. It is used alongside the locally hosted reference source at the same size and timestamp. Chromium's headless WebGPU path did not produce drawable pixels on the target driver, while its WebGL fallback uses a legacy state texture path and is retained only as a qualitative motion reference.

## GPU resources

| Resource | Size and format |
| --- | --- |
| Config, Time, Scene uniforms | 192, 16, and 144 bytes |
| Cell, IntroCell storage | 6,400 × 48 = 307,200 bytes; 80 × 16 = 1,280 bytes |
| Glyph atlas | Embedded RGBA8 unorm texture, sampled linearly |
| Rain brightness and high-pass | Two full-window RGBA8 unorm render targets with additive blending |
| Bloom pyramid | Four horizontal and four vertical RGBA8 unorm storage textures; first level is 40% of window size, rounded down |
| Bloom combine | RGBA8 unorm storage texture at the first bloom level's size |
| Palette lookup | 512 `vec3<f32>` values at 16-byte stride in an 8,192-byte uniform buffer |
| Palette output | Full-window RGBA8 unorm storage texture |
| Window surface | Prefers an available sRGB format and FIFO presentation; final pass samples the palette output with nearest filtering |

Task 3B will compare captured native frames and motion against the hosted reference at matched sizes and timestamps.
