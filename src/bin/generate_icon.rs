use image::{ImageBuffer, Rgba};
use std::{fs, io::Write};

const SIZE: usize = 256;
const LEVELS: usize = 4;
const BLOOM_STRENGTH: f32 = 0.7;

fn sample(data: &[f32], size: usize, x: usize, y: usize) -> f32 {
    data[y.min(size - 1) * size + x.min(size - 1)]
}

fn linear(data: &[f32], size: usize, x: f32, y: f32) -> f32 {
    let x = x.clamp(0.0, (size - 1) as f32);
    let y = y.clamp(0.0, (size - 1) as f32);
    let ix = x.floor() as usize;
    let iy = y.floor() as usize;
    let tx = x - ix as f32;
    let ty = y - iy as f32;
    let a = sample(data, size, ix, iy) * (1.0 - tx) + sample(data, size, ix + 1, iy) * tx;
    let b = sample(data, size, ix, iy + 1) * (1.0 - tx) + sample(data, size, ix + 1, iy + 1) * tx;
    a * (1.0 - ty) + b * ty
}

fn blur(data: &[f32], size: usize, horizontal: bool) -> Vec<f32> {
    let mut out = vec![0.0; size * size];
    let weights = [1.0, (-0.125_f32).exp()]; // shader Gaussian radius 2
    for y in 0..size {
        for x in 0..size {
            let center = sample(data, size, x, y) * weights[0];
            let (ax, ay) = if horizontal {
                (x.saturating_sub(1), y)
            } else {
                (x, y.saturating_sub(1))
            };
            let (bx, by) = if horizontal {
                ((x + 1).min(size - 1), y)
            } else {
                (x, (y + 1).min(size - 1))
            };
            out[y * size + x] = (center
                + weights[1] * (sample(data, size, ax, ay) + sample(data, size, bx, by)))
                / (weights[0] + 2.0 * weights[1]);
        }
    }
    out
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let atlas = image::open("assets/matrixcode_msdf.png")?.to_rgb8();
    let mut primary = vec![0.0_f32; SIZE * SIZE];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let u = (x as f32 - 40.0) / 176.0;
            let v = (y as f32 - 40.0) / 176.0;
            if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
                continue;
            }
            // One character from the same 8x8 MSDF atlas as the renderer.
            let px = atlas.get_pixel(2 * 64 + (u * 64.0) as u32, 2 * 64 + (v * 64.0) as u32);
            let mut channels = [px[0], px[1], px[2]];
            channels.sort();
            let distance = channels[1] as f32 / 255.0;
            primary[y * SIZE + x] = ((distance - 0.5) * 12.0 + 0.5).clamp(0.0, 1.0);
        }
    }
    let mut bloom = vec![0.0_f32; SIZE * SIZE];
    for level in 0..LEVELS {
        let side = ((SIZE as f32 * 0.4) as usize >> level).max(1);
        let mut down = vec![0.0; side * side];
        for y in 0..side {
            for x in 0..side {
                down[y * side + x] = linear(
                    &primary,
                    SIZE,
                    (x as f32 + 0.5) * SIZE as f32 / side as f32 - 0.5,
                    (y as f32 + 0.5) * SIZE as f32 / side as f32 - 0.5,
                );
            }
        }
        let blurred = blur(&blur(&down, side, true), side, false);
        let weight = (1.5 - level as f32 / LEVELS as f32).powf(1.0 / 3.0) * BLOOM_STRENGTH;
        for y in 0..SIZE {
            for x in 0..SIZE {
                bloom[y * SIZE + x] += linear(
                    &blurred,
                    side,
                    (x as f32 + 0.5) * side as f32 / SIZE as f32 - 0.5,
                    (y as f32 + 0.5) * side as f32 / SIZE as f32 - 0.5,
                ) * weight;
            }
        }
    }
    let mut output = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(SIZE as u32, SIZE as u32);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let index = y * SIZE + x;
            let light = ((primary[index] + bloom[index]) * 0.8).clamp(0.0, 0.8);
            // Green palette hue 0.3, saturation 0.9, as used by the renderer.
            let chroma = (1.0 - (2.0 * light - 1.0).abs()) * 0.9;
            let h = 0.3_f32 * 6.0;
            let second = chroma * (1.0 - ((h % 2.0) - 1.0).abs());
            let base = light - chroma / 2.0;
            let rgb = [
                (base + second).clamp(0.0, 1.0),
                (base + chroma).clamp(0.0, 1.0),
                base.clamp(0.0, 1.0),
            ];
            let edge = x.min(y).min(SIZE - 1 - x).min(SIZE - 1 - y) as f32;
            let corner = ((x.min(SIZE - 1 - x) as f32 - 27.0).min(0.0).powi(2)
                + (y.min(SIZE - 1 - y) as f32 - 27.0).min(0.0).powi(2))
            .sqrt();
            let alpha = if edge > 27.0 {
                255
            } else {
                ((27.5 - corner).clamp(0.0, 1.0) * 255.0) as u8
            };
            output.put_pixel(
                x as u32,
                y as u32,
                Rgba([
                    (rgb[0] * 255.0) as u8,
                    (rgb[1] * 255.0) as u8,
                    (rgb[2] * 255.0) as u8,
                    alpha,
                ]),
            );
        }
    }
    fs::create_dir_all("assets")?;
    output.save("assets/matrix-screensaver.png")?;
    let png = fs::read("assets/matrix-screensaver.png")?;
    let mut ico = fs::File::create("assets/matrix-screensaver.ico")?;
    ico.write_all(&[0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0])?;
    ico.write_all(&(png.len() as u32).to_le_bytes())?;
    ico.write_all(&22_u32.to_le_bytes())?;
    ico.write_all(&png)?;
    Ok(())
}
