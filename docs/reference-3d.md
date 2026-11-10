# Matrix 3D reference study

Source: the local `reference/matrix` copy of [Rezmason/matrix](https://github.com/Rezmason/matrix), `index.html?version=3d`. The source has no recorded commit, so it is not established that it is identical to the current upstream or the hosted reference. The WebGL (`regl`) path is authoritative here. Values below come from the local source, not the upstream README, which disagrees about forward speed.

## Resolved configuration

`config.js` merges defaults, the 3d preset, the chosen font, then URL overrides. With only `version=3d`:

| Group | Effective values |
| --- | --- |
| Rendering | `renderer=regl`, `effect=palette`, `resolution=0.75`, `fps=60`, `useHalfFloat=false`, `volumetric=true`, `isometric=false`, `useHoloplay=false` |
| Grid and movement | `numColumns=80`, `density=1`, `animationSpeed=1`, `forwardSpeed=0.25`, `fallSpeed=0.5`, `cycleSpeed=0.03`, `cycleFrameSkip=1`, `raindropLength=0.3`, `loops=false`, `skipIntro=true` |
| Glyph | `font=matrixcode`, `assets/matrixcode_msdf.png`, 57 symbols, 8×8 atlas; `glyphHeightToWidth=1`, `glyphVerticalSpacing=1`, `glyphEdgeCrop=0`, `glyphFlip=false`, `glyphRotation=0` |
| Brightness | `baseBrightness=-0.9`, `baseContrast=1.5`, `brightnessOverride=0`, `brightnessThreshold=0`, `brightnessDecay=1`, `ditherMagnitude=0.05` |
| Glow | `bloomStrength=0.7`, `bloomSize=0.4`, `highPassThreshold=0.1` |
| Color | black background HSL(0,0,0); isolated cursor HSL(0.242,1,0.73), intensity 2. Palette stops `(position: H,S,L)`: `0: 0.3,0.9,0`; `0.2: 0.3,0.9,0.2`; `0.7: 0.3,0.9,0.7`; `0.8: 0.3,0.9,0.8`, clamped after last stop |
| Inactive effects | no base or glint texture, `isolateGlint=false`, glint HSL(0,0,1), intensity 1, brightness -1.5, contrast 2.5; `hasThunder=false`, `rippleTypeName=null`, `isPolar=false`, `slant=0`, `useCamera=false` |
| Remaining defaults | `rippleThickness=0.2`, `rippleScale=30`, `rippleSpeed=0.2`, `suppressWarnings=false`, `testFix=null`; `once` is undefined |

Canvas backing dimensions are `ceil(CSS size × devicePixelRatio × 0.75)` in each axis. This is an internal resolution, not a change in CSS size. The 3D effect uses an 80×80 cell grid and 6 vertices per cell: 6,400 glyph quads and 38,400 vertices, irrespective of viewport aspect. Alpha and color target precision matter: simulation textures are half-float, while render and postprocess targets default to unsigned normalized 8-bit.

## Passes and animation

The pipeline in `js/regl/main.js` is rain → bloom → palette → quilt → screen. Quilt is a no-op without Looking Glass. The full-screen scope disables depth testing. The rain render uses additive one/one blending, with no depth occlusion. Four ping-pong compute targets advance per browser animation tick, even when presentation is frame-limited; `fps=60` renders on every tick, not a strict 60 Hz cap.

1. **Intro:** one cell per column. `skipIntro=true` outputs 2 immediately, so the screen begins with active rain rather than a blank-screen reveal.
2. **Rain state:** an 80×80 half-float texture stores brightness, cursor flag, activation and intro progress. For each column, a deterministic hash picks an initial time offset and a speed factor between 0.5 and 1. `columnTime = hash(x)*1000 + time*0.5*speed`; `rainTime = (y*0.01 + columnTime)/0.3`; wobble it with `t + 0.3*sin(sqrt(2)*t) + 0.2*sin(sqrt(5)*t)`; brightness is `1-fract(t)`. A brightness discontinuity relative to the cell below selects the leading cursor. Brightness decay 1 means no temporal smoothing.
3. **Symbols:** the second 80×80 half-float state stores glyph index and age. On the first tick, deterministic position hashes choose both. Age increases by 0.03 each tick and, at 1, a time/position hash picks a new glyph. This is tick-based, so its real-world cycle rate depends on display refresh.
4. **Effects:** the third 80×80 half-float texture outputs multiplier 1 and addition 0 for this preset. Thunder and ripples are disabled.
5. **Geometry and MSDF:** each column has a shared random depth `fract(hash(column,0)+time*0.25)` and a shared random vertical offset of up to one cell. Quads cover a fixed world-space grid, translated by z=-1 and projected using a 90° perspective camera, near 0.0001, far 1000 and viewport aspect. As depth increases, columns approach the eye and wrap abruptly to the distant plane; the shader multiplies brightness by depth. The 512×512 RGB MSDF is loaded with linear filtering and vertical flip; atlas row index is reversed in the shader. Reconstruct edges using median RGB, derivatives and pixel range 4. With the intro skipped, base is `rainBrightness*1.5-0.9`; cursor and ordinary glyphs are written to separate channels, multiplied by the MSDF mask. See `rainPass.vert.glsl` and `rainPass.frag.glsl` for coordinate conventions and exact clipping behavior.
6. **Bloom:** per-channel high-pass removes values below 0.1. Five high-pass levels start at 40% of the render dimensions and halve each level; each next level samples the preceding *high-pass*, not the blurred image. Each level receives horizontal and vertical 3-tap blurs with center weight 0.442 and side weights 0.279. Combine blurred levels with weights 0.96549, 0.92832, 0.88790, 0.84343, 0.79370, multiplied by 0.7. Match the source's width/height uniform assignments and sampling when comparing output.
7. **Palette:** add rain and bloom, subtract time-varying per-pixel noise up to `0.05/3`, sample a 2048-entry linearly interpolated RGB palette from the red channel, then add the green cursor channel scaled by the cursor color and intensity. The blue glint channel is inactive. Palette entries are converted from HSL before interpolation and quantized to 8-bit.

## Assets and attribution

The only image loaded for this preset is `assets/matrixcode_msdf.png` (512×512, RGB); the other image slots use placeholder textures. The included font file and MSDF generation sources are not needed to render the existing atlas. Original source attribution and license are preserved in [third-party notices](third-party-notices.md). The provenance and redistribution rights of the glyph artwork remain to be confirmed; the upstream author says the classic glyphs were derived from an archived official Matrix product, so do not infer independent font rights solely from the code license.

## Visual targets and capture record

The reference targets are source-verified from the exact GLSL and preset rather than inferred from a generic Matrix effect: green glyph columns on black; bright pale-green leading glyphs; variable brightness and scale with perspective depth; additive overlapping columns; and soft green bloom. Glyphs cycle in place, while columns fall downward, advance toward the camera, dim by depth, then wrap from the near plane to the distant plane. Startup is already active because `skipIntro=true`.

| Target viewport | Required composition check |
| --- | --- |
| 1280×720 landscape | The 90° perspective gives a broad horizontal field; 80 columns retain their fixed simulation resolution while projected glyph size varies with depth. |
| 720×1280 portrait | The same 80×80 state grid fills a tall projection; do not introduce viewport-dependent cell counts or stretch the glyph atlas. |
| 1024×1024 square | A neutral-aspect comparison for centered perspective, column overlap, bloom spread, cursor brightness, and wrap timing. |

No PNGs could be produced in the execution sandbox: on 2026-11-09 it denied localhost binding (`PermissionError: Operation not permitted`), Chromium crash-reporter socket setup, direct DNS resolution, and browser-service access to `tools.lmx.to`. That is an environment limitation, not an unresolved implementation requirement. During Task 3, capture the hosted reference and the native renderer at the three tabled viewports after startup and compare them against these source-verified targets.
