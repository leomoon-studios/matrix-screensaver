struct Params {
    time: f32,
    aspect: f32,
    width: f32,
    height: f32,
};

struct Cell {
    rain: vec4<f32>,
    symbol: vec4<f32>,
};

struct Cells { values: array<Cell> };

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) glyph_uv: vec2<f32>,
    @location(1) brightness: f32,
    @location(2) cursor: f32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var glyph_sampler: sampler;
@group(0) @binding(2) var glyph_atlas: texture_2d<f32>;
@group(0) @binding(3) var<storage, read> cells: Cells;

const GRID: u32 = 80u;

fn random_float(uv: vec2<f32>) -> f32 {
    return fract(sin(dot(uv, vec2<f32>(12.9898, 78.233)) % 3.14159265359) * 43758.5453);
}

fn corner(vertex: u32) -> vec2<f32> {
    let corners = array<vec2<f32>, 6>(
        vec2<f32>(-0.5, -0.5), vec2<f32>(0.5, -0.5), vec2<f32>(0.5, 0.5),
        vec2<f32>(-0.5, -0.5), vec2<f32>(0.5, 0.5), vec2<f32>(-0.5, 0.5),
    );
    return corners[vertex % 6u];
}

@vertex
fn vertex_main(@builtin(vertex_index) vertex: u32) -> VertexOut {
    let cell_index = vertex / 6u;
    let column = cell_index % GRID;
    let row = cell_index / GRID;
    let cell = cells.values[cell_index];
    let depth = cell.rain.w;
    // Match the reference's 90° camera after its z=-1 translation. In clip
    // space this is scale = 1/(1-depth), with a small near-plane guard.
    let perspective = 1.0 / max(0.035, 1.0 - depth);
    let base = vec2<f32>(
        (f32(column) + 0.5) / f32(GRID) * 2.0 - 1.0,
        (f32(row) + 0.5) / f32(GRID) * 2.0 - 1.0,
    );
    var position = base + corner(vertex) / f32(GRID);
    position.y += random_float(vec2<f32>(f32(column), 1.0)) / f32(GRID);
    let pos = position * perspective;
    let glyph_x = i32(cell.symbol.x) % 8;
    let glyph_y = 7 - i32(cell.symbol.x) / 8;
    let local = corner(vertex) + vec2<f32>(0.5);

    var output: VertexOut;
    output.position = vec4<f32>(pos.x / params.aspect, pos.y, depth, 1.0);
    output.glyph_uv = (local + vec2<f32>(f32(glyph_x), f32(glyph_y))) / 8.0;
    output.brightness = clamp((cell.rain.x * 1.5 - 0.9) * depth, 0.0, 1.0);
    output.cursor = cell.rain.y;
    return output;
}

fn median(value: vec3<f32>) -> f32 {
    return max(min(value.x, value.y), min(max(value.x, value.y), value.z));
}

fn palette(brightness: f32) -> vec3<f32> {
    // Exact RGB conversion of the version=3d HSL palette stops:
    // 0.0=(.3,.9,0), .2=(.3,.9,.2), .7=(.3,.9,.7), .8=(.3,.9,.8).
    let black = vec3<f32>(0.0);
    let low = vec3<f32>(0.092, 0.38, 0.02);
    let bright = vec3<f32>(0.538, 0.97, 0.43);
    let white = vec3<f32>(0.692, 0.98, 0.62);
    if (brightness < 0.2) { return mix(black, low, brightness / 0.2); }
    if (brightness < 0.7) { return mix(low, bright, (brightness - 0.2) / 0.5); }
    if (brightness < 0.8) { return mix(bright, white, (brightness - 0.7) / 0.1); }
    return white;
}

@fragment
fn fragment_main(input: VertexOut) -> @location(0) vec4<f32> {
    let sdf = median(textureSample(glyph_atlas, glyph_sampler, input.glyph_uv).rgb);
    let glyph = smoothstep(0.46, 0.54, sdf);
    let halo = smoothstep(0.28, 0.52, sdf) - glyph;
    let tracer = input.cursor * input.brightness;
    let base = palette(input.brightness) * glyph * input.brightness;
    let glow = vec3<f32>(0.05, 0.75, 0.02) * halo * input.brightness * 0.55;
    // HSL(.242, 1, .73), with the reference cursor intensity of 2.
    let cursor = min(vec3<f32>(0.756, 1.0, 0.46) * tracer * glyph * 2.0, vec3<f32>(1.0));
    return vec4<f32>(base + glow + cursor, glyph * input.brightness);
}
