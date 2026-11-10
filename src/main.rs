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
const FRAME_TIME: std::time::Duration = std::time::Duration::from_nanos(16_666_667);

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
            WindowEvent::RedrawRequested => renderer.render(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(renderer) = &self.renderer {
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
}

impl Renderer {
    async fn new(window: Arc<Window>) -> Result<Self, String> {
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
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        let reference_resources = reference_resources::ReferenceResources::new(
            &device,
            &queue,
            MATRIXCODE_MSDF,
            config.width,
            config.height,
        )?;
        let reference_post = reference_post::ReferencePost::new(
            &device,
            config.width,
            config.height,
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
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        self.reference_resources.resize(
            &self.device,
            &self.queue,
            self.config.width,
            self.config.height,
        );
        self.reference_post.resize(
            &self.device,
            self.config.width,
            self.config.height,
            self.reference_resources.primary_view(),
            self.reference_resources.high_pass_view(),
        );
    }

    fn render(&mut self, event_loop: &ActiveEventLoop) {
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
        event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + FRAME_TIME));
    }
}
