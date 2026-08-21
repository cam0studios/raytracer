// accumulate shaded values, calculate ray termination and bounces

import super::types::{Vars, Ray, Indirect, Intersection, Material};
import super::sampling::{pcg, sample_hemisphere_cosine};

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
        if material.kind == 0 {
            calc_termination(id, &ray, lambertian_shade(&ray, intersection, material, &rng_state));
        } else {
            calc_termination(id, &ray, unknown_shade(&ray, intersection, &rng_state));
        }
    }

    ray.rng_state = rng_state;
    rays[id] = ray;
}

// materials

fn ocean_shade(ray: ptr<function, Ray>, intersection: Intersection, rng_state: ptr<function, u32>) -> ShadeOut {
    let f0 = 0.02;
    let fresnel = f0 + (1 - f0) * pow(1 - ray.direction.y, 5.0);

    // if ray.direction.y > 0.0001 {
    //     let ocean_height = -10;

    //     let t = (-10 - ray.origin.y) / ray.direction.y;
    //     let pos = ray.origin + ray.direction * t;

    //     ray.direction = normalize(ray.direction);
    // }
    ray.direction.y *= -1;

    let absorb_col = vec3f(0.005, 0.015, 0.03);
    let sky_col = sky_shade(ray, intersection, rng_state).col;

    let col = mix(absorb_col, sky_col, fresnel);

    return ShadeOut(col, true);
}

fn sky_shade(ray: ptr<function, Ray>, intersection: Intersection, rng_state: ptr<function, u32>) -> ShadeOut {
    let sun_dir = normalize(vec3f(0.25, -0.02, 1.0));
    // let sun_dir = normalize(vec3f(0.25, -1.0, 0.5));

    let sun_height = -sun_dir.y;
    let day_weight = clamp(sun_height, 0.0, 1.0);

    // config
    let day_sun /*         */ = vec3f(200.0, 190.0, 170.0);
    let day_horizon_far /* */ = vec3f(0.5, 0.7, 1.0);
    let day_horizon_near /**/ = vec3f(0.85, 0.9, 0.9);
    let day_zenith /*      */ = vec3f(0.02, 0.11, 0.38);
    let day_horizon_pow /*       */ = 3.0;
    let day_zenith_pow /*        */ = 0.25;
    let day_glow_g /*            */ = 0.88;
    let day_glow_scale /*        */ = 0.12;
    let day_glow_modifier /*     */ = 1.2;

    let sunset_sun /*         */ = vec3f(150.0, 40.0, 5.0);
    let sunset_horizon_far /* */ = vec3f(0.08, 0.04, 0.08);
    let sunset_horizon_near /**/ = vec3f(2.5, 0.8, 0.1);
    let sunset_zenith /*      */ = vec3f(0.005, 0.01, 0.04);
    let sunset_horizon_pow /*       */ = 3.0;
    let sunset_zenith_pow /*        */ = 0.8;
    let sunset_glow_g /*            */ = 0.60;
    let sunset_glow_scale /*        */ = 0.35;
    let sunset_glow_modifier /*     */ = 2.5;

    let night_horizon /* */ = vec3f(0.001, 0.0015, 0.005);
    let night_zenith /*  */ = vec3f(0.0002, 0.0004, 0.0015);
    let night_horizon_pow /*   */ = 1.0;
    let night_zenith_pow /*    */ = 2.0;

    // sunset fading
    let sun_col /*              */ = mix(sunset_sun, day_sun, day_weight);
    var horizon_near_col /**/ = mix(sunset_horizon_near, day_horizon_near, day_weight);
    var horizon_far_col /* */ = mix(sunset_horizon_far, day_horizon_far, day_weight);
    var zenith_col /*      */ = mix(sunset_zenith, day_zenith, day_weight);
    var horizon_pow /*           */ = mix(sunset_horizon_pow, day_horizon_pow, day_weight);
    var zenith_pow /*            */ = mix(sunset_zenith_pow, day_zenith_pow, day_weight);

    // night fading
    if sun_height < 0.0 {
        let night_weight = clamp(-sun_height / 0.2, 0.0, 1.0);
        let night_weight_fast = clamp(-sun_height / 0.05, 0.0, 1.0);

        horizon_far_col /* */ = mix(horizon_far_col, night_horizon, night_weight);
        horizon_near_col /**/ = mix(horizon_near_col, horizon_far_col, night_weight_fast);
        zenith_col /*      */ = mix(zenith_col, night_zenith, night_weight);
        horizon_pow /*     */ = mix(horizon_pow, night_horizon_pow, night_weight_fast);
        zenith_pow /*      */ = mix(zenith_pow, night_zenith_pow, night_weight_fast);
    }

    // calculations
    let sun_dot = dot(ray.direction, sun_dir);

    let sun_azimuth_weight /**/ = pow(clamp(sun_dot * 0.5 + 0.5, 0.0, 1.0), horizon_pow);
    let base_horizon /**/ = mix(horizon_far_col, horizon_near_col, sun_azimuth_weight);

    let sky_altitude /*      */ = asin(clamp(-ray.direction.y, 0.0, 1.0)) / (3.14159265 / 2.0);
    let altitude_weight /*   */ = pow(sky_altitude, zenith_pow);

    var col = mix(base_horizon, zenith_col, altitude_weight);

    // henyey greenstein glow
    if sun_height > -0.1 {
        let g /*            */ = mix(sunset_glow_g, day_glow_g, day_weight);
        let glow_scale /*   */ = mix(sunset_glow_scale, day_glow_scale, day_weight);
        let glow_modifier /**/ = mix(sunset_glow_modifier, day_glow_modifier, day_weight);

        let g2 = g * g;
        let d_num = 1.0 - g2;
        let d_den = 1.0 + g2 - 2.0 * g * sun_dot;
        let result = d_num / (12.5663706 * pow(max(d_den, 0.0001), 1.5));

        let horizon_fade = smoothstep(-0.1, 0.1, sun_height);
        let glow_factor = result * glow_scale * horizon_fade;

        let target_glow_color = horizon_near_col * glow_modifier;

        col += target_glow_color * glow_factor;
    }

    // sun disk
    if sun_height > -0.05 && sun_dot > 0.998 {
        let sun_visibility = smoothstep(-0.05, 0.05, sun_height);
        let sun_mask = smoothstep(0.998, 0.9985, sun_dot);

        col = mix(col, sun_col, sun_visibility * sun_mask);
    }

    return ShadeOut(col, true);
}

fn lambertian_shade(ray: ptr<function, Ray>, intersection: Intersection, material: Material, rng_state: ptr<function, u32>) -> ShadeOut {
    // let dot = -dot(ray.direction, intersection.normal);
    // col *= dot;

    ray.throughput *= material.datav;

    ray.origin += ray.direction * intersection.dist;

    var normal = intersection.normal;
    if dot(ray.direction, intersection.normal) > 0 {
        normal *= -1;
    }
    ray.direction = sample_hemisphere_cosine(rng_state, normal);
    // ray.direction = reflect(ray.direction, intersection.normal);
    ray.inv_dir = 1.0 / ray.direction;

    ray.origin += ray.direction * 1e-4; // todo: replace with threshold and data stored for next frame

    return ShadeOut(vec3f(0.0), false);
}

fn unknown_shade(ray: ptr<function, Ray>, intersection: Intersection, rng_state: ptr<function, u32>) -> ShadeOut {
    var col = vec3f(0.0, 0.0, 0.0);
    let pos = floor(ray.origin + ray.direction * intersection.dist);

    if (pos.x + pos.y + pos.z) % 2 == 0 {
        col = vec3f(0.5, 0.0, 0.7);
    }

    out(*ray, col);
    return ShadeOut(col, true);
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
fn calc_termination(id: u32, ray: ptr<function, Ray>, shade_out: ShadeOut) {
    if shade_out.terminate {
        active_rays[id] = 0u;
        out(*ray, shade_out.col);
        atomicAdd(&active_ray_counter, 1); // temp while compaction doesn't exist
    } else {
        atomicAdd(&active_ray_counter, 1); // todo: speed up
    }
}

struct ShadeOut {
    col: vec3f,
    terminate: bool,
}
