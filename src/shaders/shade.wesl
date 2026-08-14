// accumulate shaded values, calculate ray termination and bounces

import super::types::{Vars, Ray, Indirect, Intersection, Material};
import super::sampling::{pcg, sample_hemisphere_cosine, lerp_3f};

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
var<storage, read> materials: array<Material>;

@group(0) @binding(7)
var output: texture_storage_2d<rgba32float, read_write>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id)idv: vec3<u32>) {
    let fsize = vec2f(vars.size);
    let id = idv.x;

    if id >= u32(active_ray_indirect.count) {
        return;
    }
    if active_rays[id] != 1u {
        atomicAdd(&active_ray_counter, 1); // temp while compaction doesn't exist
        return;
    }

    var ray = rays[id];
    let intersection = intersections[id];

    var rng_state = ray.rng_state;

    if intersection.material < 0 {
        calc_termination(id, sky_shade(&ray, intersection, &rng_state));
    } else if intersection.material >= i32(arrayLength(&materials)) {
        calc_termination(id, unknown_shade(&ray, intersection, &rng_state));
    } else {
        let material = materials[intersection.material];
        if material.kind == 0 {
            calc_termination(id, lambertian_shade(&ray, intersection, material, &rng_state));
        } else {
            calc_termination(id, unknown_shade(&ray, intersection, &rng_state));
        }
    }

    ray.rng_state = rng_state;
    rays[id] = ray;
}

// materials

fn sky_shade(ray: ptr<function, Ray>, intersection: Intersection, rng_state: ptr<function, u32>) -> bool {
    var col = vec3f(0.0, 0.0, 0.0);

    if ray.direction.y >= 0 {
        col = vec3f(0.3, 0.3, 0.3);
    } else {
        let sun_dot = dot(ray.direction, normalize(vec3f(0.5, -1.0, 0.0)));

        if sun_dot > 0.95 {
            col = vec3f(7, 7, 6.5);
        }

        col += lerp_3f(vec3f(0.6, 0.8, 0.95), vec3f(0.1, 0.2, 0.7), pow(-ray.direction.y, 0.5));
    }

    out(*ray, col);
    return true;
}

fn lambertian_shade(ray: ptr<function, Ray>, intersection: Intersection, material: Material, rng_state: ptr<function, u32>) -> bool {
    // let dot = -dot(ray.direction, intersection.normal);
    // col *= dot;

    ray.throughput *= material.datav;

    ray.origin += ray.direction * intersection.dist;
    ray.direction = sample_hemisphere_cosine(rng_state, intersection.normal);
    // ray.direction = reflect(ray.direction, intersection.normal);

    ray.origin += ray.direction * 1e-4; // todo: replace with threshold and data stored for next frame

    return false;
}

fn unknown_shade(ray: ptr<function, Ray>, intersection: Intersection, rng_state: ptr<function, u32>) -> bool {
    var col = vec3f(0.0, 0.0, 0.0);
    let pos = floor(ray.origin + ray.direction * intersection.dist);

    if (pos.x + pos.y + pos.z) % 2 == 0 {
        col = vec3f(0.5, 0.0, 0.7);
    }

    out(*ray, col);
    return true;
}

// utils

fn out(ray: Ray, col: vec3f) {
    let out_col = col * ray.throughput;
    if col.x != col.x || col.y != col.y || col.z != col.z {
        return;
    }

    let coord = vec2u(u32(ray.pixel_x), u32(ray.pixel_y));
    textureStore(output, coord, textureLoad(output, coord) + vec4f(out_col, 1.0));
}
fn calc_termination(id: u32, terminate: bool) {
    if terminate {
        active_rays[id] = 0u;
        atomicAdd(&active_ray_counter, 1); // temp while compaction doesn't exist
    } else {
        atomicAdd(&active_ray_counter, 1); // todo: speed up
    }
}
