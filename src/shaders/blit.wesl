// draw output buffer to screen

import super::types::Vars;

@group(0) @binding(0)
var<uniform> vars: Vars;
@group(0) @binding(1)
var output: texture_storage_2d<rgba32float, read>;

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32
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
    return textureLoad(output, vec2i(in.uv * vars.size));
}