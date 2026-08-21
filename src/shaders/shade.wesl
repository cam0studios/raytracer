// accumulate shaded values, calculate ray termination and bounces

import super::types::{Vars, Ray, Indirect, Intersection, Material, intersection_none};
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

// const sun_dir = normalize(vec3f(0.25, -0.01, 1.0));
const sun_dir = normalize(vec3f(0.25, -1.0, -0.25));

fn ocean_shade(ray: ptr<function, Ray>, intersection: Intersection, rng_state: ptr<function, u32>) -> ShadeOut {
    let f0 = 0.02;
    var fresnel = f0;

    let sun_height = -sun_dir.y;
    let night_weight = clamp(-sun_height / 0.2, 0.0, 1.0);
    var absorb_col = mix(vec3f(0.005, 0.015, 0.03), vec3f(0.0), night_weight);

    if ray.direction.y > 0.0001 {
        let ocean_height = 20.0;
        let time = 0.0;

        // position stored in normal property
        let ocean_hit = intersect_ocean(ray, ocean_height, time);

        if ocean_hit.dist > 0.0 {
            var normal = get_wave_normal(ocean_hit.normal.xz, 0.0);
            normal.y = -abs(normal.y);

            let view_dir_flat = normalize(ray.direction.xz);
            let view_tangent_flat = vec2f(-view_dir_flat.y, view_dir_flat.x);
            let slope_along_view = dot(normal.xz, view_dir_flat);
            let slope_cross_view = dot(normal.xz, view_tangent_flat);

            let distance_scale = 1.0 - 1.0 / (ocean_hit.dist * 0.0004 + 1.0);

            let filtered_cross = mix(slope_cross_view, 0.0, distance_scale);
            let filtered_along = mix(slope_along_view, slope_along_view * 0.4, distance_scale);

            normal.x = view_dir_flat.x * filtered_along + view_tangent_flat.x * filtered_cross;
            normal.z = view_dir_flat.y * filtered_along + view_tangent_flat.y * filtered_cross;
            normal = normalize(normal);

            let local_height = get_wave_height(ocean_hit.normal.xz, time);

            let height_factor = clamp(local_height / WAVE_MAX_AMPLITUDE, 0.0, 1.0);
            absorb_col = mix(absorb_col * 0.25, absorb_col, pow(height_factor, 2.0));

            let dot = max(dot(-ray.direction, normal), 0.0);
            let exponent = mix(2.0, 5.0, height_factor);
            fresnel = f0 + (1 - f0) * pow(1 - dot, exponent);

            ray.direction = reflect(ray.direction, normal);

            if ray.direction.y > -0.0001 {
                ray.direction.y = -0.0001;
                ray.direction = normalize(ray.direction);
            }

            if height_factor >= 1 {
                // return ShadeOut(vec3f(1.0, 0.0, 0.0), true);
            }
        }
    }

    let sky_col = sky_shade(ray, intersection, rng_state).col;

    let col = mix(absorb_col, sky_col, fresnel);

    return ShadeOut(col, true);
}

fn sky_shade(ray: ptr<function, Ray>, intersection: Intersection, rng_state: ptr<function, u32>) -> ShadeOut {
    let sun_height = -sun_dir.y;
    let day_weight = pow(smoothstep(0.0, 1.0, sun_height), 0.9);

    // config
    let day_sun /*         */ = vec3f(38000.0, 36500.0, 35000.0);
    let day_horizon_far /* */ = vec3f(0.5, 0.7, 1.0);
    let day_horizon_near /**/ = vec3f(0.85, 0.9, 0.9);
    let day_zenith /*      */ = vec3f(0.04, 0.16, 0.45);
    let day_horizon_pow /*       */ = 3.0;
    let day_zenith_pow /*        */ = 0.25;
    let day_glow_g /*            */ = 0.88;
    let day_glow_scale /*        */ = 0.12;
    let day_glow_modifier /*     */ = 1.2;

    let sunset_sun /*         */ = vec3f(8000.0, 5000.0, 500.0);
    let sunset_horizon_far /* */ = vec3f(0.9, 0.2, 0.08);
    let sunset_horizon_near /**/ = vec3f(2.5, 0.8, 0.1);
    let sunset_zenith /*      */ = vec3f(0.1, 0.15, 0.25);
    let sunset_horizon_pow /*       */ = 3.0;
    let sunset_zenith_pow /*        */ = 0.7;
    let sunset_glow_g /*            */ = 0.70;
    let sunset_glow_scale /*        */ = 0.45;
    let sunset_glow_modifier /*     */ = 2.8;

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
    if sun_height > -0.05 && sun_dot > 0.999966 {
        let sun_visibility = smoothstep(-0.05, 0.05, sun_height);
        let sun_mask = smoothstep(0.999966, 0.99997, sun_dot);

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

// shading utils

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

// ocean waves

struct WaveParameters {
    direction: vec2f,
    steepness: f32,
    wavelength: f32,
    speed: f32,
}

// const WAVE_COUNT: u32 = 6;
// const WAVES = array<WaveParameters, 6>(
//     WaveParameters(vec2f(-0.3, -0.65), 0.22, 67.0, 3.5),
//     WaveParameters(vec2f(0.85, 0.45), 0.18, 35.0, 2.5),
//     WaveParameters(vec2f(-0.65, 0.65), 0.15, 18.0, 1.8),
//     WaveParameters(vec2f(0.20, -0.90), 0.12, 8.0, 1.2),
//     WaveParameters(vec2f(0.88, -0.30), 0.08, 3.5, 0.7),
//     WaveParameters(vec2f(0.75, 0.55), 0.05, 1.8, 0.4)
// );

const WAVE_COUNT: u32 = 6;
const WAVES = array<WaveParameters, 6>(
    WaveParameters(vec2f(-0.2, -0.6), 0.2, 67.0, 3.5),
    WaveParameters(vec2f(0.89, 0.45), 0.25, 35.0, 2.5),
    WaveParameters(vec2f(-0.7, 0.71), 0.30, 18.0, 1.8),
    WaveParameters(vec2f(0.15, -0.98), 0.35, 8.0, 1.2),
    WaveParameters(vec2f(0.92, -0.38), 0.40, 3.5, 0.7),
    WaveParameters(vec2f(0.8, 0.6), 0.42, 1.8, 0.4),
);
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

        let k = 2.0 * 3.14159265 / wave.wavelength;
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

        let k = 2.0 * 3.14159265 / wave.wavelength;
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

    let MAX_SEARCH_DIST = 400.0;
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
