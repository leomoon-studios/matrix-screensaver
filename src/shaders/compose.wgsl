struct Params {
    time: f32,
    aspect: f32,
    width: f32,
    height: f32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var scene_sampler: sampler;
@group(0) @binding(2) var scene: texture_2d<f32>;

struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vertex_main(@builtin(vertex_index) index: u32) -> Output {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0),
    );
    var output: Output;
    output.position = vec4<f32>(positions[index], 0.0, 1.0);
    output.uv = positions[index] * 0.5 + vec2<f32>(0.5);
    return output;
}

@fragment
fn fragment_main(input: Output) -> @location(0) vec4<f32> {
    let pixel = vec2<f32>(1.0 / params.width, 1.0 / params.height);
    let source = textureSample(scene, scene_sampler, input.uv).rgb;
    // A compact five-level approximation of the reference high-pass blur.
    var bloom = vec3<f32>(0.0);
    for (var radius = 1.0; radius <= 8.0; radius *= 2.0) {
        bloom += max(textureSample(scene, scene_sampler, input.uv + vec2<f32>(radius, 0.0) * pixel).rgb - 0.1, vec3<f32>(0.0));
        bloom += max(textureSample(scene, scene_sampler, input.uv - vec2<f32>(radius, 0.0) * pixel).rgb - 0.1, vec3<f32>(0.0));
        bloom += max(textureSample(scene, scene_sampler, input.uv + vec2<f32>(0.0, radius) * pixel).rgb - 0.1, vec3<f32>(0.0));
        bloom += max(textureSample(scene, scene_sampler, input.uv - vec2<f32>(0.0, radius) * pixel).rgb - 0.1, vec3<f32>(0.0));
    }
    return vec4<f32>(source + bloom * (0.7 / 16.0), 1.0);
}
