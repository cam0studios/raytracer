// camera ray generation

import super::types::{Vars, Ray, Indirect};

// requires readonly_and_readwrite_storage_textures;

@group(0) @binding(0)
var<uniform> vars: Vars;
@group(0) @binding(1)
var<storage, read_write> rays: array<Ray>;
@group(0) @binding(2)
var<storage, read_write> active_ray_counter: atomic<i32>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) idv: vec3<u32>) {
    let fsize = vec2f(vars.size);
    let isize = vec2i(fsize);
    let usize = vec2u(isize);
    // let ray = Ray();
    let coords = vec2i(idv.xy);
    let id: u32 = idv.x + idv.y * usize.x;

    if coords.x >= isize.x || coords.y >= isize.y {
        return;
    }

    var ray: Ray = Ray();
    ray.throughput = vec3f(1.0, 1.0, 1.0);
    ray.pixel_x = coords.x;
    ray.pixel_y = coords.y;
    ray.direction = normalize(vec3f((f32(coords.x) - 0.5) / fsize.y, (f32(coords.y) - 0.5) / fsize.y, 2.0));

    rays[id] = ray;

    if id == 0 {
        atomicStore(&active_ray_counter, isize.x * isize.y);
    }
}