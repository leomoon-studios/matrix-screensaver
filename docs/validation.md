# Renderer validation

## 3B capture run

On the NVIDIA GeForce RTX 3060, native reference frames were captured at frame 1 and frame 121 (two seconds at 60 Hz) for 1280 × 720, 720 × 1280, and 1024 × 1024. Each native capture drives the same copied reference state, rain, bloom, palette, and end shaders that the application uses.

The locally hosted `version=3d` reference was captured at the same sizes and times through Chromium's WebGL fallback. It confirmed the visual model: vertical streams, bright heads, fading trails, projected depth scaling, green palette output, and bloom. Its legacy state textures produce noticeably different density and brightness from the WebGPU implementation, so those screenshots are qualitative motion checks rather than a pixel threshold for the native renderer.

Chromium 155's headless mode on this host exposes `navigator.gpu` but fails to create a WebGPU adapter, so it cannot capture the hosted WebGPU renderer. The repository protects the actual parity boundary by testing that all five active native WGSL shader sources exactly equal their local WebGPU reference sources.

## Target hardware and pacing

The application launched successfully at all three capture sizes on an NVIDIA GeForce RTX 3060. It also ran on the active Dell S2721QS Wayland output, which is 3840 × 2160 at 59.997 Hz with a compositor scale of 1.6; resizes rebuild the reference scene and post-processing targets at the surface's physical size.

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

The 75% internal scale matches the hosted reference's resolution setting. The full-size presentation surface and copied rain, bloom, and palette shaders remain in use. A 1280 × 720 live capture showed the expected falling streams, bright heads, darkening tails, depth scaling, and green bloom. GPU timing per pass is still outstanding.

The Radeon 610M integrated GPU on this host was selected through `WGPU_BACKEND=vulkan VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json`. It created a presentable Wayland surface and ran the 50% scale low-power preset at 30.0–30.1 FPS over three 10-second intervals at the tiled large-window size. Resident RAM was about 98 MiB and CPU use about 1.5% of one core during a 15-second sample. The `/sys/class/drm/card0/device/gpu_busy_percent` counter was 0 in most samples and briefly 99, so it is not treated as a reliable workload average here. A CPU software adapter exits with an explicit message; there is no software Vulkan ICD installed on this host to exercise that path.

On the NVIDIA GPU, a separate 30-second large-window low-power run held 30.0–30.1 FPS. Its 95th percentile render-submission interval was 33.51–33.59 ms, with a worst interval of 34.29 ms. Total GPU utilization averaged 13.8% and board power 51.6 W during that run. These system counters include other desktop activity.

Repeatable commands on this host:

```bash
cargo build --release
python scripts/benchmark.py target/release/matrix-screensaver --seconds 60 --size 1280x720 --float
python scripts/benchmark.py target/release/matrix-screensaver --seconds 60 --size 1280x720
WGPU_BACKEND=vulkan VK_DRIVER_FILES=/usr/share/vulkan/icd.d/radeon_icd.json MATRIX_LOW_POWER=1 MATRIX_BENCHMARK=1 timeout 35s target/release/matrix-screensaver
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
python scripts/benchmark_dual.py target/release/matrix-screensaver --seconds 60
python scripts/benchmark_dual.py target/release/matrix-screensaver --seconds 60 --low-power
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
