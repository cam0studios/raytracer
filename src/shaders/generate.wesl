// camera ray generation

import super::types::{Vars, Ray, Indirect};
import super::sampling::ray_seed;

// requires readonly_and_readwrite_storage_textures;

@group(0) @binding(0)
var<uniform> vars: Vars;
@group(0) @binding(1)
var<storage, read_write> rays: array<Ray>;
@group(0) @binding(2)
var<storage, read_write> active_ray_counter: atomic<i32>;
@group(0) @binding(3)
var<storage, read_write> active_rays: array<u32>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id)idv: vec3<u32>) {
    let usize = vars.size;
    let isize = vec2i(vars.size);
    let fsize = vec2f(vars.size);
    // let ray = Ray();
    let coords = vec2i(idv.xy);
    let id: u32 = idv.x + idv.y * usize.x;

    active_rays[id] = 0u;

    if coords.x >= isize.x || coords.y >= isize.y {
        return;
    }

    var ray: Ray = Ray();
    ray.throughput = vec3f(1.0, 1.0, 1.0);
    ray.pixel_x = coords.x;
    ray.pixel_y = coords.y;
    ray.direction = normalize(vec3f((f32(coords.x) - fsize.x / 2) / fsize.y, (f32(coords.y) - fsize.y / 2) / fsize.y, 2.0));
    // ray.direction = normalize(vec3f((f32(coords.x) - fsize.x / 2) / fsize.y, -2.0, (f32(coords.y) - fsize.y / 2) / fsize.y));
    ray.rng_state = ray_seed(u32(coords.x), u32(coords.y), vars.frame, 1);

    rays[id] = ray;
    active_rays[id] = 1u;

    if id == 0 {
        atomicStore(&active_ray_counter, isize.x * isize.y);
    }
}