mod gpu_layout;
mod post;
mod rain;
pub mod renderer;
pub mod settings;

pub use settings::{Quality, RainType};

// The atlas and shaders are embedded in the executable and in host libraries.
pub const MATRIXCODE_MSDF: &[u8] = include_bytes!("../assets/matrixcode_msdf.png");

#[cfg(test)]
mod tests {
    use super::MATRIXCODE_MSDF;

    #[test]
    fn glyph_atlas_is_embedded_png() {
        assert!(MATRIXCODE_MSDF.starts_with(b"\x89PNG\r\n\x1a\n"));
        let atlas = image::load_from_memory(MATRIXCODE_MSDF).unwrap();
        assert!(atlas.width() > 0 && atlas.height() > 0);
    }
}
