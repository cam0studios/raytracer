// intersect rays with scene

import super::types::{Vars, Ray, Indirect, Intersection, intersection_none, Bvh, Object, Aabb};

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

@group(0) @binding(5)
var<storage, read> bvh: array<Bvh>;
@group(0) @binding(6)
var<storage, read> objects: array<Object>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id)idv: vec3<u32>) {
    let fsize = vec2f(vars.size);
    let id = idv.x;

    if id >= u32(active_ray_indirect.count) || active_rays[id] != 1u {
        return;
    }

    let ray = rays[id];
    let intersection = traverse_bvh(ray);

    intersections[id] = intersection;
}

// todo: wide bvh?
fn traverse_bvh(ray: Ray) -> Intersection {
    var stack: array<i32, 32>;
    var stack_ptr = 0i;

    stack[stack_ptr] = 0;
    stack_ptr++;

    var closest = intersection_none();
    closest.dist = 1e38;

    var iter = 0u;
    while stack_ptr > 0 && iter < 64u {
        iter++;
        stack_ptr--;
        let node_i = stack[stack_ptr];
        if node_i < 0 || u32(node_i) >= arrayLength(&bvh) {
            continue;
        }
        let node = bvh[u32(node_i)];

        let box_t = aabb_intersect(ray, Aabb(node.min, node.max), closest.dist);
        if box_t < 0.0 || box_t > closest.dist {
            continue;
        }

        if node.left >= 0 {
            if stack_ptr < 31 {
                stack[stack_ptr] = node.right;
                stack_ptr++;
                stack[stack_ptr] = node.left;
                stack_ptr++;
            }
        } else {
            // todo: loop through multiple
            let obj_i = u32(-node.left - 1);
            if u32(obj_i) >= arrayLength(&objects) {
                continue;
            }

            let obj = objects[obj_i];
            if obj.kind == 0 {
                let t = triangle_intersect(ray, obj);
                if t.dist > 0.0 && t.dist < closest.dist {
                    closest = t;
                }
            } else if obj.kind == 1 {
                let t = sphere_intersect(ray, obj);
                if t.dist > 0.0 && t.dist < closest.dist {
                    closest = t;
                }
            }
        }
    }

    return closest;
}

fn aabb_intersect(ray: Ray, aabb: Aabb, t_max: f32) -> f32 {
    let t1 = (aabb.min - ray.origin) * ray.inv_dir;
    let t2 = (aabb.max - ray.origin) * ray.inv_dir;

    let tmin = min(t1, t2);
    let tmax = max(t1, t2);

    let t_enter = max(max(tmin.x, tmin.y), tmin.z);
    let t_leave = min(min(tmax.x, tmax.y), tmax.z);

    if t_enter <= t_leave && t_leave > 0.0 && t_enter < t_max {
        return max(t_enter, 0.0);
    }
    return -1.0;
}

// todo: return f32, calculate normal after?

fn triangle_intersect(ray: Ray, triangle: Object) -> Intersection {
    let epsilon = 1e-6;

    let a = triangle.origin;
    let b = triangle.datav;
    let c = triangle.datam[0].xyz;

    let e1 = b - a;
    let e2 = c - a;

    let ray_cross_e2 = cross(ray.direction, e2);
    let det = dot(e1, ray_cross_e2);

    if det > -epsilon && det < epsilon {
        return intersection_none();
    }

    let inv_det = 1.0 / det;
    let s = ray.origin - a;
    let u = inv_det * dot(s, ray_cross_e2);
    if u < 0.0 || u > 1.0 {
        return intersection_none();
    }

    let s_cross_e1 = cross(s, e1);
    let v = inv_det * dot(ray.direction, s_cross_e1);
    if v < 0.0 || u + v > 1.0 {
        return intersection_none();
    }

    let t = inv_det * dot(e2, s_cross_e1);

    if t <= epsilon {
        return intersection_none();
    }

    let normal = normalize(cross(e1, e2));
    return Intersection(normal, t, vec3f(0.0, 0.0, 0.0), triangle.material);
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

    return Intersection(normal, dist, vec3f(0.0, 0.0, 0.0), sphere.material);
}
