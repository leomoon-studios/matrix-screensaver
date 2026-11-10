# Renderer validation

## 3B capture run

On the NVIDIA GeForce RTX 3060, native reference frames were captured at frame 1 and frame 121 (two seconds at 60 Hz) for 1280 × 720, 720 × 1280, and 1024 × 1024. Each native capture drives the same copied reference state, rain, bloom, palette, and end shaders that the application uses.

The locally hosted `version=3d` reference was captured at the same sizes and times through Chromium's WebGL fallback. It confirmed the visual model: vertical streams, bright heads, fading trails, projected depth scaling, green palette output, and bloom. Its legacy state textures produce noticeably different density and brightness from the WebGPU implementation, so those screenshots are qualitative motion checks rather than a pixel threshold for the native renderer.

Chromium 155's headless mode on this host exposes `navigator.gpu` but fails to create a WebGPU adapter, so it cannot capture the hosted WebGPU renderer. The repository protects the actual parity boundary by testing that all five active native WGSL shader sources exactly equal their local WebGPU reference sources.

## Target hardware and pacing

The application launched successfully at all three capture sizes on an NVIDIA GeForce RTX 3060. It also ran on the active Dell S2721QS Wayland output, which is 3840 × 2160 at 59.997 Hz with a compositor scale of 1.6; resizes rebuild the reference scene and post-processing targets at the surface's physical size.

The event loop uses a 16.666667 ms deadline. Its test verifies that a slow frame skips to the next cadence boundary instead of extending the frame period. Direct presentation profiling on additional GPU models remains future validation work.
