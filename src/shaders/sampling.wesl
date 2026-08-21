// random numbers, hemisphere sampling

fn hash(x: u32) -> u32 {
    var h = x;
    h ^= h >> 16u;
    h *= 0x7feb352du;
    h ^= h >> 15u;
    h *= 0x846ca68bu;
    h ^= h >> 16u;
    return h;
}
fn ray_seed(pixel_x: u32, pixel_y: u32, frame: u32, sample_idx: u32) -> u32 {
    var h = 0u;
    h ^= hash(pixel_x ^ (pixel_y * 0x9e3779b9u));
    h ^= hash(frame * 0x85ebca6bu);
    h ^= hash(sample_idx * 0xc2b2ae3du);
    h ^= hash(h + 0x27d4eb2du);
    return select(h, 1u, h == 0u);
}

fn pcg(state: ptr<function, u32>) -> u32 {
    let old_state = *state;
    *state = old_state * 747796405u + 2891336453u;
    let word = ((old_state >> ((old_state >> 28u) + 4u)) ^ old_state) * 277803737u;
    return (word >> 22u) ^ word;
}
fn pcg_f(state: ptr<function, u32>) -> f32 {
    return (f32(pcg(state)) + 0.5) * (1.0 / 4294967296.0);
}

fn pcg_2f(state: ptr<function, u32>) -> vec2f {
    let u = (f32(pcg(state)) + 0.5) * (1.0 / 4294967296.0);
    let v = (f32(pcg(state)) + 0.5) * (1.0 / 4294967296.0);
    return vec2f(u, v);
}

fn local_to_world(dir: vec3f, normal: vec3f) -> vec3f {
    var tangent = vec3f(0.0, 1.0, 0.0);
    if abs(normal.z) < 0.999 {
        tangent = normalize(cross(normal, vec3f(0.0, 0.0, 1.0)));
    } else {
        tangent = normalize(cross(normal, vec3f(1.0, 0.0, 0.0)));
    }
    let bitangent = cross(normal, tangent);

    return normalize(tangent * dir.x + bitangent * dir.y + normal * dir.z);
}

fn sample_local_hemisphere_cosine(state: ptr<function, u32>) -> vec3f {
    let r = pcg_2f(state);

    let phi = 6.28318530718 * r.x;
    let cos_theta = sqrt(1.0 - r.y);
    let sin_theta = sqrt(r.y);

    return vec3f(
        sin_theta * cos(phi),
        sin_theta * sin(phi),
        cos_theta
    );
}
fn sample_hemisphere_cosine(state: ptr<function, u32>, normal: vec3f) -> vec3f {
    let local_dir = sample_local_hemisphere_cosine(state);
    return normalize(local_to_world(local_dir, normal));
}

fn sample_local_cone(max_cos: f32, state: ptr<function, u32>) -> vec3f {
    let r = pcg_2f(state);

    let phi = 6.28318530718 * r.x;
    let cos_theta = (1.0 - r.y) + r.y * max_cos;
    let sin_theta = sqrt(max(0.0, 1.0 - cos_theta * cos_theta));

    return vec3f(
        sin_theta * cos(phi),
        sin_theta * sin(phi),
        cos_theta
    );
}
fn sample_cone(state: ptr<function, u32>, normal: vec3f, max_cos: f32) -> vec3f {
    let local_dir = sample_local_cone(max_cos, state);
    return normalize(local_to_world(local_dir, normal));
}