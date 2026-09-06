// camera ray generation

import super::types::{Vars, Ray, Indirect};
import super::sampling::{ray_seed, pcg_2f};

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
    let usize = vec2u(vars.size);
    let isize = vec2i(usize);
    let fsize = vec2f(usize);
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

    var rng_state = ray_seed(u32(coords.x), u32(coords.y), vars.frame, vars.sample_idx);

    ray.origin = (vec4f(0.0, 0.0, 0.0, 1.0) * vars.camera_mat).xyz;

    let aa_offset = pcg_2f(&rng_state);
    var pixel_dir = vec3f((f32(coords.x) + aa_offset.x - fsize.x / 2) / -fsize.y, (f32(coords.y) + aa_offset.y - fsize.y / 2) / fsize.y, 1.5);
    pixel_dir = (vec4f(pixel_dir, 0.0) * vars.camera_mat).xyz;

    ray.direction = normalize(pixel_dir);
    ray.inv_dir = 1.0 / normalize(ray.direction);
    ray.rng_state = rng_state;

    rays[id] = ray;
    active_rays[id] = 1u;

    if id == 0 {
        atomicStore(&active_ray_counter, isize.x * isize.y);
    }
}