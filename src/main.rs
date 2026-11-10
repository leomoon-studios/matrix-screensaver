use std::{sync::Arc, time::Instant};

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

mod reference_layout;
mod reference_post;
mod reference_resources;

// The reference glyph atlas is embedded at compile time.
const MATRIXCODE_MSDF: &[u8] = include_bytes!("../reference/matrix/assets/matrixcode_msdf.png");
const DEFAULT_RENDER_SCALE: f32 = 0.5;
const DEFAULT_FPS: u32 = 30;

#[derive(Clone, Copy)]
struct Quality {
    fps: u32,
    render_scale: f32,
}

impl Quality {
    fn from_environment() -> Result<Self, String> {
        let mut quality = Self {
            fps: DEFAULT_FPS,
            render_scale: DEFAULT_RENDER_SCALE,
        };
        if let Ok(value) = std::env::var("MATRIX_FPS") {
            quality.fps = value
                .parse()
                .map_err(|_| "MATRIX_FPS must be 1–60".to_owned())?;
        }
        if let Ok(value) = std::env::var("MATRIX_RENDER_SCALE") {
            quality.render_scale = value
                .parse()
                .map_err(|_| "MATRIX_RENDER_SCALE must be 0.25–1".to_owned())?;
        }
        if !(1..=60).contains(&quality.fps) || !(0.25..=1.0).contains(&quality.render_scale) {
            return Err(
                "MATRIX_FPS must be 1–60 and MATRIX_RENDER_SCALE must be 0.25–1".to_owned(),
            );
        }
        Ok(quality)
    }

    fn render_size(self, width: u32, height: u32) -> (u32, u32) {
        (
            ((width as f32 * self.render_scale).round() as u32).max(1),
            ((height as f32 * self.render_scale).round() as u32).max(1),
        )
    }

    fn frame_time(self) -> std::time::Duration {
        std::time::Duration::from_secs_f64(1.0 / self.fps as f64)
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("failed to create the window event loop");
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App {
        initial_size: initial_size(),
        renderer: None,
    };
    event_loop.run_app(&mut app).expect("event loop error");
}

fn initial_size() -> PhysicalSize<u32> {
    let Ok(size) = std::env::var("MATRIX_WINDOW_SIZE") else {
        return PhysicalSize::new(1280, 720);
    };
    let Some((width, height)) = size.split_once('x') else {
        return PhysicalSize::new(1280, 720);
    };
    let (Ok(width), Ok(height)) = (width.parse(), height.parse()) else {
        return PhysicalSize::new(1280, 720);
    };
    PhysicalSize::new(width, height)
}

struct App {
    initial_size: PhysicalSize<u32>,
    renderer: Option<Renderer>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_title("Matrix Screensaver")
                        .with_inner_size(self.initial_size)
                        .with_resizable(true),
                )
                .expect("failed to create the Matrix window"),
        );

        match pollster::block_on(Renderer::new(window)) {
            Ok(renderer) => self.renderer = Some(renderer),
            Err(error) => {
                eprintln!("Matrix Screensaver could not initialize its GPU renderer: {error}");
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        if renderer.window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => renderer.resize(size),
            WindowEvent::Occluded(occluded) => renderer.set_occluded(occluded),
            WindowEvent::RedrawRequested => renderer.render(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(renderer) = &self.renderer
            && !renderer.occluded
            && Instant::now() >= renderer.next_frame_at
        {
            renderer.window.request_redraw();
        }
    }
}

struct Renderer {
    reference_resources: reference_resources::ReferenceResources,
    reference_post: reference_post::ReferencePost,
    reference_frame: i32,
    instance: wgpu::Instance,
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    started_at: Instant,
    next_frame_at: Instant,
    quality: Quality,
    occluded: bool,
    occluded_at: Option<Instant>,
    benchmark_since: Option<Instant>,
    benchmark_frames: u32,
    benchmark_last_frame: Option<Instant>,
    benchmark_intervals_ms: Vec<f64>,
}

impl Renderer {
    async fn new(window: Arc<Window>) -> Result<Self, String> {
        let quality = Quality::from_environment()?;
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window.clone())
            .map_err(|error| format!("could not create a rendering surface: {error}"))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|error| format!("no compatible GPU adapter: {error}"))?;
        if adapter.get_info().device_type == wgpu::DeviceType::Cpu {
            return Err("only a software graphics adapter was found; this GPU renderer would consume substantial CPU time".to_owned());
        }
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Matrix Screensaver device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|error| format!("could not create GPU device: {error}"))?;

        let size = window.inner_size();
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .or_else(|| capabilities.formats.first().copied())
            .ok_or_else(|| "surface has no supported formats".to_owned())?;
        let present_mode = capabilities
            .present_modes
            .iter()
            .copied()
            .find(|mode| *mode == wgpu::PresentMode::Fifo)
            .or_else(|| capabilities.present_modes.first().copied())
            .ok_or_else(|| "surface has no supported present modes".to_owned())?;
        let alpha_mode = capabilities
            .alpha_modes
            .first()
            .copied()
            .ok_or_else(|| "surface has no supported alpha modes".to_owned())?;

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(&device, &config);
        let (render_width, render_height) = quality.render_size(config.width, config.height);
        let reference_resources = reference_resources::ReferenceResources::new(
            &device,
            &queue,
            MATRIXCODE_MSDF,
            render_width,
            render_height,
        )?;
        let reference_post = reference_post::ReferencePost::new(
            &device,
            render_width,
            render_height,
            config.format,
            &reference_resources.time_buffer,
            reference_resources.primary_view(),
            reference_resources.high_pass_view(),
        );

        eprintln!(
            "GPU ready: {} (bundled matrix glyph atlas: {} bytes)",
            adapter.get_info().name,
            MATRIXCODE_MSDF.len()
        );

        Ok(Self {
            reference_resources,
            reference_post,
            reference_frame: 0,
            instance,
            window,
            surface,
            device,
            queue,
            config,
            started_at: Instant::now(),
            next_frame_at: Instant::now(),
            quality,
            occluded: false,
            occluded_at: None,
            benchmark_since: std::env::var("MATRIX_BENCHMARK")
                .is_ok_and(|value| value == "1")
                .then(Instant::now),
            benchmark_frames: 0,
            benchmark_last_frame: None,
            benchmark_intervals_ms: Vec::new(),
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        let (render_width, render_height) = self.quality.render_size(size.width, size.height);
        self.reference_resources
            .resize(&self.device, &self.queue, render_width, render_height);
        self.reference_post.resize(
            &self.device,
            render_width,
            render_height,
            self.reference_resources.primary_view(),
            self.reference_resources.high_pass_view(),
        );
    }

    fn render(&mut self, event_loop: &ActiveEventLoop) {
        if self.occluded || Instant::now() < self.next_frame_at {
            return;
        }
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                drop(frame);
                self.surface.configure(&self.device, &self.config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self
                    .instance
                    .create_surface(self.window.clone())
                    .expect("window surface was lost and could not be recreated");
                self.surface.configure(&self.device, &self.config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => return,
            wgpu::CurrentSurfaceTexture::Validation => {
                eprintln!("GPU surface validation failed; exiting Matrix Screensaver");
                event_loop.exit();
                return;
            }
        };

        self.queue.write_buffer(
            &self.reference_resources.time_buffer,
            0,
            bytemuck::bytes_of(&reference_layout::ReferenceTime {
                seconds: self.started_at.elapsed().as_secs_f32(),
                frames: self.reference_frame,
                _padding: [0; 2],
            }),
        );
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Matrix reference frame"),
            });
        self.reference_resources.encode_simulation(&mut encoder);
        self.reference_resources.encode_render(&mut encoder);
        self.reference_post.encode(&mut encoder, &view);
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
        self.reference_frame = self.reference_frame.wrapping_add(1);
        let now = Instant::now();
        if let Some(since) = self.benchmark_since {
            if let Some(last) = self.benchmark_last_frame {
                self.benchmark_intervals_ms
                    .push(now.duration_since(last).as_secs_f64() * 1000.0);
            }
            self.benchmark_last_frame = Some(now);
            self.benchmark_frames += 1;
            let elapsed = now.duration_since(since);
            if elapsed.as_secs() >= 10 {
                self.benchmark_intervals_ms.sort_unstable_by(f64::total_cmp);
                let count = self.benchmark_intervals_ms.len();
                let p95 = self
                    .benchmark_intervals_ms
                    .get(count.saturating_sub(1) * 95 / 100)
                    .copied()
                    .unwrap_or(0.0);
                let worst = self.benchmark_intervals_ms.last().copied().unwrap_or(0.0);
                eprintln!(
                    "Matrix benchmark: {:.1} FPS over {:.1} s ({} frames); interval p95 {:.2} ms, worst {:.2} ms",
                    self.benchmark_frames as f64 / elapsed.as_secs_f64(),
                    elapsed.as_secs_f64(),
                    self.benchmark_frames,
                    p95,
                    worst,
                );
                self.benchmark_since = Some(now);
                self.benchmark_frames = 0;
                self.benchmark_intervals_ms.clear();
            }
        }
        self.next_frame_at =
            next_frame_deadline(self.next_frame_at, now, self.quality.frame_time());
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame_at));
    }

    fn set_occluded(&mut self, occluded: bool) {
        if self.occluded == occluded {
            return;
        }
        self.occluded = occluded;
        if occluded {
            self.occluded_at = Some(Instant::now());
            self.benchmark_last_frame = None;
        } else {
            if let Some(since) = self.occluded_at.take() {
                let paused = since.elapsed();
                self.started_at += paused;
                if let Some(benchmark_since) = &mut self.benchmark_since {
                    *benchmark_since += paused;
                }
            }
            self.next_frame_at = Instant::now();
            self.window.request_redraw();
        }
    }
}

fn next_frame_deadline(
    previous: Instant,
    now: Instant,
    frame_time: std::time::Duration,
) -> Instant {
    let mut deadline = previous + frame_time;
    while deadline <= now {
        deadline += frame_time;
    }
    deadline
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_pacer_holds_a_60_hz_deadline_after_slow_frames() {
        let start = Instant::now();
        let frame_time = Quality {
            fps: 60,
            render_scale: 1.0,
        }
        .frame_time();
        assert_eq!(
            next_frame_deadline(start, start, frame_time),
            start + frame_time
        );
        assert_eq!(
            next_frame_deadline(start, start + frame_time + frame_time / 2, frame_time),
            start + frame_time * 2
        );
    }

    #[test]
    fn copied_gpu_shaders_match_the_local_reference() {
        assert_eq!(
            include_str!("shaders/reference-rain.wgsl"),
            include_str!("../reference/matrix/shaders/wgsl/rainPass.wgsl")
        );
        assert_eq!(
            include_str!("shaders/reference-bloom-blur.wgsl"),
            include_str!("../reference/matrix/shaders/wgsl/bloomBlur.wgsl")
        );
        assert_eq!(
            include_str!("shaders/reference-bloom-combine.wgsl"),
            include_str!("../reference/matrix/shaders/wgsl/bloomCombine.wgsl")
        );
        assert_eq!(
            include_str!("shaders/reference-palette.wgsl"),
            include_str!("../reference/matrix/shaders/wgsl/palettePass.wgsl")
        );
        assert_eq!(
            include_str!("shaders/reference-end.wgsl"),
            include_str!("../reference/matrix/shaders/wgsl/endPass.wgsl")
        );
    }
}
