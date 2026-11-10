use wgpu::util::DeviceExt;

use crate::reference_layout::{
    ReferenceCell, ReferenceConfig, ReferenceIntroCell, ReferenceScene, ReferenceTime,
};

pub struct ReferenceResources {
    pub _config: wgpu::Buffer,
    pub _time: wgpu::Buffer,
    scene_buffer: wgpu::Buffer,
    pub _cells: wgpu::Buffer,
    pub _intro_cells: wgpu::Buffer,
    pub _sampler: wgpu::Sampler,
    pub _glyph: wgpu::Texture,
    pub _glint_msdf: wgpu::Texture,
    pub _base: wgpu::Texture,
    pub _glint: wgpu::Texture,
    render_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    primary_texture: wgpu::Texture,
    primary_view: wgpu::TextureView,
    high_pass_texture: wgpu::Texture,
    high_pass_view: wgpu::TextureView,
    pub time_buffer: wgpu::Buffer,
    intro_pipeline: wgpu::ComputePipeline,
    main_pipeline: wgpu::ComputePipeline,
    intro_group: wgpu::BindGroup,
    main_group: wgpu::BindGroup,
}

impl ReferenceResources {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        atlas: &[u8],
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        let config = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Reference Config"),
            contents: bytemuck::bytes_of(&ReferenceConfig::matrix_3d()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let time = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Reference Time"),
            contents: bytemuck::bytes_of(&ReferenceTime {
                seconds: 0.0,
                frames: 0,
                _padding: [0; 2],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let scene = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Reference Scene"),
            size: std::mem::size_of::<ReferenceScene>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let storage_usage = wgpu::BufferUsages::STORAGE
            | if cfg!(test) {
                wgpu::BufferUsages::COPY_SRC
            } else {
                wgpu::BufferUsages::empty()
            };
        let cells = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Reference Cells"),
            size: (80 * 80 * std::mem::size_of::<ReferenceCell>()) as u64,
            usage: storage_usage,
            mapped_at_creation: false,
        });
        let intro_cells = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Reference Intro Cells"),
            size: (80 * std::mem::size_of::<ReferenceIntroCell>()) as u64,
            usage: storage_usage,
            mapped_at_creation: false,
        });
        let mut image = image::load_from_memory(atlas)
            .map_err(|error| format!("cannot decode bundled reference atlas: {error}"))?
            .into_rgba8();
        image::imageops::flip_vertical_in_place(&mut image);
        let glyph = texture(
            device,
            queue,
            "Reference glyph atlas",
            image.width(),
            image.height(),
            image.as_raw(),
        );
        let neutral = [255, 255, 255, 255];
        let glint_msdf = texture(
            device,
            queue,
            "Reference glint MSDF placeholder",
            1,
            1,
            &neutral,
        );
        let base = texture(
            device,
            queue,
            "Reference base texture placeholder",
            1,
            1,
            &neutral,
        );
        let glint = texture(
            device,
            queue,
            "Reference glint texture placeholder",
            1,
            1,
            &neutral,
        );
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Reference linear sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Reference rain render bindings"),
            entries: &[
                buffer_entry(
                    0,
                    wgpu::ShaderStages::VERTEX_FRAGMENT,
                    wgpu::BufferBindingType::Uniform,
                ),
                buffer_entry(
                    1,
                    wgpu::ShaderStages::VERTEX_FRAGMENT,
                    wgpu::BufferBindingType::Uniform,
                ),
                buffer_entry(
                    2,
                    wgpu::ShaderStages::VERTEX_FRAGMENT,
                    wgpu::BufferBindingType::Uniform,
                ),
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                texture_entry(4),
                texture_entry(5),
                texture_entry(6),
                texture_entry(7),
                buffer_entry(
                    8,
                    wgpu::ShaderStages::VERTEX_FRAGMENT,
                    wgpu::BufferBindingType::Storage { read_only: true },
                ),
            ],
        });
        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Reference rain render group"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: config.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: time.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: scene.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(
                        &glyph.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(
                        &glint_msdf.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(
                        &base.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(
                        &glint.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: cells.as_entire_binding(),
                },
            ],
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Reference rain WGSL"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/reference-rain.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Reference rain pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let additive = wgpu::BlendComponent {
            operation: wgpu::BlendOperation::Add,
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
        };
        let target = wgpu::ColorTargetState {
            format: wgpu::TextureFormat::Rgba8Unorm,
            blend: Some(wgpu::BlendState {
                color: additive,
                alpha: additive,
            }),
            write_mask: wgpu::ColorWrites::ALL,
        };
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Reference rain geometry"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vertMain"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fragMain"),
                targets: &[Some(target.clone()), Some(target)],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let intro_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Reference intro state"),
            layout: None,
            module: &module,
            entry_point: Some("computeIntro"),
            compilation_options: Default::default(),
            cache: None,
        });
        let main_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Reference rain and symbol state"),
            layout: None,
            module: &module,
            entry_point: Some("computeMain"),
            compilation_options: Default::default(),
            cache: None,
        });
        let intro_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Reference intro bindings"),
            layout: &intro_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: config.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: time.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: intro_cells.as_entire_binding(),
                },
            ],
        });
        let main_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Reference state bindings"),
            layout: &main_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: config.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: time.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: cells.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: intro_cells.as_entire_binding(),
                },
            ],
        });
        let (primary_texture, primary_view) =
            render_target(device, "Reference rain brightness", width, height);
        let (high_pass_texture, high_pass_view) =
            render_target(device, "Reference rain high pass", width, height);
        queue.write_buffer(
            &scene,
            0,
            bytemuck::bytes_of(&reference_scene(width, height)),
        );
        Ok(Self {
            time_buffer: time.clone(),
            intro_pipeline,
            main_pipeline,
            intro_group,
            main_group,
            _config: config,
            _time: time,
            scene_buffer: scene,
            _cells: cells,
            _intro_cells: intro_cells,
            _sampler: sampler,
            _glyph: glyph,
            _glint_msdf: glint_msdf,
            _base: base,
            _glint: glint,
            render_bind_group,
            render_pipeline,
            primary_texture,
            primary_view,
            high_pass_texture,
            high_pass_view,
        })
    }

    pub fn encode_simulation(&self, encoder: &mut wgpu::CommandEncoder) {
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Reference intro update"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.intro_pipeline);
            pass.set_bind_group(0, &self.intro_group, &[]);
            pass.dispatch_workgroups(80_u32.div_ceil(32), 1, 1);
        }
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Reference state update"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.main_pipeline);
            pass.set_bind_group(0, &self.main_group, &[]);
            pass.dispatch_workgroups(80_u32.div_ceil(32), 80, 1);
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32) {
        queue.write_buffer(
            &self.scene_buffer,
            0,
            bytemuck::bytes_of(&reference_scene(width, height)),
        );
        (self.primary_texture, self.primary_view) =
            render_target(device, "Reference rain brightness", width, height);
        (self.high_pass_texture, self.high_pass_view) =
            render_target(device, "Reference rain high pass", width, height);
    }

    pub fn encode_render(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Reference rain geometry pass"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.primary_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.high_pass_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                }),
            ],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.render_pipeline);
        pass.set_bind_group(0, &self.render_bind_group, &[]);
        pass.draw(0..80 * 80 * 6, 0..1);
    }

    pub fn primary_view(&self) -> &wgpu::TextureView {
        &self.primary_view
    }

    pub fn high_pass_view(&self) -> &wgpu::TextureView {
        &self.high_pass_view
    }

    #[cfg(test)]
    pub fn high_pass_texture(&self) -> &wgpu::Texture {
        &self.high_pass_texture
    }
}

fn reference_scene(width: u32, height: u32) -> ReferenceScene {
    let aspect = width as f32 / height as f32;
    let near = 0.0001_f32;
    let far = 1000.0_f32;
    ReferenceScene {
        screen_size: if aspect > 1.0 {
            [1.0, aspect]
        } else {
            [1.0 / aspect, 1.0]
        },
        _padding: [0.0; 2],
        camera: [
            [1.0 / aspect, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, far / (near - far), -1.0],
            [0.0, 0.0, far * near / (near - far), 0.0],
        ],
        transform: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, -1.0, 1.0],
        ],
    }
}

fn render_target(
    device: &wgpu::Device,
    label: &'static str,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let usage = wgpu::TextureUsages::RENDER_ATTACHMENT
        | wgpu::TextureUsages::TEXTURE_BINDING
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

fn buffer_entry(
    binding: u32,
    visibility: wgpu::ShaderStages,
    ty: wgpu::BufferBindingType,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            multisampled: false,
            view_dimension: wgpu::TextureViewDimension::D2,
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
        },
        count: None,
    }
}

fn texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &'static str,
    width: u32,
    height: u32,
    bytes: &[u8],
) -> wgpu::Texture {
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
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytes,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    texture
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    const CELL_BYTES: u64 = (80 * 80 * std::mem::size_of::<ReferenceCell>()) as u64;
    const INTRO_BYTES: u64 = (80 * std::mem::size_of::<ReferenceIntroCell>()) as u64;

    fn snapshot(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        resources: &ReferenceResources,
        seconds: f32,
        frame: i32,
    ) -> (Vec<ReferenceCell>, Vec<ReferenceIntroCell>) {
        queue.write_buffer(
            &resources.time_buffer,
            0,
            bytemuck::bytes_of(&ReferenceTime {
                seconds,
                frames: frame,
                _padding: [0; 2],
            }),
        );
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Reference state test readback"),
            size: CELL_BYTES + INTRO_BYTES,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Reference state test"),
        });
        resources.encode_simulation(&mut encoder);
        encoder.copy_buffer_to_buffer(&resources._cells, 0, &readback, 0, CELL_BYTES);
        encoder.copy_buffer_to_buffer(
            &resources._intro_cells,
            0,
            &readback,
            CELL_BYTES,
            INTRO_BYTES,
        );
        queue.submit([encoder.finish()]);
        let (sender, receiver) = mpsc::channel();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                sender.send(result).unwrap()
            });
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        receiver.recv().unwrap().unwrap();
        let mapped = readback.slice(..).get_mapped_range().unwrap();
        let cells = mapped[..CELL_BYTES as usize]
            .chunks_exact(std::mem::size_of::<ReferenceCell>())
            .map(bytemuck::pod_read_unaligned)
            .collect();
        let intro = mapped[CELL_BYTES as usize..]
            .chunks_exact(std::mem::size_of::<ReferenceIntroCell>())
            .map(bytemuck::pod_read_unaligned)
            .collect();
        drop(mapped);
        readback.unmap();
        (cells, intro)
    }

    fn rendered_pixels(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        resources: &ReferenceResources,
        width: u32,
        height: u32,
        high_pass: bool,
    ) -> Vec<u8> {
        let bytes_per_row = (width * 4).div_ceil(256) * 256;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Reference geometry test readback"),
            size: u64::from(bytes_per_row * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Reference geometry test"),
        });
        resources.encode_render(&mut encoder);
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: if high_pass {
                    &resources.high_pass_texture
                } else {
                    &resources.primary_texture
                },
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
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
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                sender.send(result).unwrap()
            });
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        receiver.recv().unwrap().unwrap();
        let mapped = readback.slice(..).get_mapped_range().unwrap();
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for row in mapped.chunks_exact(bytes_per_row as usize) {
            pixels.extend_from_slice(&row[..(width * 4) as usize]);
        }
        drop(mapped);
        readback.unmap();
        pixels
    }

    #[test]
    fn reference_state_starts_and_advances_on_gpu() {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .expect("GPU adapter needed for reference state test");
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Reference state test device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        }))
        .unwrap();
        let resources =
            ReferenceResources::new(&device, &queue, crate::MATRIXCODE_MSDF, 640, 480).unwrap();
        let (first, first_intro) = snapshot(&device, &queue, &resources, 0.0, 0);
        assert_eq!(first.len(), 80 * 80);
        assert_eq!(first_intro.len(), 80);
        assert!(
            first
                .iter()
                .all(|cell| cell.raindrop.iter().all(|x| x.is_finite()))
        );
        assert!(first.iter().any(|cell| cell.raindrop[0] > 0.0));
        assert!(first.iter().any(|cell| cell.raindrop[1] > 0.0));

        let (second, second_intro) = snapshot(&device, &queue, &resources, 20.0, 1);
        // version=3d skips the intro; the intro pass must write the reference's
        // fully activated value before the main pass reads it.
        assert!(first_intro.iter().all(|cell| cell.progress[0] == 2.0));
        assert!(second_intro.iter().all(|cell| cell.progress[0] == 2.0));
        assert!(
            second
                .iter()
                .zip(&first)
                .any(|(new, old)| new.raindrop[0] > old.raindrop[0])
        );
        assert!(
            second
                .iter()
                .zip(&first)
                .any(|(new, old)| new.raindrop[0] < old.raindrop[0])
        );
        assert!(
            second
                .iter()
                .zip(&first)
                .any(|(new, old)| new.raindrop[1] != old.raindrop[1])
        );
        assert!(
            second
                .iter()
                .all(|cell| (0.0..=1.0).contains(&cell.raindrop[0]))
        );
        assert!(second.windows(81).any(|pair| {
            pair[0].raindrop[0] > pair[80].raindrop[0]
                && pair[0].raindrop[0] - pair[80].raindrop[0] < 0.2
        }));

        let mut latest = second;
        for frame in 2..40 {
            latest = snapshot(&device, &queue, &resources, 20.0 + frame as f32, frame).0;
        }
        assert!(
            latest
                .iter()
                .zip(&first)
                .any(|(new, old)| new.symbol[0] != old.symbol[0])
        );
    }

    #[test]
    fn reference_geometry_renders_two_targets_and_resizes() {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .unwrap();
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Reference geometry test device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        }))
        .unwrap();
        let mut resources =
            ReferenceResources::new(&device, &queue, crate::MATRIXCODE_MSDF, 128, 128).unwrap();
        snapshot(&device, &queue, &resources, 0.0, 0);
        let primary = rendered_pixels(&device, &queue, &resources, 128, 128, false);
        let high_pass = rendered_pixels(&device, &queue, &resources, 128, 128, true);
        let lit: Vec<_> = primary
            .chunks_exact(4)
            .enumerate()
            .filter(|(_, p)| p[0] > 0 || p[1] > 0)
            .map(|(i, _)| (i % 128, i / 128))
            .collect();
        assert!(lit.len() > 1_000 && lit.len() < 10_000);
        assert!(lit.iter().map(|p| p.0).max().unwrap() > 100);
        assert!(lit.iter().map(|p| p.1).max().unwrap() > 100);
        assert_eq!(primary.len(), 128 * 128 * 4);
        assert!(primary.chunks_exact(4).filter(|pixel| pixel[0] > 0).count() > 100);
        assert!(primary.chunks_exact(4).any(|pixel| pixel[1] > 0));
        assert!(primary.chunks_exact(4).any(|pixel| pixel[0] == 0));
        assert!(high_pass.chunks_exact(4).any(|pixel| pixel[0] > 0));
        assert!(
            high_pass
                .iter()
                .zip(&primary)
                .all(|(high, base)| high <= base)
        );

        // The reference depth is fract(startDepth + seconds * 0.25): four seconds
        // makes a full depth cycle while the stored glyph state remains fixed.
        queue.write_buffer(
            &resources.time_buffer,
            0,
            bytemuck::bytes_of(&ReferenceTime {
                seconds: 4.0,
                frames: 0,
                _padding: [0; 2],
            }),
        );
        let wrapped = rendered_pixels(&device, &queue, &resources, 128, 128, false);
        let changed = wrapped
            .iter()
            .zip(&primary)
            .filter(|(new, old)| new != old)
            .count();
        assert!(
            changed < primary.len() / 20,
            "depth cycle changed {changed} output bytes"
        );

        resources.resize(&device, &queue, 256, 128);
        assert!((reference_scene(256, 128).camera[0][0] - 0.5).abs() < 0.0001);
        assert!((reference_scene(128, 256).camera[0][0] - 2.0).abs() < 0.0001);
        let wide = rendered_pixels(&device, &queue, &resources, 256, 128, false);
        assert_eq!(wide.len(), 256 * 128 * 4);
        assert!(wide.chunks_exact(4).any(|pixel| pixel[0] > 0));
    }
}
