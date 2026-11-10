use wgpu::util::DeviceExt;

use crate::reference_layout::{
    ReferenceCell, ReferenceConfig, ReferenceIntroCell, ReferenceScene, ReferenceTime,
};

pub struct ReferenceResources {
    pub _config: wgpu::Buffer,
    pub _time: wgpu::Buffer,
    pub _scene: wgpu::Buffer,
    pub _cells: wgpu::Buffer,
    pub _intro_cells: wgpu::Buffer,
    pub _sampler: wgpu::Sampler,
    pub _glyph: wgpu::Texture,
    pub _glint_msdf: wgpu::Texture,
    pub _base: wgpu::Texture,
    pub _glint: wgpu::Texture,
    pub _render_bind_group: wgpu::BindGroup,
    pub _validation_pipeline: wgpu::RenderPipeline,
}

impl ReferenceResources {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, atlas: &[u8]) -> Result<Self, String> {
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
        let cells = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Reference Cells"),
            size: (80 * 80 * std::mem::size_of::<ReferenceCell>()) as u64,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let intro_cells = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Reference Intro Cells"),
            size: (80 * std::mem::size_of::<ReferenceIntroCell>()) as u64,
            usage: wgpu::BufferUsages::STORAGE,
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
            label: Some("Reference rain WGSL validation"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/reference-rain.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Reference rain pipeline layout validation"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let validation_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Reference rain binding validation"),
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
                targets: &[
                    Some(wgpu::TextureFormat::Rgba8Unorm.into()),
                    Some(wgpu::TextureFormat::Rgba8Unorm.into()),
                ],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        Ok(Self {
            _config: config,
            _time: time,
            _scene: scene,
            _cells: cells,
            _intro_cells: intro_cells,
            _sampler: sampler,
            _glyph: glyph,
            _glint_msdf: glint_msdf,
            _base: base,
            _glint: glint,
            _render_bind_group: render_bind_group,
            _validation_pipeline: validation_pipeline,
        })
    }
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
