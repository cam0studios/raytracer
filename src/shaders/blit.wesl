// draw output buffer to screen, tone mapping

import super::types::Vars;

@group(0) @binding(0)
var<uniform> vars: Vars;
@group(0) @binding(1)
var output: texture_storage_2d<rgba32float, read>;

struct VertexOutput {
    @builtin(position)
    position: vec4f,
    @location(0)
    uv: vec2f}

@vertex
fn vs_main(
    @builtin(vertex_index)vertex_index: u32
) -> VertexOutput {
    var out: VertexOutput;

    let x = f32((vertex_index & 1u) << 1u);
    let y = f32((vertex_index & 2u));

    out.uv = vec2f(x, y);
    out.position = vec4f(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let inv_frame = 1 / f32(vars.frame);
    var col = textureLoad(output, vec2i(in.position.xy)).xyz * inv_frame;

    col = tone_map_aces(col);
    // col = tone_map_reinhard(col);
    // col = tone_map_exposure(col, 0.5);

    return vec4f(col, 1.0);
}

fn tone_map_aces(linear: vec3f) -> vec3f {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;

    let num = linear * (a * linear + vec3f(b));
    let den = linear * (c * linear + vec3f(d)) + vec3f(e);

    return clamp(num / den, vec3f(0.0f), vec3f(1.0f));
}
fn tone_map_reinhard(linear: vec3f) -> vec3f {
    return clamp(linear / (linear + vec3f(1.0)), vec3f(0.0), vec3f(1.0));
}
fn tone_map_exposure(linear: vec3f, exposure: f32) -> vec3f {
    return clamp(vec3f(1.0) - exp(-linear * exposure), vec3f(0.0), vec3f(1.0));
}