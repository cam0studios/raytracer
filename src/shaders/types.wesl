// shared structs

/*

bvh:       box aabbi, left? bvhi, right? bvhi, object? obji
aabb:      pos vec3f, size vec3f
ray:       throughput vec3f, pixelx i32, origin vec3f, pixely i32, direction vec3f
intersect: dist f32, normal vec3f, material materieli
indirect:  count atomic(i32), _ atomic(i32), _ atomic(i32), next count atomic(i32)

*/

struct Vars {
    size: vec2u,
    frame: u32,
    bounce: u32,
}

struct Aabb {
    pos: vec3f,
    size: vec3f,
}

struct Bvh {
    aabb_i: i32,
    left_i: i32,
    right_i: i32,
    object: i32,
}

struct Ray {
    throughput: vec3f,
    pixel_x: i32,
    origin: vec3f,
    pixel_y: i32,
    direction: vec3f,
    _skip: f32,
}

struct Intersection {
    normal: vec3f,
    dist: f32,
    _skip: vec3f,
    material: i32,
}
fn intersection_none() -> Intersection {
    return Intersection(vec3f(0.0, 0.0, 0.0), -1.0, vec3f(0.0, 0.0, 0.0), -1);
}

struct Indirect {
    dim_x: i32,
    dim_y: i32,
    dim_z: i32,
    count: i32,
}