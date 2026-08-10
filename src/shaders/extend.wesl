// intersect rays with scene

import super::types::{Vars, Ray, Indirect, Intersection};

@group(0) @binding(0)
var<uniform> vars: Vars;
@group(0) @binding(1)
var<storage, read> rays: array<Ray>;
@group(0) @binding(2)
var<storage, read> active_rays: array<u32>;
@group(0) @binding(3)
var<storage, read> active_ray_indirect: Indirect;
@group(0) @binding(4)
var<storage, read_write> intersections: array<Intersection>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id)idv: vec3<u32>) {
    let fsize = vec2f(vars.size);
    let id = idv.x;

    if id >= u32(active_ray_indirect.count) || active_rays[id] != 1u {
        return;
    }

    let ray = rays[id];
    let pos = vec2f(ray.direction.xy);

    if length(pos) > 0.2f {
        intersections[id] = Intersection(vec3f(0.0, 0.0, 0.0), -1.0, vec3f(0.0, 0.0, 0.0), -1);
        return;
    }

    intersections[id] = Intersection(vec3f(0.0, 0.0, 1.0), 1.0, vec3f(0.0, 0.0, 0.0), 0);
}
