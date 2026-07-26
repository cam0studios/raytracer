// shared structs

/*

bvh:       box aabbi, left bvhi, right bvhi
aabb:      pos vec3f, size vec3f
ray:       throughput vec3f, origin vec3f, direction vec3f, pixel vec2u
hit:       dist f32, normal vec3f, material materieli
indirect:  count atomic(u32), _ atomic(u32), _ atomic(u32), next count atomic(u32)

*/

struct Ray {
    throughput: vec3f,
    origin: vec3f,
    direction: vec3f,
    pixel: vec2u,
}

struct Vars {
    size: vec2f,
}
