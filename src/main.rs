// entry point

fn main() {
    println!("Hello, world!");
}

/*

STRUCTURES:
    bvh  (C+G): box aabb, left bvh, right bvh, primitives
    aabb (C+G): pos vec3, size vec3
    ray  (G):   throughput RGB, origin vec3, direction vec3
    hit  (G):   dist int, normal vec3, materiali int

BUFFERS:
    bvh/scene
    materials
    extend+material indirect buffers
    active counters a+b
    material counters: lambertian, dielectric, emissive, miss
    rays
    active ray indices a+b
    intersections
    lambertian indices
    dielectric indices
    emissive indices
    miss indices
    output image

FOR EVERY FRAME:
    camera data  --- generate.wesl -->  ray buffer + active indices+counter
    CPU: write extend indirect buffer

    FOR EVERY BOUNCE:
        ray buffer, scene  >---------------------------------- extend.wesl --------->  intersection buffer
        CPU: clear material counters
        intersection buffer  >-------------------------------- sort.wesl ----------->  material indices+counters
        material indirect buffer, material counters  >-------- prep_indirect.wesl -->  material indirect buffer
        lambertian indices+counter, ray buffer, materials  >-- lambertian.wesl ----->  new active indices buffer
        dielectric indices+counter, ray buffer, materials  >-- dielectric.wesl ----->  new active indices buffer
        emissive indices+counter, ray buffer, materials  >---- emissive.wesl ------->  output image
        miss indices+counter, ray buffer  >------------------- miss.wesl ----------->  output image
        extend indirect buffer, active counter  >------------- prep_indirect.wesl -->  extend indirect buffer
        CPU: swap old and new active indices buffers and counters, clear used one

NOTES:
  - use atomicAdd to get next available index for buffer pushing
  - double buffered active indices, swapping buffers every bounce
  - add atomics first in workgroups, then one write per group
  - dispatch steps with thread count equal to active counter
  - set active counter buffers as indirect to use as dispatch dimensions
  - 2 separate entry points in prep indirect

TO DO LATER:
  - Russian roulette in lambertian/dielectric
  - Shadow rays

*/
