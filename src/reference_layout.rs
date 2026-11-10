//! Host-side layouts for the copied WebGPU reference shaders.
//!
//! These types deliberately mirror the fixed-size storage structures in
//! `reference-rain.wgsl`.

#[repr(C, align(16))]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ReferenceTime {
    pub seconds: f32,
    pub frames: i32,
    pub _padding: [u32; 2],
}

/// Byte-exact 192-byte `Config` uniform from `reference-rain.wgsl`.
/// Grouped scalar fields retain their exact WGSL declaration order.
#[repr(C, align(16))]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ReferenceConfig {
    pub animation_speed: f32,
    pub glyph_sequence_length: f32,
    pub glyph_texture_grid_size: [i32; 2],
    pub glyph_height_to_width: f32,
    pub _pad0: f32,
    pub glyph_transform: [[f32; 2]; 2],
    pub grid_size: [f32; 2],
    pub show_debug_view: i32,
    pub compute: [f32; 5],
    pub cycle_frame_skip: i32,
    pub fall_speed: f32,
    pub has_thunder: i32,
    pub ripple: [f32; 4],
    pub ripple_type: i32,
    pub render: [f32; 6],
    pub flags: [i32; 2],
    pub glyph_vertical_spacing: f32,
    pub glyph_edge_crop: f32,
    pub is_polar: i32,
    pub density: f32,
    pub slant_scale: f32,
    pub _pad1: f32,
    pub slant_vec: [f32; 2],
    pub booleans: [i32; 5],
    pub high_pass_threshold: f32,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ReferenceScene {
    pub screen_size: [f32; 2],
    pub _padding: [f32; 2],
    pub camera: [[f32; 4]; 4],
    pub transform: [[f32; 4]; 4],
}

#[repr(C, align(16))]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ReferenceCell {
    pub raindrop: [f32; 4],
    pub symbol: [f32; 4],
    pub effect: [f32; 4],
}

#[repr(C, align(16))]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ReferenceIntroCell {
    pub progress: [f32; 4],
}

const _: () = assert!(std::mem::size_of::<ReferenceTime>() == 16);
const _: () = assert!(std::mem::size_of::<ReferenceConfig>() == 192);
const _: () = assert!(std::mem::size_of::<ReferenceScene>() == 144);
const _: () = assert!(std::mem::size_of::<ReferenceCell>() == 48);
const _: () = assert!(std::mem::size_of::<ReferenceIntroCell>() == 16);

impl ReferenceConfig {
    pub fn matrix_classic() -> Self {
        let mut config = Self::matrix_3d();
        config.fall_speed = 0.3;
        config.ripple[0] = 0.75;
        config.render[2] = -0.5;
        config.render[3] = 1.1;
        config.booleans[0] = 0; // Flat, single-quad renderer.
        config.booleans[4] = 0; // Enable the staggered intro.
        config
    }

    pub fn matrix_3d() -> Self {
        Self {
            animation_speed: 1.0,
            glyph_sequence_length: 57.0,
            glyph_texture_grid_size: [8, 8],
            glyph_height_to_width: 1.0,
            _pad0: 0.0,
            glyph_transform: [[1.0, 0.0], [0.0, 1.0]],
            grid_size: [80.0, 80.0],
            show_debug_view: 0,
            compute: [0.0, 0.0, 1.0, 0.0, 0.03],
            cycle_frame_skip: 1,
            fall_speed: 0.5,
            has_thunder: 0,
            ripple: [0.3, 30.0, 0.2, 0.2],
            ripple_type: -1,
            render: [4.0, 0.25, -0.9, 1.5, -1.5, 2.5],
            flags: [0, 0],
            glyph_vertical_spacing: 1.0,
            glyph_edge_crop: 0.0,
            is_polar: 0,
            density: 1.0,
            slant_scale: 1.0,
            _pad1: 0.0,
            slant_vec: [1.0, 0.0],
            booleans: [1, 1, 0, 0, 1],
            high_pass_threshold: 0.1,
        }
    }
}
