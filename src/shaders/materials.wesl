import super::types::{Ray, Intersection, Material, intersection_none, PI};
import super::sampling::{pcg, pcg_f, sample_hemisphere_cosine, sample_cone};
import super::shade::{ShadeOut, intersect_ocean, get_wave_normal, get_wave_height, WAVE_MAX_AMPLITUDE, sun_dir, sun_max, sun_solid};

// BRDFS

struct MaterialSample {
    wi: vec3f,
    pdf: f32,
    weight: vec3f,
    is_specular: bool,
}
fn brdf_shade(ray: ptr<function, Ray>, intersection: Intersection, material: Material, rng_state: ptr<function, u32>) -> ShadeOut {
    let wo = -ray.direction;

    var normal = intersection.normal;
    if dot(ray.direction, normal) > 0.0 {
        normal = -normal;
    }

    ray.origin += ray.direction * intersection.dist;

    var sample: MaterialSample;
    if material.kind == 0 {
        sample = lambertian_brdf_sample(wo, normal, lambertian(material), rng_state);
    } else {
        return unknown_shade(ray, intersection, rng_state);
    }
    if sample.pdf <= 0.0 {
        return ShadeOut(vec3f(0.0), true);
    }

    ray.direction = sample.wi;
    ray.inv_dir = 1.0 / ray.direction;

    ray.throughput *= sample.weight;
    ray.origin += ray.direction * 1e-4;

    return ShadeOut(vec3f(0.0), false);
}

// LAMBERTIAN

struct Lambertian {
    albedo: vec3f,
}
fn lambertian(material: Material) -> Lambertian {
    return Lambertian(material.datav);
}
// todo: full mis (two separate brdfs and pdfs, then combine using balance/power heuristic)?
fn lambertian_brdf_eval(wo: vec3f, wi: vec3f, normal: vec3f, material: Lambertian) -> vec3f {
    let cos_theta = dot(wi, normal);

    if cos_theta <= 0.0 || dot(wo, normal) <= 0.0 {
        return vec3f(0.0);
    }

    return material.albedo * (1.0 / PI);
}
fn lambertian_brdf_sample(wo: vec3f, normal: vec3f, material: Lambertian, rng_state: ptr<function, u32>) -> MaterialSample {
    var result: MaterialSample;

    let sample_sun_chance = 0.5;
    let sample_sun = pcg_f(rng_state) < sample_sun_chance; // configurable

    if sample_sun {
        result.wi = sample_cone(rng_state, sun_dir, sun_max);
    } else {
        result.wi = sample_hemisphere_cosine(rng_state, normal);
    }

    let lambertian_pdf = lambertian_pdf(wo, result.wi, normal, material);
    let sun_pdf = sun_pdf(wo, result.wi, normal);
    result.pdf = mix(lambertian_pdf, sun_pdf, sample_sun_chance);

    if result.pdf > 0.0 {
        // non importance sampled: ( (albedo/pi) * cos_theta ) / (cos_theta/pi) = albedo
        // result.weight = material.datav;
        let eval = lambertian_brdf_eval(wo, result.wi, normal, material);
        result.weight = (eval * dot(result.wi, normal)) / result.pdf;
    } else {
        result.weight = vec3f(0.0);
    }

    return result;
}
fn lambertian_pdf(wo: vec3f, wi: vec3f, normal: vec3f, material: Lambertian) -> f32 {
    let cos_theta = dot(wi, normal);

    if cos_theta <= 0.0 || dot(wo, normal) <= 0.0 {
        return 0.0;
    }

    return cos_theta * (1.0 / PI);
}
fn sun_pdf(wo: vec3f, wi: vec3f, normal: vec3f) -> f32 {
    if dot(wi, sun_dir) >= sun_max && dot(wi, normal) > 0.0 {
        return 1.0 / sun_solid;
    }

    return 0.0;
}

// DISNEY

struct Disney {
    base: vec3f,
    subsurface: f32,
    metallic: f32,
    specular: f32,
    specular_tint: f32,
    roughness: f32,
    anisotropic: f32,
    sheen: f32,
    sheen_tint: f32,
    clearcoat: f32,
    clearcoat_gloss: f32,
}
fn disney(material: Material) /* -> Disney */ {}
fn disney_brdf_eval(wo: vec3f, wi: vec3f, normal: vec3f, material: Disney) /* -> vec3f */ {}
fn disney_brdf_sample(wo: vec3f, normal: vec3f, material: Disney, rng_state: ptr<function, u32>) /* -> MaterialSample */ {}
fn disney_brdf_pdf(wo: vec3f, wi: vec3f, normal: vec3f, material: Disney) /* f32 */ {}

// MISC

// todo: ocean as physical intersection?
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
        } else {
            // todo: improve approximation
            ray.direction.y = -abs(ray.direction.y);

            let dot = -ray.direction.y;
            fresnel = f0 + (1 - f0) * pow(1 - dot, 5.0);
            fresnel = 0.5;
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
    let day_sun /*         */ = vec3f(19000.0, 18200.0, 17500.0);
    let day_horizon_far /* */ = vec3f(0.5, 0.7, 1.0);
    let day_horizon_near /**/ = vec3f(0.85, 0.9, 0.9);
    let day_zenith /*      */ = vec3f(0.04, 0.16, 0.45);
    let day_horizon_pow /*       */ = 3.0;
    let day_zenith_pow /*        */ = 0.25;
    let day_glow_g /*            */ = 0.88;
    let day_glow_scale /*        */ = 0.12;
    let day_glow_modifier /*     */ = 1.2;

    let sunset_sun /*         */ = vec3f(4000.0, 2500.0, 250.0);
    let sunset_horizon_far /* */ = vec3f(0.9, 0.2, 0.08);
    let sunset_horizon_near /**/ = vec3f(2.5, 0.8, 0.1);
    let sunset_zenith /*      */ = vec3f(0.1, 0.15, 0.25);
    let sunset_horizon_pow /*       */ = 3.0;
    let sunset_zenith_pow /*        */ = 0.3;
    let sunset_glow_g /*            */ = 0.77;
    let sunset_glow_scale /*        */ = 0.45;
    let sunset_glow_modifier /*     */ = 2.1;

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

    let sky_altitude /*      */ = asin(clamp(-ray.direction.y, 0.0, 1.0)) / (PI / 2.0);
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
        let result = d_num / (2.0 * PI * pow(max(d_den, 0.0001), 1.5));

        let horizon_fade = smoothstep(-0.1, 0.1, sun_height);
        let glow_factor = result * glow_scale * horizon_fade;

        let target_glow_color = horizon_near_col * glow_modifier;

        col += target_glow_color * glow_factor;
    }

    // sun disk
    if sun_height > -0.05 && sun_dot > sun_max {
        let sun_visibility = smoothstep(-0.05, 0.05, sun_height);
        let sun_mask = smoothstep(sun_max, mix(sun_max, 1.0, 0.2), sun_dot);

        col = mix(col, sun_col, sun_visibility * sun_mask);
    }

    return ShadeOut(col, true);
}

fn unknown_shade(ray: ptr<function, Ray>, intersection: Intersection, rng_state: ptr<function, u32>) -> ShadeOut {
    var col = vec3f(0.0, 0.0, 0.0);
    let pos = floor(ray.origin + ray.direction * intersection.dist);

    if (pos.x + pos.y + pos.z) % 2 == 0 {
        col = vec3f(0.5, 0.0, 0.7);
    }

    return ShadeOut(col, true);
}
