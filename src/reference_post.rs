//! Native copies of the reference WebGPU bloom, palette, and end passes.

use wgpu::util::DeviceExt;

const LEVELS: usize = 4;
const BLOOM_SIZE: f32 = 0.4;
const BLOOM_STRENGTH: f32 = 0.7;

struct BloomLevel {
    width: u32,
    height: u32,
    _horizontal: wgpu::Texture,
    _vertical: wgpu::Texture,
    horizontal_group: wgpu::BindGroup,
    vertical_group: wgpu::BindGroup,
}

struct Targets {
    levels: Vec<BloomLevel>,
    _bloom: wgpu::Texture,
    _palette: wgpu::Texture,
    combine_group: wgpu::BindGroup,
    palette_group: wgpu::BindGroup,
    end_group: wgpu::BindGroup,
}

pub struct ReferencePost {
    blur_pipeline: wgpu::ComputePipeline,
    combine_pipeline: wgpu::ComputePipeline,
    palette_pipeline: wgpu::ComputePipeline,
    end_pipeline: wgpu::RenderPipeline,
    horizontal_config: wgpu::Buffer,
    vertical_config: wgpu::Buffer,
    combine_config: wgpu::Buffer,
    palette_config: wgpu::Buffer,
    palette_colors: wgpu::Buffer,
    linear_sampler: wgpu::Sampler,
    nearest_sampler: wgpu::Sampler,
    time_buffer: wgpu::Buffer,
    targets: Option<Targets>,
    width: u32,
    height: u32,
}

impl ReferencePost {
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        surface_format: wgpu::TextureFormat,
        time_buffer: &wgpu::Buffer,
        primary: &wgpu::TextureView,
        high_pass: &wgpu::TextureView,
    ) -> Self {
        let shader = |label: &'static str, source: &'static str| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            })
        };
        let blur = shader(
            "Reference bloom blur",
            include_str!("shaders/reference-bloom-blur.wgsl"),
        );
        let combine = shader(
            "Reference bloom combine",
            include_str!("shaders/reference-bloom-combine.wgsl"),
        );
        let palette = shader(
            "Reference palette",
            include_str!("shaders/reference-palette.wgsl"),
        );
        let end = shader(
            "Reference final presentation",
            include_str!("shaders/reference-end.wgsl"),
        );
        let compute = |label, module: &wgpu::ShaderModule| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: None,
                module,
                entry_point: Some("computeMain"),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let blur_pipeline = compute("Reference bloom blur pipeline", &blur);
        let combine_pipeline = compute("Reference bloom combine pipeline", &combine);
        let palette_pipeline = compute("Reference palette pipeline", &palette);
        let end_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Reference final presentation pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &end,
                entry_point: Some("vertMain"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &end,
                entry_point: Some("fragMain"),
                targets: &[Some(surface_format.into())],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let uniform = |label, bytes: &[u8]| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytes,
                usage: wgpu::BufferUsages::UNIFORM,
            })
        };
        // WGSL's vec2 starts at byte 8 after the bloom radius.
        let horizontal_config = uniform(
            "Reference horizontal blur settings",
            bytemuck::cast_slice(&[2.0_f32, 0.0, 1.0, 0.0]),
        );
        let vertical_config = uniform(
            "Reference vertical blur settings",
            bytemuck::cast_slice(&[2.0_f32, 0.0, 0.0, 1.0]),
        );
        let combine_config = uniform(
            "Reference bloom settings",
            bytemuck::cast_slice(&[LEVELS as f32, BLOOM_STRENGTH]),
        );
        let palette_config = uniform(
            "Reference palette settings",
            bytemuck::cast_slice(&palette_config_data()),
        );
        let palette_colors = uniform(
            "Reference 512-color palette",
            bytemuck::cast_slice(&palette_data()),
        );
        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Reference post linear sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let nearest_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Reference final nearest sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let mut post = Self {
            blur_pipeline,
            combine_pipeline,
            palette_pipeline,
            end_pipeline,
            horizontal_config,
            vertical_config,
            combine_config,
            palette_config,
            palette_colors,
            linear_sampler,
            nearest_sampler,
            time_buffer: time_buffer.clone(),
            targets: None,
            width,
            height,
        };
        post.resize(device, width, height, primary, high_pass);
        post
    }

    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        primary: &wgpu::TextureView,
        high_pass: &wgpu::TextureView,
    ) {
        self.width = width;
        self.height = height;
        let scaled_width = ((width as f32 * BLOOM_SIZE).floor() as u32).max(1);
        let scaled_height = ((height as f32 * BLOOM_SIZE).floor() as u32).max(1);
        let sizes: Vec<_> = (0..LEVELS)
            .map(|index| {
                (
                    (scaled_width >> index).max(1),
                    (scaled_height >> index).max(1),
                )
            })
            .collect();
        let horizontal: Vec<_> = sizes
            .iter()
            .map(|&(w, h)| target(device, "Reference horizontal bloom", w, h))
            .collect();
        let vertical: Vec<_> = sizes
            .iter()
            .map(|&(w, h)| target(device, "Reference vertical bloom", w, h))
            .collect();
        let mut levels: Vec<BloomLevel> = Vec::with_capacity(LEVELS);
        for (index, ((horizontal_texture, horizontal_view), (vertical_texture, vertical_view))) in
            horizontal.into_iter().zip(vertical).enumerate()
        {
            let input_view = if index == 0 {
                high_pass.clone()
            } else {
                levels[index - 1]
                    ._horizontal
                    .create_view(&Default::default())
            };
            let horizontal_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Reference horizontal bloom bindings"),
                layout: &self.blur_pipeline.get_bind_group_layout(0),
                entries: &[
                    entry(0, self.horizontal_config.as_entire_binding()),
                    entry(1, wgpu::BindingResource::Sampler(&self.linear_sampler)),
                    entry(2, wgpu::BindingResource::TextureView(&input_view)),
                    entry(3, wgpu::BindingResource::TextureView(&horizontal_view)),
                ],
            });
            let vertical_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Reference vertical bloom bindings"),
                layout: &self.blur_pipeline.get_bind_group_layout(0),
                entries: &[
                    entry(0, self.vertical_config.as_entire_binding()),
                    entry(1, wgpu::BindingResource::Sampler(&self.linear_sampler)),
                    entry(2, wgpu::BindingResource::TextureView(&horizontal_view)),
                    entry(3, wgpu::BindingResource::TextureView(&vertical_view)),
                ],
            });
            levels.push(BloomLevel {
                width: sizes[index].0,
                height: sizes[index].1,
                _horizontal: horizontal_texture,
                _vertical: vertical_texture,
                horizontal_group,
                vertical_group,
            });
        }
        let vertical_views: Vec<_> = levels
            .iter()
            .map(|level| level._vertical.create_view(&Default::default()))
            .collect();
        let (bloom, bloom_view) = target(
            device,
            "Reference combined bloom",
            scaled_width,
            scaled_height,
        );
        let combine_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Reference bloom combine bindings"),
            layout: &self.combine_pipeline.get_bind_group_layout(0),
            entries: &[
                entry(0, self.combine_config.as_entire_binding()),
                entry(1, wgpu::BindingResource::Sampler(&self.linear_sampler)),
                entry(2, wgpu::BindingResource::TextureView(&vertical_views[0])),
                entry(3, wgpu::BindingResource::TextureView(&vertical_views[1])),
                entry(4, wgpu::BindingResource::TextureView(&vertical_views[2])),
                entry(5, wgpu::BindingResource::TextureView(&vertical_views[3])),
                entry(6, wgpu::BindingResource::TextureView(&bloom_view)),
            ],
        });
        let (palette, palette_view) = target(device, "Reference palette output", width, height);
        let palette_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Reference palette bindings"),
            layout: &self.palette_pipeline.get_bind_group_layout(0),
            entries: &[
                entry(0, self.palette_config.as_entire_binding()),
                entry(1, self.palette_colors.as_entire_binding()),
                entry(2, self.time_buffer.as_entire_binding()),
                entry(3, wgpu::BindingResource::Sampler(&self.linear_sampler)),
                entry(4, wgpu::BindingResource::TextureView(primary)),
                entry(5, wgpu::BindingResource::TextureView(&bloom_view)),
                entry(6, wgpu::BindingResource::TextureView(&palette_view)),
            ],
        });
        let end_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Reference final bindings"),
            layout: &self.end_pipeline.get_bind_group_layout(0),
            entries: &[
                entry(0, wgpu::BindingResource::Sampler(&self.nearest_sampler)),
                entry(1, wgpu::BindingResource::TextureView(&palette_view)),
            ],
        });
        self.targets = Some(Targets {
            levels,
            _bloom: bloom,
            _palette: palette,
            combine_group,
            palette_group,
            end_group,
        });
    }

    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder, surface: &wgpu::TextureView) {
        let targets = self
            .targets
            .as_ref()
            .expect("reference post targets were not built");
        for level in &targets.levels {
            for group in [&level.horizontal_group, &level.vertical_group] {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Reference bloom blur pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.blur_pipeline);
                pass.set_bind_group(0, group, &[]);
                pass.dispatch_workgroups(level.width.div_ceil(32), level.height, 1);
            }
        }
        let scaled_width = ((self.width as f32 * BLOOM_SIZE).floor() as u32).max(1);
        let scaled_height = ((self.height as f32 * BLOOM_SIZE).floor() as u32).max(1);
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Reference bloom combine pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.combine_pipeline);
            pass.set_bind_group(0, &targets.combine_group, &[]);
            pass.dispatch_workgroups(scaled_width.div_ceil(32), scaled_height, 1);
        }
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Reference palette pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.palette_pipeline);
            pass.set_bind_group(0, &targets.palette_group, &[]);
            pass.dispatch_workgroups(self.width.div_ceil(32), self.height, 1);
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Reference final presentation"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.end_pipeline);
        pass.set_bind_group(0, &targets.end_group, &[]);
        pass.draw(0..6, 0..1);
    }
}

fn entry(binding: u32, resource: wgpu::BindingResource<'_>) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry { binding, resource }
}

fn target(
    device: &wgpu::Device,
    label: &'static str,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let usage = wgpu::TextureUsages::TEXTURE_BINDING
        | wgpu::TextureUsages::STORAGE_BINDING
        | if cfg!(test) {
            wgpu::TextureUsages::COPY_SRC
        } else {
            wgpu::TextureUsages::empty()
        };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    (texture, view)
}

fn hsl(hue: f32, saturation: f32, lightness: f32) -> [f32; 3] {
    let a = saturation * lightness.min(1.0 - lightness);
    let channel = |n: f32| {
        let k = (n + hue * 12.0) % 12.0;
        lightness - a * (-1.0_f32).max((k - 3.0).min((9.0 - k).min(1.0)))
    };
    [channel(0.0), channel(8.0), channel(4.0)]
}

fn palette_data() -> [f32; 512 * 4] {
    let stops = [(0.0, 0.0), (0.2, 0.2), (0.7, 0.7), (0.8, 0.8)];
    let mut output = [0.0; 512 * 4];
    for index in 0..512 {
        let left = stops
            .iter()
            .rposition(|&(at, _)| at * 511.0 <= index as f32)
            .unwrap_or(0);
        let right = (left + 1).min(stops.len() - 1);
        let (left_at, left_light) = stops[left];
        let (right_at, right_light) = stops[right];
        let left_index = (left_at * 511.0_f32).floor();
        let right_index = (right_at * 511.0_f32).floor();
        let ratio = if right_index > left_index {
            ((index as f32 - left_index) / (right_index - left_index)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let a = hsl(0.3, 0.9, left_light);
        let b = hsl(0.3, 0.9, right_light);
        for channel in 0..3 {
            output[index * 4 + channel] = a[channel] * (1.0 - ratio) + b[channel] * ratio;
        }
    }
    output
}

fn palette_config_data() -> [f32; 20] {
    let cursor = hsl(0.242, 1.0, 0.73);
    let mut data = [0.0; 20];
    data[0] = 0.05;
    data[8..11].copy_from_slice(&cursor);
    data[12..15].copy_from_slice(&[1.0, 1.0, 1.0]);
    data[15] = 2.0;
    data[16] = 1.0;
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference_resources::ReferenceResources;
    use std::sync::mpsc;

    fn read_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        width: u32,
        height: u32,
    ) -> Vec<u8> {
        let bytes_per_row = (width * 4).div_ceil(256) * 256;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Reference post test readback"),
            size: u64::from(bytes_per_row * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Reference post copy"),
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);
        let (sender, receiver) = mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                sender.send(result).unwrap()
            });
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        receiver.recv().unwrap().unwrap();
        let mapped = buffer.slice(..).get_mapped_range().unwrap();
        let mut output = Vec::with_capacity((width * height * 4) as usize);
        for row in mapped.chunks_exact(bytes_per_row as usize) {
            output.extend_from_slice(&row[..(width * 4) as usize]);
        }
        drop(mapped);
        buffer.unmap();
        output
    }

    #[test]
    fn palette_matches_reference_stops() {
        let colors = palette_data();
        assert_eq!(&colors[0..3], &[0.0; 3]);
        for (index, lightness) in [(102, 0.2), (357, 0.7), (408, 0.8), (511, 0.8)] {
            let expected = hsl(0.3, 0.9, lightness);
            for channel in 0..3 {
                assert!((colors[index * 4 + channel] - expected[channel]).abs() < 0.001);
            }
        }
        assert_eq!(colors[511 * 4 + 3], 0.0);
    }

    #[test]
    fn reference_post_blooms_colors_and_presents() {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .unwrap();
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Reference post test device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        }))
        .unwrap();
        let resources =
            ReferenceResources::new(&device, &queue, crate::MATRIXCODE_MSDF, 128, 128).unwrap();
        let post = ReferencePost::new(
            &device,
            128,
            128,
            wgpu::TextureFormat::Rgba8Unorm,
            &resources.time_buffer,
            resources.primary_view(),
            resources.high_pass_view(),
        );
        let final_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Reference post test presentation"),
            size: wgpu::Extent3d {
                width: 128,
                height: 128,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = final_texture.create_view(&Default::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Reference post test frame"),
        });
        resources.encode_simulation(&mut encoder);
        resources.encode_render(&mut encoder);
        post.encode(&mut encoder, &view);
        queue.submit([encoder.finish()]);
        let targets = post.targets.as_ref().unwrap();
        let bloom_width = ((128.0_f32 * BLOOM_SIZE).floor() as u32).max(1);
        let bloom = read_texture(&device, &queue, &targets._bloom, bloom_width, bloom_width);
        let high_pass = read_texture(&device, &queue, resources.high_pass_texture(), 128, 128);
        let palette = read_texture(&device, &queue, &targets._palette, 128, 128);
        let final_pixels = read_texture(&device, &queue, &final_texture, 128, 128);
        assert!(bloom.chunks_exact(4).filter(|pixel| pixel[0] > 0).count() > 1_500);
        assert!(high_pass.chunks_exact(4).any(|pixel| pixel[0] > 0));
        assert!(
            palette
                .chunks_exact(4)
                .any(|pixel| pixel[1] > pixel[0] && pixel[1] > 0)
        );
        assert!(
            palette
                .chunks_exact(4)
                .any(|pixel| pixel[0] > 0 && pixel[1] > 0)
        );
        assert_eq!(palette, final_pixels);
    }

    /// Run with MATRIX_CAPTURE_SIZE=1280x720, MATRIX_CAPTURE_FRAME=121, and
    /// MATRIX_CAPTURE_FILE=/tmp/native.png to make a deterministic comparison.
    #[test]
    #[ignore]
    fn capture_native_reference_frame() {
        let size = std::env::var("MATRIX_CAPTURE_SIZE").unwrap();
        let (width, height) = size.split_once('x').unwrap();
        let (width, height): (u32, u32) = (width.parse().unwrap(), height.parse().unwrap());
        let frames: i32 = std::env::var("MATRIX_CAPTURE_FRAME")
            .unwrap()
            .parse()
            .unwrap();
        let file = std::env::var("MATRIX_CAPTURE_FILE").unwrap();
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .unwrap();
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Native comparison capture"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        }))
        .unwrap();
        let resources =
            ReferenceResources::new(&device, &queue, crate::MATRIXCODE_MSDF, width, height)
                .unwrap();
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let post = ReferencePost::new(
            &device,
            width,
            height,
            format,
            &resources.time_buffer,
            resources.primary_view(),
            resources.high_pass_view(),
        );
        let output = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Native comparison output"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = output.create_view(&Default::default());
        for frame in 0..frames {
            queue.write_buffer(
                &resources.time_buffer,
                0,
                bytemuck::bytes_of(&crate::reference_layout::ReferenceTime {
                    seconds: frame as f32 / 60.0,
                    frames: frame,
                    _padding: [0; 2],
                }),
            );
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Native comparison frame"),
            });
            resources.encode_simulation(&mut encoder);
            if frame == frames - 1 {
                resources.encode_render(&mut encoder);
                post.encode(&mut encoder, &view);
            }
            queue.submit([encoder.finish()]);
        }
        let pixels = read_texture(&device, &queue, &output, width, height);
        image::save_buffer(file, &pixels, width, height, image::ColorType::Rgba8).unwrap();
        assert!(pixels.chunks_exact(4).any(|pixel| pixel[1] > 0));
    }
}
