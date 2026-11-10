struct Params {
    time: f32,
    aspect: f32,
    width: f32,
    height: f32,
};

struct Cell {
    // brightness, cursor flag, activated flag, perspective depth
    rain: vec4<f32>,
    // glyph index, glyph age, unused, unused
    symbol: vec4<f32>,
};

struct Cells { values: array<Cell> };

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read_write> cells: Cells;

const PI: f32 = 3.14159265359;
const SQRT_2: f32 = 1.4142135623730951;
const SQRT_5: f32 = 2.23606797749979;
const GRID: u32 = 80u;

fn random_float(uv: vec2<f32>) -> f32 {
    return fract(sin(dot(uv, vec2<f32>(12.9898, 78.233)) % PI) * 43758.5453);
}

fn wobble(value: f32) -> f32 {
    return value + 0.3 * sin(SQRT_2 * value) + 0.2 * sin(SQRT_5 * value);
}

// Ported from the reference rain-state shader for the version=3d defaults.
fn rain_brightness(time: f32, glyph_pos: vec2<f32>) -> f32 {
    let offset = random_float(vec2<f32>(glyph_pos.x, 0.0)) * 1000.0;
    let speed = random_float(vec2<f32>(glyph_pos.x + 0.1, 0.0)) * 0.5 + 0.5;
    let rain_time = wobble((glyph_pos.y * 0.01 + offset + time * 0.5 * speed) / 0.3);
    return 1.0 - fract(rain_time);
}

@compute @workgroup_size(64)
fn simulate(@builtin(global_invocation_id) invocation: vec3<u32>) {
    let index = invocation.x;
    if (index >= GRID * GRID) {
        return;
    }

    let column = index % GRID;
    let row = index / GRID;
    let glyph_pos = vec2<f32>(f32(column), f32(row));
    let brightness = rain_brightness(params.time, glyph_pos);
    let below = rain_brightness(params.time, glyph_pos + vec2<f32>(0.0, -1.0));
    let cursor = select(0.0, 1.0, brightness > below);
    let glyph_age = fract(random_float(glyph_pos / f32(GRID) + 0.5) + params.time * 0.03);
    let glyph = floor(57.0 * random_float(glyph_pos / f32(GRID) + floor(params.time * 0.03)));
    let depth = fract(random_float(vec2<f32>(f32(column), 0.0)) + params.time * 0.25);

    cells.values[index].rain = vec4<f32>(brightness, cursor, 1.0, depth);
    cells.values[index].symbol = vec4<f32>(glyph, glyph_age, 0.0, 0.0);
}
