// accumulate shaded values, calculate ray termination and bounces

import super::types::{Vars, Ray, Indirect, Intersection, Material, intersection_none, PI};
import super::sampling::{pcg, pcg_f, sample_hemisphere_cosine, sample_cone};
import super::materials::{brdf_shade, sky_shade, ocean_shade, unknown_shade};

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

    if intersection.dist < 0 || intersection.dist >= 1e37 {
        if ray.direction.y < 0 {
            calc_termination(id, &ray, sky_shade(&ray, intersection, &rng_state));
        } else {
            calc_termination(id, &ray, ocean_shade(&ray, intersection, &rng_state));
        }
    } else if intersection.material >= arrayLength(&materials) {
        calc_termination(id, &ray, unknown_shade(&ray, intersection, &rng_state));
    } else {
        let material = materials[intersection.material];
        if material.kind == 1000 {
            // example non brdf material
            calc_termination(id, &ray, unknown_shade(&ray, intersection, &rng_state));
        } else {
            calc_termination(id, &ray, brdf_shade(&ray, intersection, material, &rng_state));
        }
    }

    ray.rng_state = rng_state;
    rays[id] = ray;
}

// shading utils

const sun_dir = normalize(vec3f(0.4, -0.01, 1.0));
// const sun_dir = normalize(vec3f(0.25, -1.0, -0.25));
const sun_max: f32 = 0.999966;
const sun_solid: f32 = 2.0 * PI * (1.0 - sun_max);

fn calc_termination(id: u32, ray: ptr<function, Ray>, shade_out: ShadeOut) {
    let col = shade_out.col * ray.throughput;
    if col.x == col.x && col.y == col.y && col.z == col.z {
        let coord = vec2u(u32(ray.pixel_x), u32(ray.pixel_y));
        textureStore(output, coord, textureLoad(output, coord) + vec4f(col, 1.0));
    }

    if shade_out.terminate {
        active_rays[id] = 0u;
        atomicAdd(&active_ray_counter, 1); // temp while compaction doesn't exist
    } else {
        atomicAdd(&active_ray_counter, 1); // todo: speed up
    }
}

struct ShadeOut {
    col: vec3f,
    terminate: bool,
}

// ocean waves
// todo: better location?

struct WaveParameters {
    direction: vec2f,
    steepness: f32,
    wavelength: f32,
    speed: f32,
}

const WAVE_COUNT: u32 = 6;
const WAVES = array<WaveParameters, 6>(
    WaveParameters(vec2f(-0.3, -0.65), 0.22, 67.0, 3.5),
    WaveParameters(vec2f(0.85, 0.45), 0.18, 35.0, 2.5),
    WaveParameters(vec2f(-0.65, 0.65), 0.15, 18.0, 1.8),
    WaveParameters(vec2f(0.20, -0.90), 0.12, 8.0, 1.2),
    WaveParameters(vec2f(0.88, -0.30), 0.08, 3.5, 0.7),
    WaveParameters(vec2f(0.75, 0.55), 0.05, 1.8, 0.4)
);

// const WAVE_COUNT: u32 = 6;
// const WAVES = array<WaveParameters, 6>(
//     WaveParameters(vec2f(-0.2, -0.6), 0.2, 67.0, 3.5),
//     WaveParameters(vec2f(0.89, 0.45), 0.25, 35.0, 2.5),
//     WaveParameters(vec2f(-0.7, 0.71), 0.30, 18.0, 1.8),
//     WaveParameters(vec2f(0.15, -0.98), 0.35, 8.0, 1.2),
//     WaveParameters(vec2f(0.92, -0.38), 0.40, 3.5, 0.7),
//     WaveParameters(vec2f(0.8, 0.6), 0.42, 1.8, 0.4),
// );
const WAVE_MAX_AMPLITUDE = 4.0;

fn get_wave_normal(position: vec2f, time: f32) -> vec3f {
    let distort_x = sin(position.y * 0.1 + time * 0.5) * 1.5;
    let distort_y = cos(position.x * 0.1 + time * 0.4) * 1.5;
    let distorted = position + vec2f(distort_x, distort_y);

    var tangent = vec3f(1.0, 0.0, 0.0);
    var binormal = vec3f(0.0, 0.0, 1.0);
    var displaced_pos = vec3f(distorted.x, 0.0, distorted.y);

    let gravity = 9.81;

    for (var i = 0u; i < WAVE_COUNT; i++) {
        let wave = WAVES[i];
        let d = normalize(wave.direction);

        let k = 2.0 * PI / wave.wavelength;
        let c = sqrt(gravity / k) * wave.speed;
        let phase = k * (dot(d, distorted) - c * time);

        let sin_p = sin(phase);
        let base_val = sin_p * 0.5 + 0.5;

        let sharp_derivative = 1.5 * pow(max(base_val, 0.0), 0.5) * 0.5;

        let cos_p = cos(phase) * sharp_derivative;

        let a = wave.steepness / k;

        tangent.x -= d.x * d.x * wave.steepness * sin_p;
        tangent.y += d.x * wave.steepness * cos_p;
        tangent.z -= d.x * d.y * wave.steepness * sin_p;

        binormal.x -= d.x * d.y * wave.steepness * sin_p;
        binormal.y += d.y * wave.steepness * cos_p;
        binormal.z -= d.y * d.y * wave.steepness * sin_p;
    }

    return normalize(cross(binormal, tangent));
}

fn get_wave_height(position: vec2f, time: f32) -> f32 {
    let distort_x = sin(position.y * 0.1 + time * 0.5) * 1.5;
    let distort_y = cos(position.x * 0.1 + time * 0.4) * 1.5;
    let distorted = position + vec2f(distort_x, distort_y);

    var height = 0.0;
    let gravity = 9.81;
    for (var i = 0u; i < WAVE_COUNT; i++) {
        let wave = WAVES[i];
        let d = normalize(wave.direction);

        let k = 2.0 * PI / wave.wavelength;
        let c = sqrt(gravity / k) * wave.speed;
        let phase = k * (dot(d, distorted) - c * time);

        let sharp_factor = pow(sin(phase) * 0.5 + 0.5, 1.5);
        height += (wave.steepness / k) * sharp_factor;
    }

    return height;
}

fn intersect_ocean(ray: ptr<function, Ray>, base_height: f32, time: f32) -> Intersection {
    var result = intersection_none();
    // position stored in normal property
    result.normal = ray.origin;

    let t_top = (base_height - WAVE_MAX_AMPLITUDE - ray.origin.y) / ray.direction.y;
    let t_bottom = (base_height - ray.origin.y) / ray.direction.y;

    let t_start = max(0.0, min(t_top, t_bottom));
    var t_end = max(t_bottom, t_top);

    let MAX_SEARCH_DIST = 1000.0; // todo: decrease back to 400 when approximation complete?
    if (t_end - t_start) > MAX_SEARCH_DIST {
        t_end = t_start + MAX_SEARCH_DIST;
    }

    if t_start > t_end {
        return result;
    }

    const LINEAR_STEPS = 12u;
    var t_current = t_start;
    let step_size = (t_end - t_start) / f32(LINEAR_STEPS);

    var t_prev = t_start;

    for (var i = 0u; i < LINEAR_STEPS; i++) {
        t_current = t_start + f32(i) * step_size;
        let p = ray.origin + ray.direction * t_current;
        let h = base_height - get_wave_height(p.xz, time);

        if p.y > h {
            result.dist = 1.0;
            break;
        }
        t_prev = t_current;
    }

    if result.dist < 0 && t_end >= t_bottom {
        let t_flat = (base_height - ray.origin.y) / ray.direction.y;
        if t_flat > 0.0 {
            result.dist = t_flat;
            result.normal = ray.origin + ray.direction * t_flat;
            return result;
        }
    }

    if result.dist > 0 {
        var t_min = t_prev;
        var t_max = t_current;

        const BINARY_STEPS = 5u;
        for (var j = 0u; j < BINARY_STEPS; j++) {
            let t_mid = mix(t_min, t_max, 0.5);
            let p = ray.origin + ray.direction * t_mid;
            let h = base_height - get_wave_height(p.xz, time);

            if p.y > h {
                t_max = t_mid;
            } else {
                t_min = t_mid;
            }
        }
        result.dist = mix(t_min, t_max, 0.5);
        result.normal = ray.origin + ray.direction * result.dist;
    }

    return result;
}
