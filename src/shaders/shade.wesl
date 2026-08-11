// accumulate shaded values, calculate ray termination and bounces

import super::types::{Vars, Ray, Indirect, Intersection};

@group(0) @binding(0)
var<uniform> vars: Vars;
@group(0) @binding(1)
var<storage, read_write> rays: array<Ray>;
@group(0) @binding(2)
var<storage, read_write> active_rays: array<u32>;
@group(0) @binding(3)
var<storage, read_write> active_ray_counter: atomic<i32>;
@group(0) @binding(4)
var<storage, read> active_ray_indirect: Indirect;
@group(0) @binding(5)
var<storage, read> intersections: array<Intersection>;
@group(0) @binding(6)
var output: texture_storage_2d<rgba32float, read_write>;
// materials

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id)idv: vec3<u32>) {
    let fsize = vec2f(vars.size);
    let id = idv.x;

    if id >= u32(active_ray_indirect.count) || active_rays[id] != 1u {
        return;
    }

    var ray = rays[id];
    let intersection = intersections[id];

    if intersection.material < 0 {
        calc_termination(id, sky_shade(&ray, intersection));
    } else if intersection.material == 0 {
        calc_termination(id, lambertian_shade(&ray, intersection));
    }

    rays[id] = ray;
}

// materials

fn sky_shade(ray: ptr<function, Ray>, intersection: Intersection) -> bool {
    var col = vec3f(0.05, 0.05, 0.05);

    if ray.direction.y < 0 {
        col = vec3f(0.5, 0.5, 0.5);
    }

    out(*ray, col);
    return true;
}

fn lambertian_shade(ray: ptr<function, Ray>, intersection: Intersection) -> bool {
    let dot = -dot(ray.direction, intersection.normal);
    var col = vec3f(0.0, 0.0, 1.0);

    if (vars.frame / 60) % 2 == 1 {
        col.x = 1;
    }

    col *= dot;
    ray.throughput *= col;

    ray.origin = vec3f(100.0, 0.0, 0.0);
    ray.direction = normalize(vec3f(0.0, -1.0, 1.0));

    return false;
}

// utils

fn out(ray: Ray, col: vec3f) {
    let coord = vec2u(u32(ray.pixel_x), u32(ray.pixel_y));
    textureStore(output, coord, textureLoad(output, coord) + vec4f(col * ray.throughput, 1.0));
}
fn calc_termination(id: u32, terminate: bool) {
    if terminate {
        active_rays[id] = 0u;
        atomicAdd(&active_ray_counter, 1); // temp while compaction doesn't exist
    } else {
        atomicAdd(&active_ray_counter, 1); // todo: speed up
    }
}