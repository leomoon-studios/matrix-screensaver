# Renderer validation

The current default is 30 FPS at 50% internal scale. Earlier sections describe measurements made before this default changed; their 60 FPS and 75% scale runs remain historical comparisons. Use `MATRIX_FPS=60 MATRIX_RENDER_SCALE=0.75` to reproduce those quality settings.

## 3B capture run

On the NVIDIA GeForce RTX 3060, native frames were captured at frame 1 and frame 121 (two seconds at 60 Hz) for 1280 × 720, 720 × 1280, and 1024 × 1024. Each capture drives the same rain, bloom, palette, and presentation shaders that the application uses. The frames showed vertical streams, bright heads, fading trails, projected depth scaling, green color, and bloom. The app also launched successfully at all three sizes.

## Target hardware and pacing

The app also ran on the active Dell S2721QS Wayland output, which is 3840 × 2160 at 59.997 Hz with a compositor scale of 1.6; resizes rebuild the scene and post-processing targets at the surface's physical size.

The event loop uses a 16.666667 ms deadline. Its test verifies that a slow frame skips to the next cadence boundary instead of extending the frame period. Direct presentation profiling on additional GPU models remains future validation work.

## 3C performance check

The current release build was compared with commit `082c0fd` on the NVIDIA GeForce RTX 3060 under the same desktop session. The Dell S2721QS runs at 3840 × 2160, 59.997 Hz, scale 1.6. Hyprland tiled the app to 2376 × 1272 logical pixels, or approximately 3802 × 2035 surface pixels, for the large-window runs. Each build was run twice, with 10 seconds of idle samples and 60 seconds of running samples per run, sampled once per second with `scripts/benchmark.py`. The 3C build reports consecutive 10-second intervals at 59.9–60.1 FPS. The old build did not log measured FPS.

| Large-window metric | Commit `082c0fd` | 3C build |
| --- | ---: | ---: |
| Per-process GPU allocation | 548 MiB both runs | 260–262 MiB |
| Resident RAM, run means | 160.1–194.7 MiB | 154.8–157.2 MiB |
| Process CPU, run means | 2.5% (second run) | 2.4–2.5% |
| GPU utilization, running mean | 22.9–23.0% | 19.9% |
| GPU utilization, idle mean | 18.0–20.3% | 18.5–18.7% |
| Board power, running mean | 58.1–58.3 W | 55.8–56.0 W |
| Board power, idle mean | 56.6–57.2 W | 56.7–57.0 W |

The direct process GPU allocation fell by 286–288 MiB (52%). The desktop's other GPU allocations varied between runs, so total GPU memory and board power are context, not a precise measure of the screensaver alone. The configured frame period is 16.67 ms at 60 FPS. The low-power preset targets 33.33 ms at 30 FPS.

For 1280 × 720, the benchmark temporarily floated the window at 800 × 450 logical pixels on the same scale-1.6 monitor. In repeated 60-second release runs, per-process GPU allocation was 214 MiB before and 208–210 MiB after; resident RAM was about 160 MiB before and 155–157 MiB after. Both builds averaged about 15% total NVIDIA utilization and 53 W board power. The small-size allocation is dominated by driver and surface overhead, so the savings there are modest. The current build reported 59.9–60.1 FPS; across twelve 10-second reporting windows the 95th percentile interval between completed render submissions was 16.75–16.87 ms and the worst interval was 17.74 ms. This is a CPU-side cadence measurement, not a GPU execution timestamp. The old build had no interval logger. In the repeated small run, system CPU averaged 3.9% idle and 2.0% running for the old build, and 2.7% idle and 2.0% running for the new build; this reflects changing desktop activity rather than an app-only CPU measure. Process CPU averaged 2.3–2.4% for both.

A live resize probe switched the current build between 1280 × 720 and approximately 3802 × 2035 twice. Its process GPU allocation alternated 208, 260, 208, 260, and 208 MiB, so the large targets were released after each shrink instead of accumulating.

The native live `Rgba8Unorm` targets are two full-size rain outputs, one full-size palette output, eight blur textures across four levels, and one combined bloom texture. The bloom base is 40% of internal width and height. Their expected unpadded storage is about 105.8 MiB at the former full 3802 × 2035 internal size and 59.5 MiB at the current 75% internal size (approximately 2852 × 1526); at 1280 × 720 it drops from 12.6 to 7.1 MiB. The fixed atlas is 512 × 512 RGBA (1 MiB), cell state is about 0.3 MiB, and the remaining placeholders and uniforms are small. The surface swapchain is allocated by the driver at the physical window size, with one frame of requested maximum latency; its exact backing count is not exposed by `wgpu`. The expected target bytes also exclude driver allocation granularity and transient resize overlap. Production render targets no longer request `COPY_SRC`; capture tests still do.

The 75% internal scale gives a sharper image than the 50% low-power setting. The full-size presentation surface and embedded rain, bloom, and palette shaders remain in use. A 1280 × 720 live capture showed falling streams, bright heads, darkening tails, depth scaling, and green bloom. GPU timing per pass is still outstanding.

The Radeon 610M integrated GPU on this host was selected through `WGPU_BACKEND=vulkan VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json`. It created a presentable Wayland surface and ran the 50% scale low-power preset at 30.0–30.1 FPS over three 10-second intervals at the tiled large-window size. Resident RAM was about 98 MiB and CPU use about 1.5% of one core during a 15-second sample. The `/sys/class/drm/card0/device/gpu_busy_percent` counter was 0 in most samples and briefly 99, so it is not treated as a reliable workload average here. A CPU software adapter exits with an explicit message; there is no software Vulkan ICD installed on this host to exercise that path.

On the NVIDIA GPU, a separate 30-second large-window low-power run held 30.0–30.1 FPS. Its 95th percentile render-submission interval was 33.51–33.59 ms, with a worst interval of 34.29 ms. Total GPU utilization averaged 13.8% and board power 51.6 W during that run. These system counters include other desktop activity.

Repeatable commands on this host:

```bash
cargo build --release
MATRIX_FPS=60 MATRIX_RENDER_SCALE=0.75 python scripts/benchmark.py target/release/matrix-screensaver --seconds 60 --size 1280x720 --float
MATRIX_FPS=60 MATRIX_RENDER_SCALE=0.75 python scripts/benchmark.py target/release/matrix-screensaver --seconds 60 --size 1280x720
WGPU_BACKEND=vulkan VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json MATRIX_BENCHMARK=1 timeout 35s target/release/matrix-screensaver
```

## Two-monitor performance check

With two Dell S2721QS displays connected at 3840 × 2160, 59.997 Hz and scale 1.6, two release-build instances were run simultaneously, one visible on each monitor. DP-2's tiled window was 2376 × 1300 logical pixels (about 3802 × 2080 physical); DP-1's was 2376 × 1272 (about 3802 × 2035). The application currently creates one window per instance, so this measures two instances rather than one process spanning displays. `scripts/benchmark_dual.py` focuses each monitor for launch, verifies the windows are on different monitors, samples 10 seconds idle and 60 seconds running, then restores the original focus. The default run was repeated.

| Both screens active | Default, 60 FPS (two runs) | Low power, 30 FPS (one run) |
| --- | ---: | ---: |
| Combined process GPU memory | 524 MiB | 524 MiB |
| Combined resident RAM | 309.7–310.0 MiB | 309.9 MiB |
| Combined process CPU, run mean | 4.8% of one core | 3.6% of one core |
| NVIDIA GPU utilization, run mean | 39.2–39.4% | 37.8% |
| Board power, idle mean | 22.1 W | 24.0 W |
| Board power, running mean | 52.7–53.0 W | 35.8 W |
| Per-window measured frame rate | 59.9–60.1 FPS | 30.0–30.1 FPS |
| Per-window 95th percentile submission interval | 16.74–16.90 ms | 33.46–33.55 ms |

The low-power preset saved roughly 17 W of measured running board power in this session. The two-instance GPU allocation is about twice the one-window 260–264 MiB allocation, as expected. Total GPU memory and utilization include changing desktop activity: in the two default runs, the idle total was about 4710 MiB and the running total about 5300 MiB, while idle GPU utilization fluctuated widely. Process GPU allocation, frame rate, and the repeated board-power readings were steadier. This check does not show that launching one instance automatically covers both monitors.

```bash
MATRIX_FPS=60 MATRIX_RENDER_SCALE=0.75 python scripts/benchmark_dual.py target/release/matrix-screensaver --seconds 60
python scripts/benchmark_dual.py target/release/matrix-screensaver --seconds 60
```

## Single-monitor comparison with Omarchy effects

With both monitors connected, only one window was animated on DP-2 for each run. Hyprland gave every window 2376 × 1272 logical pixels, approximately 3802 × 2035 physical pixels at scale 1.6. Each run sampled 10 seconds idle and 60 seconds running on the RTX 3060. The native binary was built with `cargo build --release`. Omarchy's installed `omarchy-screensaver` invokes `ttfx` at a configured 120 FPS with the user's branding text and random effects. To compare specific effects, each was repeated inside one Foot terminal using Omarchy's packaged `screensaver.ini` and the same `ttfx` canvas, input, and frame-rate options. This bypassed the normal launcher, which opens a window on every monitor. The five effects were chosen for their full-canvas or particle animation; they are not an exhaustive ranking of all effects. Actual `ttfx` frame rate was not measured.

| One window, 60-second run | GPU board power above its idle mean | Process-tree CPU, % of one core | Resident RAM | GPU allocation |
| --- | ---: | ---: | ---: | ---: |
| Native Matrix, 60 FPS | 17.7 W | 2.8% | 154.8 MiB | 260 MiB process |
| Native Matrix low power, 30 FPS (two runs) | 3.2–4.1 W | 2.2–2.5% | 154.8–154.9 MiB | 260 MiB process |
| Omarchy `ttfx matrix` | 23.0 W | 103.3% | 68.6 MiB | Not attributed by `nvidia-smi` |
| Omarchy `ttfx fireworks` | 22.0 W | 90.4% | 66.2 MiB | Not attributed by `nvidia-smi` |
| Omarchy `ttfx smoke` | 22.5 W | 103.7% | 67.1 MiB | Not attributed by `nvidia-smi` |
| Omarchy `ttfx thunderstorm` | 21.4 W | 89.9% | 67.0 MiB | Not attributed by `nvidia-smi` |
| Omarchy `ttfx blackhole` | 22.9 W | 91.8% | 66.2 MiB | Not attributed by `nvidia-smi` |

The terminal process tree includes Foot, the shell loop, and `ttfx`. `nvidia-smi` listed the native application's GPU memory directly but did not list Foot's allocation, so its GPU-memory column is not directly comparable; total GPU memory varied with other desktop activity. NVIDIA GPU-utilization readings also varied widely at low board power, so board-power delta is the more useful GPU-load comparison here. The native 30 FPS preset used much less CPU and measured board power than the selected Omarchy effects, while retaining about 2.3 times their resident RAM. The native 60 FPS setting also used less CPU and about 4–5 W less incremental board power. These effects have different visuals and their configured frame rates differ.

## 3C final checks

`MATRIX_PROFILE_GPU=1` enables six timestamp queries for one frame every 300 frames. The sample divides GPU execution into state simulation, rain geometry, all bloom passes, palette, and final presentation. On the RTX 3060 at the 30 FPS, 50% internal-scale default, a tiled approximately 3802 × 2035 physical-pixel window gave warm GPU times of 0.028, 0.085, 0.157, 0.100, and 0.113 ms respectively. At 1280 × 720 physical pixels, a sample gave 0.058, 0.114, 0.204, 0.084, and 0.075 ms. These are sampled frames, not means across a full minute; warm-up and GPU clock changes affect the figures. Bloom is the largest measured block, but all measured passes together are well below the 33.33 ms frame budget. Profiling is opt-in and requires GPU timestamp support.

The DP-2 monitor was switched off for eight seconds using Hyprland DPMS while Matrix ran, then restored automatically. The first check exposed an animation-clock gap; the renderer now detects a gap over 250 ms between presented frames and excludes it from animation time and FPS interval reporting. A repeat produced normal 30.1 FPS 10-second reporting intervals around the blackout, with 95th-percentile intervals of 33.61–33.67 ms. This verifies pause and resume on the tested Hyprland/NVIDIA setup; other compositors may report occlusion differently.

Live captures were inspected after resizing 1280 × 720 → approximately 3802 × 2035 → 1280 × 720. The image refilled each surface without a stale target, crop, or stretch. Falling streams, large near glyphs, dim distant glyphs, bright heads, darkening trails, and bloom remained visible after the final resize. The 50% internal scale softens small glyph edges and bloom detail compared with 75% scale. That is the accepted quality tradeoff for the measured low-power default; `MATRIX_RENDER_SCALE=0.75 MATRIX_FPS=60` restores the earlier presentation setting.

The final default was also measured twice for 60 seconds at 1280 × 720 physical pixels on the scale-1.6 display. It held 30.0–30.1 FPS in all twelve 10-second reports, with 95th-percentile render-submission intervals of 33.55–33.83 ms. Each run used 208 MiB of process GPU allocation and about 155 MiB of resident RAM; process CPU averaged 2.4–2.6% of one core. Idle board power averaged 16.7–16.8 W and running power 18.4 W, an increase of 1.6–1.7 W in this session. The two earlier 60-second large-window low-power runs in the single-monitor comparison measured 3.2–4.1 W over idle. The historical pre-3C build did not have FPS or interval logging; its process GPU, CPU, RAM, GPU utilization, and board-power measurements are available above.

## 3D and classic 2D type check

On 2026-11-09, the release binary was launched at a verified 1280 × 720 physical window on the scale-1.6 RTX 3060 display. Each type ran for 60 seconds at the default 30 FPS and 50% render scale, with a separate 5-second idle sample. The benchmark script accepts `--type` and reports the observed physical window size.

| 60-second run | Modern | Classic |
| --- | ---: | ---: |
| Measured FPS, six 10-second reports | 30.0–30.1 | 30.0–30.1 |
| 95th percentile frame interval | 33.46–33.52 ms | 33.46–33.50 ms |
| Process CPU, run mean | 2.1% of one core | 2.0% |
| Resident RAM, run mean | 155.7 MiB | 155.7 MiB |
| Process GPU allocation | 208 MiB | 208 MiB |
| Board power, idle → run mean | 32.9 → 34.2 W | 32.8 → 34.0 W |
| GPU utilization, run mean | 36.3% | 37.2% |

Both types share the 80×80 state and post-processing targets, which explains their equal measured GPU allocation; classic draws one six-vertex quad in place of modern's 38,400 vertices. The separate idle samples had substantial background GPU activity (about 39% utilization), and each run had brief board-power spikes near 50 W, so the small mean differences are not evidence of a stable power advantage for either type. Frame rate and process memory were stable.

These measurements preceded a classic-only bloom-strength adjustment from 0.7 to 0.55. The change alters a uniform value and allocates no additional resources; a new 1280 × 720 live capture was inspected after the adjustment, and the full test suite and release build passed. The 60-second performance numbers above apply to the original 0.7 classic setting.

Classic frames were captured at startup, during reveal, and after reveal at 1280 × 720 and at a requested 3840 × 2160 physical window. A GPU readback test also verified that the first frame is black, that more cells activate by 10 seconds, and that resizing during the intro retains the state. The native mid-intro image has flat columns, downward bright heads, green fading trails, and black background like the supplied capture. A direct headless Chromium capture of the hosted classic page was also inspected with `suppressWarnings=true`; the native image has noticeably stronger bloom and grain at comparable steady state, so a pixel-level visual match is not claimed. Different capture timing and graphics backends also limit a direct frame comparison. The 4K floating window extended 73 logical pixels past the monitor's left edge, so its screenshots cover the visible portion. Modern retained its prior geometry and post-processing path, and the existing GPU tests passed.

```bash
python scripts/benchmark.py target/release/matrix-screensaver --type classic --seconds 60 --idle-seconds 5 --float --size 1280x720
python scripts/benchmark.py target/release/matrix-screensaver --type modern --seconds 60 --idle-seconds 5 --float --size 1280x720
```
