// intersect rays with scene

import super::types::{Vars, Ray, Indirect, Intersection, intersection_none, Object};

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

// scene
@group(0) @binding(5)
var<storage, read> objects: array<Object>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id)idv: vec3<u32>) {
    let fsize = vec2f(vars.size);
    let id = idv.x;

    if id >= u32(active_ray_indirect.count) || active_rays[id] != 1u {
        return;
    }

    let ray = rays[id];

    var min = intersection_none();
    min.dist = 1e20;

    // todo: bvh
    for (var i = 0u; i < arrayLength(&objects); i++) {
        let obj = objects[i];
        var intersection = intersection_none();
        if obj.kind == 0 {
            // triangle
        } else if obj.kind == 1 {
            intersection = sphere_intersect(ray, obj);
        }
        if intersection.dist > 0 && intersection.dist < min.dist {
            min = intersection;
        }
    }

    intersections[id] = min;
}

fn sphere_intersect(ray: Ray, sphere: Object) -> Intersection {
    let oc = ray.origin - sphere.origin;
    let a = dot(ray.direction, ray.direction);
    let b = 2 * dot(oc, ray.direction);
    let c = dot(oc, oc) - sphere.datav.x * sphere.datav.x;

    let det = b * b - 4 * a * c;

    if det < 0 {
        return intersection_none();
    }
    let sqrt_det = sqrt(det);

    var dist = (-b - sqrt_det) / (2 * a);
    if dist < 0 {
        dist = (-b + sqrt_det) / (2 * a);
    }
    if dist < 0 {
        return intersection_none();
    }

    let normal = normalize(oc + ray.direction * dist);

    return Intersection(normal, dist, vec3f(0.0, 0.0, 0.0), i32(sphere.material));
}
