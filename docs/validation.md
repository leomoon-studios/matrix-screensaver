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
