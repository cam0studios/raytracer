// shared structs

/*

bvh:       pos vec3f, left? bvhi, size vec3f, right? bvhi, object? obji
ray:       throughput vec3f, pixelx i32, origin vec3f, pixely i32, direction vec3f, rng state u32
intersect: dist f32, normal vec3f, material materieli
indirect:  dimx i32, dimy i32, dimz i32, count i32

*/

struct Vars {
    camera_mat: mat4x4f,
    size: vec2u,
    frame: u32,
    bounce: u32,
    sample_idx: u32,
    exposure: f32,
}

// todo: object count as negative right index
struct Bvh {
    min: vec3f,
    left: i32,
    max: vec3f,
    right: i32,
}

struct Ray {
    throughput: vec3f,
    pixel_x: i32,
    origin: vec3f,
    pixel_y: i32,
    direction: vec3f,
    rng_state: u32,
    inv_dir: vec3f,
    skip: f32,
}

struct Intersection {
    normal: vec3f,
    dist: f32,
    _skip: vec3f,
    material: u32,
}
fn intersection_none() -> Intersection {
    return Intersection(vec3f(0.0, 0.0, 0.0), -1.0, vec3f(0.0, 0.0, 0.0), 0);
}

struct Indirect {
    dim_x: i32,
    dim_y: i32,
    dim_z: i32,
    count: i32,
}

struct Object {
    origin: vec3f,
    kind: u32,
    datav: vec3f,
    material: u32,
    datam: mat2x4f,
}

struct Material {
    datav: vec3f,
    kind: u32,
    datam: mat3x4f,
}

struct Aabb {
    min: vec3f,
    max: vec3f,
}

const PI = 3.14159265;