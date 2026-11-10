use std::{sync::Arc, time::Instant};

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

mod gpu_layout;
mod post;
mod rain;

// The glyph atlas is embedded at compile time.
const MATRIXCODE_MSDF: &[u8] = include_bytes!("../assets/matrixcode_msdf.png");
const DEFAULT_RENDER_SCALE: f32 = 0.5;
const DEFAULT_FPS: u32 = 30;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RainType {
    Modern,
    Classic,
}

#[derive(Clone, Copy)]
struct Quality {
    fps: u32,
    render_scale: f32,
    rain_type: RainType,
}

impl Quality {
    fn from_environment() -> Result<Self, String> {
        let mut quality = Self {
            fps: DEFAULT_FPS,
            render_scale: DEFAULT_RENDER_SCALE,
            rain_type: RainType::Modern,
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
        quality.validate()?;
        Ok(quality)
    }

    fn with_arguments(mut self, args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut args = args.into_iter();
        while let Some(argument) = args.next() {
            let (flag, inline_value) = argument
                .split_once('=')
                .map_or((argument.as_str(), None), |(flag, value)| {
                    (flag, Some(value))
                });
            match flag {
                "--type" => {
                    let value = inline_value
                        .map(str::to_owned)
                        .or_else(|| args.next())
                        .ok_or("--type needs modern or classic")?;
                    self.rain_type = match value.as_str() {
                        "modern" => RainType::Modern,
                        "classic" => RainType::Classic,
                        _ => return Err(format!("--type must be modern or classic, got {value}")),
                    };
                }
                "--fps" => {
                    let value = inline_value
                        .map(str::to_owned)
                        .or_else(|| args.next())
                        .ok_or("--fps needs a value from 1 to 60")?;
                    self.fps = value.parse().map_err(|_| "--fps must be 1–60")?;
                }
                "--render-scale" => {
                    let value = inline_value
                        .map(str::to_owned)
                        .or_else(|| args.next())
                        .ok_or("--render-scale needs a value from 0.25 to 1")?;
                    self.render_scale =
                        value.parse().map_err(|_| "--render-scale must be 0.25–1")?;
                }
                _ => return Err(format!("unknown argument: {argument}")),
            }
        }
        self.validate()?;
        Ok(self)
    }

    fn validate(self) -> Result<(), String> {
        if !(1..=60).contains(&self.fps) || !(0.25..=1.0).contains(&self.render_scale) {
            return Err("FPS must be 1–60 and render scale must be 0.25–1".to_owned());
        }
        Ok(())
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
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments
        .iter()
        .any(|argument| argument == "--help" || argument == "-h")
    {
        println!(
            "Usage: matrix-screensaver [--type modern|classic] [--fps 1..60] [--render-scale 0.25..1]\n\nDefaults: --type modern --fps 30 --render-scale 0.5\nEnvironment: MATRIX_FPS and MATRIX_RENDER_SCALE remain supported; CLI values take precedence."
        );
        return;
    }
    let quality =
        match Quality::from_environment().and_then(|quality| quality.with_arguments(arguments)) {
            Ok(quality) => quality,
            Err(error) => {
                eprintln!("matrix-screensaver: {error}. Run --help for usage.");
                std::process::exit(2);
            }
        };
    let event_loop = EventLoop::new().expect("failed to create the window event loop");
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App {
        initial_size: initial_size(),
        quality,
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
    quality: Quality,
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

        match pollster::block_on(Renderer::new(window, self.quality)) {
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
    rain: rain::RainResources,
    post: post::MatrixPost,
    simulation_frame: i32,
    instance: wgpu::Instance,
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    started_at: Instant,
    last_presented_at: Option<Instant>,
    next_frame_at: Instant,
    quality: Quality,
    occluded: bool,
    occluded_at: Option<Instant>,
    benchmark_since: Option<Instant>,
    benchmark_frames: u32,
    benchmark_last_frame: Option<Instant>,
    benchmark_intervals_ms: Vec<f64>,
    profile_queries: Option<wgpu::QuerySet>,
    profile_resolve: Option<wgpu::Buffer>,
    profile_readback: Option<wgpu::Buffer>,
}

impl Renderer {
    async fn new(window: Arc<Window>, quality: Quality) -> Result<Self, String> {
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
        let profile_requested = std::env::var("MATRIX_PROFILE_GPU").is_ok_and(|value| value == "1");
        let profile_features =
            wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
        let profile_supported = adapter.features().contains(profile_features);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Matrix Screensaver device"),
                required_features: if profile_requested && profile_supported {
                    profile_features
                } else {
                    wgpu::Features::empty()
                },
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
        let profile_queries = (profile_requested && profile_supported).then(|| {
            device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("Matrix GPU pass timing"),
                ty: wgpu::QueryType::Timestamp,
                count: 6,
            })
        });
        let profile_readback = profile_queries.as_ref().map(|_| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Matrix GPU timing readback"),
                size: 48,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            })
        });
        let profile_resolve = profile_queries.as_ref().map(|_| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Matrix GPU timing resolve"),
                size: 48,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            })
        });
        if profile_requested && !profile_supported {
            eprintln!("GPU timestamp queries are unsupported on this adapter");
        }
        let rain = rain::RainResources::new_for_type(
            &device,
            &queue,
            MATRIXCODE_MSDF,
            render_width,
            render_height,
            quality.rain_type,
        )?;
        let post = post::MatrixPost::new(
            &device,
            render_width,
            render_height,
            config.format,
            &rain.time_buffer,
            rain.primary_view(),
            rain.high_pass_view(),
            quality.rain_type,
        );

        eprintln!(
            "GPU ready: {} (bundled matrix glyph atlas: {} bytes)",
            adapter.get_info().name,
            MATRIXCODE_MSDF.len()
        );

        Ok(Self {
            rain,
            post,
            simulation_frame: 0,
            instance,
            window,
            surface,
            device,
            queue,
            config,
            started_at: Instant::now(),
            last_presented_at: None,
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
            profile_queries,
            profile_resolve,
            profile_readback,
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
        self.rain
            .resize(&self.device, &self.queue, render_width, render_height);
        self.post.resize(
            &self.device,
            render_width,
            render_height,
            self.rain.primary_view(),
            self.rain.high_pass_view(),
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

        let acquired_at = Instant::now();
        if let Some(last) = self.last_presented_at {
            let gap = acquired_at.duration_since(last);
            if gap > std::time::Duration::from_millis(250) {
                self.started_at += gap;
                if let Some(since) = &mut self.benchmark_since {
                    *since += gap;
                }
                self.benchmark_last_frame = None;
            }
        }

        self.queue.write_buffer(
            &self.rain.time_buffer,
            0,
            bytemuck::bytes_of(&gpu_layout::RainTime {
                seconds: self.started_at.elapsed().as_secs_f32(),
                frames: self.simulation_frame,
                _padding: [0; 2],
            }),
        );
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Matrix frame"),
            });
        let profile_this_frame = self.profile_queries.is_some() && self.simulation_frame % 300 == 0;
        let queries = self.profile_queries.as_ref().filter(|_| profile_this_frame);
        if let Some(queries) = queries {
            encoder.write_timestamp(queries, 0);
        }
        self.rain.encode_simulation(&mut encoder);
        if let Some(queries) = queries {
            encoder.write_timestamp(queries, 1);
        }
        self.rain.encode_render(&mut encoder);
        if let Some(queries) = queries {
            encoder.write_timestamp(queries, 2);
        }
        self.post.encode(&mut encoder, &view, queries);
        if let Some((queries, resolve, readback)) = queries
            .zip(self.profile_resolve.as_ref())
            .zip(self.profile_readback.as_ref())
            .map(|((q, r), b)| (q, r, b))
        {
            encoder.write_timestamp(queries, 5);
            encoder.resolve_query_set(queries, 0..6, resolve, 0);
            encoder.copy_buffer_to_buffer(resolve, 0, readback, 0, 48);
        }
        self.queue.submit([encoder.finish()]);
        if profile_this_frame {
            self.read_gpu_profile();
        }
        self.queue.present(frame);
        self.last_presented_at = Some(Instant::now());
        self.simulation_frame = self.simulation_frame.wrapping_add(1);
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
            self.last_presented_at = None;
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

    fn read_gpu_profile(&self) {
        let Some(readback) = &self.profile_readback else {
            return;
        };
        let (sender, receiver) = std::sync::mpsc::channel();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
        if self
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .is_err()
            || !matches!(receiver.recv(), Ok(Ok(())))
        {
            return;
        }
        if let Ok(mapped) = readback.slice(..).get_mapped_range() {
            let stamps: Vec<u64> = mapped
                .chunks_exact(8)
                .map(bytemuck::pod_read_unaligned)
                .collect();
            let period_ms = self.queue.get_timestamp_period() as f64 / 1_000_000.0;
            let durations: Vec<f64> = stamps
                .windows(2)
                .map(|pair| (pair[1] - pair[0]) as f64 * period_ms)
                .collect();
            eprintln!(
                "GPU pass ms: simulation {:.3}, rain {:.3}, bloom {:.3}, palette {:.3}, presentation {:.3}",
                durations[0], durations[1], durations[2], durations[3], durations[4]
            );
            drop(mapped);
        }
        readback.unmap();
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
    fn command_line_quality_overrides_defaults_and_rejects_bad_values() {
        let defaults = Quality {
            fps: 30,
            render_scale: 0.5,
            rain_type: RainType::Modern,
        };
        let quality = defaults
            .with_arguments(["--fps", "60", "--render-scale=0.75"].map(str::to_owned))
            .unwrap();
        assert_eq!(quality.fps, 60);
        assert_eq!(quality.render_scale, 0.75);
        assert_eq!(quality.rain_type, RainType::Modern);
        assert_eq!(
            defaults
                .with_arguments(["--type=classic".to_owned()])
                .unwrap()
                .rain_type,
            RainType::Classic
        );
        assert!(defaults.with_arguments(["--type=3d".to_owned()]).is_err());
        assert!(
            defaults
                .with_arguments(["--fps", "0"].map(str::to_owned))
                .is_err()
        );
        assert!(
            defaults
                .with_arguments(["--render-scale", "NaN"].map(str::to_owned))
                .is_err()
        );
        assert!(defaults.with_arguments(["--unknown".to_owned()]).is_err());
    }

    #[test]
    fn frame_pacer_holds_a_60_hz_deadline_after_slow_frames() {
        let start = Instant::now();
        let frame_time = Quality {
            fps: 60,
            render_scale: 1.0,
            rain_type: RainType::Modern,
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
}
