# Classic 2D reference and native preset

The classic reference is `reference/matrix/index.html?skipIntro=false` (or the hosted page at `https://tools.lmx.to/matrix/?skipIntro=false`). `config.js` merges its defaults with `versions.classic`, which is empty, then applies the URL override. The supplied capture shows the intended sparse, flat green columns, bright pale heads, fading trails, and black background during the reveal. The local source has no recorded upstream revision, so an exact match to the current hosted build cannot be established from source alone.

| Group | Effective reference values |
| --- | --- |
| Renderer | `volumetric=false`, `effect=palette`, `renderer=regl`, `resolution=0.75`, `fps=60`, `skipIntro=false` via URL |
| Rain | 80×80 state grid, `numColumns=80`, `density=1`, `fallSpeed=0.3`, `raindropLength=0.75`, `animationSpeed=1`, `cycleSpeed=0.03`, `cycleFrameSkip=1`, `brightnessDecay=1`, `loops=false` |
| Glyph | `matrixcode` 57-symbol 8×8 MSDF atlas; height-to-width 1, vertical spacing 1, no rotation, flip, or edge crop |
| Brightness | base brightness −0.5, base contrast 1.1, glint brightness −1.5, glint contrast 2.5, high-pass threshold 0.1, dither magnitude 0.05 |
| Color and bloom | Black background, HSL(0.3, 0.9, lightness) palette at 0→0, 0.2→0.2, 0.7→0.7, 0.8→0.8; isolated cursor HSL(0.242,1,0.73) at intensity 2; bloom strength 0.7 and size 0.4 |
| Inactive | No thunder, ripple, polar transform, base/glint texture, or camera movement. `forwardSpeed=0.25` is inherited but unused when `volumetric=false`. |

The non-volumetric vertex path draws one six-vertex full-screen quad. The fragment shader samples the 80×80 simulation state through screen UVs. It does not apply the perspective camera or per-column depth. The native renderer shares the embedded glyph atlas and bloom and palette passes with `modern`. The native classic bloom strength is intentionally reduced to **0.55** from the reference's **0.7** after visual review; modern remains at 0.7.

With `skipIntro=false`, the intro shader gives columns 40 and 60 offsets of −1 and −2 seconds. Other columns get a deterministic staggered offset of roughly −2.5 to −8.5 seconds. Intro progress advances at `0.375 × (elapsed seconds + offset)` and activates cells from the top down. A column takes about 2.67 seconds to fill once its offset has elapsed. The compute shader keeps activated cells active, so the intro begins on black and turns into continuous falling rain. A fresh process starts with a new zeroed simulation state and time; resize keeps the existing state and elapsed time. Occlusion pauses animation time in the existing frame pacer.

The app defaults to 30 FPS and 50% internal resolution for power, compared with the webpage's 60 FPS and 75% resolution. Use `--type classic --fps 60 --render-scale 0.75` for a closer source comparison. These options do not change the reference colors, geometry, or intro timing.
