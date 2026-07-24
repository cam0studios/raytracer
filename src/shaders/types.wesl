// shared structs

/*

bvh:      box aabb, left bvh, right bvh, primitives
aabb:     pos vec3, size vec3
ray:      throughput RGB, origin vec3, direction vec3, pixel vec2
hit:      dist int, normal vec3, materiali int
indirect: dims atomic(u32)x3, count atomic(u32)

*/

struct Ray {
    throughput: vec3f,
    origin: vec3f,
    direction: vec3f,
    pixel: vec2u,
}
