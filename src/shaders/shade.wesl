// accumulate shaded values, calculate ray termination and bounces

import super::types::{Vars, Ray, Indirect};

@group(0) @binding(0)
var<uniform> vars: Vars;
@group(0) @binding(1)
var<storage, read_write> rays: array<Ray>;
@group(0) @binding(2)
var<storage, read_write> active_ray_counter: atomic<i32>;
@group(0) @binding(3)
var<storage, read> active_ray_indirect: Indirect;
@group(0) @binding(4)
var output: texture_storage_2d<rgba32float, read_write>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) idv: vec3<u32>) {
    let fsize = vec2f(vars.size);
    let id = idv.x;

    if id >= u32(active_ray_indirect.count) {
        return;
    }

    let ray = rays[id];
    let coord = vec2u(u32(ray.pixel_x), u32(ray.pixel_y));

    textureStore(output, coord, vec4f(f32(coord.x) / fsize.x, f32(coord.y) / fsize.y, 0.0, 1.0));

    atomicAdd(&active_ray_counter, 1);
}
