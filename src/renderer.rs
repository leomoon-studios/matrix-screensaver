use crate::{MATRIXCODE_MSDF, Quality, gpu_layout, post, rain, settings::next_frame_deadline};
use std::{sync::Arc, time::Instant};
use winit::{dpi::PhysicalSize, event_loop::OwnedDisplayHandle, window::Window};

/// GPU simulation and presentation for a surface supplied by a platform host.
/// Dropping this value releases its GPU resources and surface.
pub struct Renderer {
    rain: rain::RainResources,
    post: post::MatrixPost,
    simulation_frame: i32,
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
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

#[derive(Debug, PartialEq, Eq)]
pub enum RenderStatus {
    Presented,
    Skipped,
    SurfaceLost,
}

impl Renderer {
    /// Convenience constructor for the standalone winit host.
    pub async fn new_for_window(
        window: Arc<Window>,
        quality: Quality,
        display: OwnedDisplayHandle,
        shared: Option<&Renderer>,
    ) -> Result<Self, String> {
        let instance = shared.map_or_else(
            || {
                let mut descriptor =
                    wgpu::InstanceDescriptor::new_with_display_handle_from_env(Box::new(display));
                descriptor.backends &= native_backends();
                wgpu::Instance::new(descriptor)
            },
            |renderer| renderer.instance.clone(),
        );
        let surface = instance
            .create_surface(window.clone())
            .map_err(|error| format!("could not create a rendering surface: {error}"))?;
        Self::new_for_surface(instance, surface, window.inner_size(), quality, shared).await
    }

    /// Accepts a surface created by a native host. The host owns the window or view
    /// for at least as long as this renderer and supplies its physical pixel size.
    pub async fn new_for_surface(
        instance: wgpu::Instance,
        surface: wgpu::Surface<'static>,
        size: PhysicalSize<u32>,
        quality: Quality,
        shared: Option<&Renderer>,
    ) -> Result<Self, String> {
        let adapter = if let Some(renderer) = shared {
            renderer.adapter.clone()
        } else {
            instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                    apply_limit_buckets: false,
                })
                .await
                .map_err(|error| format!("no compatible GPU adapter: {error}"))?
        };
        if adapter.get_info().device_type == wgpu::DeviceType::Cpu {
            return Err("only a software graphics adapter was found; this GPU renderer would consume substantial CPU time".to_owned());
        }
        let profile_requested = std::env::var("MATRIX_PROFILE_GPU").is_ok_and(|value| value == "1");
        let profile_features =
            wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
        let profile_supported = adapter.features().contains(profile_features);
        let (device, queue) = if let Some(renderer) = shared {
            (renderer.device.clone(), renderer.queue.clone())
        } else {
            adapter
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
                .map_err(|error| format!("could not create GPU device: {error}"))?
        };

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
            quality.render_scale,
        );

        eprintln!(
            "GPU ready: {} ({:?}; bundled matrix glyph atlas: {} bytes)",
            adapter.get_info().name,
            adapter.get_info().backend,
            MATRIXCODE_MSDF.len()
        );

        Ok(Self {
            rain,
            post,
            simulation_frame: 0,
            instance,
            adapter,
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

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
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

    pub fn render(&mut self) -> Result<RenderStatus, String> {
        if self.occluded || Instant::now() < self.next_frame_at {
            return Ok(RenderStatus::Skipped);
        }
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                drop(frame);
                self.surface.configure(&self.device, &self.config);
                return Ok(RenderStatus::Skipped);
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(RenderStatus::Skipped);
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                return Ok(RenderStatus::SurfaceLost);
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(RenderStatus::Skipped);
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err("GPU surface validation failed".to_owned());
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
        Ok(RenderStatus::Presented)
    }

    pub fn set_occluded(&mut self, occluded: bool) {
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
        }
    }

    pub fn next_frame_at(&self) -> Instant {
        self.next_frame_at
    }

    pub fn is_occluded(&self) -> bool {
        self.occluded
    }

    pub fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }

    pub fn replace_surface(&mut self, surface: wgpu::Surface<'static>) {
        self.surface = surface;
        self.surface.configure(&self.device, &self.config);
    }

    /// The host can call this after the platform signals that its surface was lost.
    pub fn recreate_surface_for_window(&mut self, window: Arc<Window>) -> Result<(), String> {
        let surface = self
            .instance
            .create_surface(window)
            .map_err(|error| format!("could not recreate rendering surface: {error}"))?;
        self.replace_surface(surface);
        Ok(())
    }

    pub fn pause(&mut self) {
        self.set_occluded(true);
    }

    pub fn resume(&mut self) {
        self.set_occluded(false);
    }

    pub fn stop(self) {
        drop(self);
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

pub fn native_backends() -> wgpu::Backends {
    #[cfg(target_os = "windows")]
    {
        wgpu::Backends::DX12
    }
    #[cfg(target_os = "macos")]
    {
        wgpu::Backends::METAL
    }
    #[cfg(target_os = "linux")]
    {
        wgpu::Backends::VULKAN | wgpu::Backends::GL
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        wgpu::Backends::PRIMARY
    }
}
