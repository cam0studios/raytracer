@group(0) @binding(0)
var<storage, read_write> output: array<vec4f>;
@group(0) @binding(1)
var<storage, read_write> objects: array<Object>;
@group(0) @binding(2)
var<storage, read_write> vars: array<vec4f>;
@group(0) @binding(3)
var<storage, read_write> boxes: array<AABB>;
@group(0) @binding(4)
var<storage, read_write> triangles: array<BasicTriangle>;
@group(0) @binding(5)
var<storage, read_write> materials: array<Material>;

struct Ray {
	position: vec3f,
	direction: vec3f,
}

struct HitInfo {
    normal: vec3f,
    distance: f32,
	index: i32,
	dot: f32,
	backface: bool,
	ttests: i32,
	btests: i32
};

struct Object {
	typ: f32,
	material: f32,
	skip1: f32,
	skip2: f32,
	data: vec4f,
	transform: mat4x4f
}

struct AABB {
	min: vec3f,
	index: f32,
	max: vec3f,
	typ: f32,
	childI1: f32,
	childI2: f32,
	objectI: f32,
	triangleI: f32,
	skip3: vec4f
}

struct BasicTriangle {
	points: mat3x4f,
	normals: mat3x4f,
}

struct Material {
	color: array<vec4f, 6>,
	light: mat2x4f,
}

@compute @workgroup_size(64)
fn main(
    @builtin(global_invocation_id)
    global_id : vec3u,

    @builtin(local_invocation_id)
    local_id : vec3u,
) {
    var id: u32 = global_id.x * $SIZE + global_id.y;
	var frame: u32 = u32(vars[0].x * $RAYS_PER_PIXEL + vars[0].y);
	var rngState: u32 = id * $SIZE * 100 + frame;

	var ray: Ray = Ray(
		vars[1].xyz,
		vec3f(0.0, 0.0, 0.0)
	);
	var ior: f32 = 1.0;
	var camDir: vec3f = vars[2].xyz;
	var forward: vec3f = normalize(camDir);
	var right: vec3f = normalize(cross(vec3f(0.0, 1.0, 0.0), forward));
	var up: vec3f = cross(forward, right);
	var defocusPoint: vec2f = randomDir(&rngState).xy * vars[3].y;
	ray.position += defocusPoint.x * right + defocusPoint.y * up;
	var fov: f32 = vars[1].w;
	var aspect_ratio: f32 = f32($SIZE) / f32($SIZE);
	var uncorrected: vec2f = vec2f(
		(f32(global_id.y) + rng(&rngState) - 0.5) / f32($SIZE) * 2.0 - 1.0,
		1.0 - (f32(global_id.x) + rng(&rngState) - 0.5) / f32($SIZE) * 2.0
	);
	uncorrected.x *= aspect_ratio;
	uncorrected *= tan(radians(fov / 2.0)) * 2.0;
	ray.direction = normalize(vec3f(uncorrected - defocusPoint / vars[3].x, -1.0));
	ray.direction = normalize(ray.direction.x * right + ray.direction.y * up + ray.direction.z * forward);

    var hits: array<HitInfo, $BOUNCES> = array<HitInfo, $BOUNCES>();
	var color: vec3f = vec3f(1.0, 1.0, 1.0);
	var light: vec3f = vec3f(0.0, 0.0, 0.0);
	var ttests: i32 = 0;
	var btests: i32 = 0;

    for (var bounce: i32 = 0; bounce < $BOUNCES; bounce += 1) {
        var hit = test(ray);
		ttests += hit.ttests;
		btests += hit.btests;
		hits[bounce] = hit;
		if (hit.index == -1) {
			var envLight: vec3f = vec3f(0.0, 0.0, 0.0);
			if (ray.direction.y < 0.0) {
				envLight += vec3f(0.35, 0.35, 0.35);
			} else {
				var gradient: f32 = pow(ray.direction.y, 0.5);
				envLight += vec3f(0.2, 0.4, 0.7) * gradient + vec3f(1.0, 1.0, 1.0) * (1.0 - gradient);
				var sunDst: f32 = dot(normalize(ray.direction), normalize(vec3f(0.7, 1.0, -0.5)));
				if (sunDst > 0.99) {
					envLight += vec3f(50.0, 48.0, 46.0) * pow((sunDst - 0.99) * 100.0, 8.0);
				}
			}
			light += envLight * color;
			break;
		}

		var obj = objects[hit.index];
		var material = materials[i32(obj.material)];

		ray.position += ray.direction * hit.distance;

		if (material.color[0].w == 0) {
			if (!hit.backface) {
				light += material.light[0].xyz * material.light[0].w * color;
				if (material.color[0].x == 0.0 && material.color[0].y == 0.0 && material.color[0].z == 0.0 && material.color[1][1] == 0.0) {
					break;
				}
				var reflDir: vec3f = reflect(ray.direction, hit.normal);
				var randDir: vec3f = normalize(hit.normal + randomDir(&rngState));
				if (rng(&rngState) < material.color[1][1]) {
					ray.direction = normalize(reflDir * material.color[2][3] + randDir * (1.0 - material.color[2][3]));
					color *= material.color[2].xyz;
				} else {
					ray.direction = normalize(reflDir * material.color[1][0] + randDir * (1.0 - material.color[1][0]));
					color *= material.color[0].xyz;
				}

			}
		} else if (material.color[0].w == 1) {
			light += material.light[0].xyz * material.light[0].w * color;
			color *= material.color[0].xyz;
			if (hit.backface) {
				color *= vec3f(pow(material.color[1].x, hit.distance), pow(material.color[1].y, hit.distance), pow(material.color[1].z, hit.distance));
			}
			var iorRatio: f32 = material.color[1].w / ior;
			if (!hit.backface) {
				iorRatio = 1.0 / iorRatio;
			}
			var cosT = min(dot(-ray.direction, hit.normal), 1.0);
			var sinT = sqrt(1.0 - cosT * cosT);
			if (iorRatio * sinT <= 1.0 && rng(&rngState) > reflectance(cosT, iorRatio)) {
				var refrDir: vec3f = refract(ray.direction, hit.normal, iorRatio);
				ray.direction = normalize(refrDir);
				ior = material.color[1].w;
			} else {
				var reflDir: vec3f = reflect(ray.direction, hit.normal);
				ray.direction = normalize(reflDir);
				color *= material.color[2].xyz;
			}
		}
		ray.position += ray.direction * 0.0001;
    }

	// light = vec3f(f32(ttests) / 100.0, 0.0, f32(btests) / 800.0);
	// var ox: bool = light.x > 1.0;
	// var oz: bool = light.z > 1.0;
	// if (ox) {
	// 	light.x = 1.0;
	// 	light.y = 1.0;
	// 	if (!oz) {
	// 		light.z *= 0.5;
	// 	}
	// }
	// if (oz) {
	// 	light.z = 1.0;
	// 	light.y = 1.0;
	// 	if (!ox) {
	// 		light.x *= 0.5;
	// 	}
	// }

	if (light.x != light.x || light.y != light.y || light.z != light.z) {
		light = vec3f(0.0, 0.0, 0.0);
	}

	output[id] *= f32(frame);
	output[id] += vec4f(light, 1.0);
	output[id] /= f32(frame) + 1;
    output[id].w = 1;
}

fn test(
    ray: Ray,
) -> HitInfo {
	var closest: HitInfo = HitInfo(vec3f(0.0, 0.0, 0.0), 0.0, -1, 0.0, false, 0, 0);
	var ttests: i32 = 0;
	var btests: i32 = 1;
	if (rayAABB(ray, boxes[0]) == -1.0) {
		return closest;
	}
	// indices offset by 1
	var stack: array<i32, 32> = array<i32, 32>();
	stack[0] = 1;
	var stackEnd: i32 = 1;
	for (var lim: i32 = 0; lim < 16384; lim += 1) {
		stackEnd -= 1;
		if (stackEnd < 0) {
			break;
		}
		var box: AABB = boxes[stack[stackEnd] - 1];
		stack[stackEnd] = 0;
		if (box.childI1 != -1 || box.childI2 != -1) {
			btests += 2;
			var hit1: f32 = -1;
			if (box.childI1 != -1) {
				hit1 = rayAABB(ray, boxes[i32(box.childI1)]);
			}
			var hit2: f32 = -1;
			if (box.childI2 != -1) {
				hit2 = rayAABB(ray, boxes[i32(box.childI2)]);
			}
			if (hit1 > hit2) {
				if (hit1 != -1.0 && (hit1 < closest.distance || closest.index == -1)) {
					stack[stackEnd] = i32(box.childI1) + 1;
					stackEnd += 1;
				}
				if (hit2 != -1.0 && (hit2 < closest.distance || closest.index == -1)) {
					stack[stackEnd] = i32(box.childI2) + 1;
					stackEnd += 1;
				}
			} else {
				if (hit2 != -1.0 && (hit2 < closest.distance || closest.index == -1)) {
					stack[stackEnd] = i32(box.childI2) + 1;
					stackEnd += 1;
				}
				if (hit1 != -1.0 && (hit1 < closest.distance || closest.index == -1)) {
					stack[stackEnd] = i32(box.childI1) + 1;
					stackEnd += 1;
				}
			}
		}
		var i: i32 = i32(box.objectI);
		if (box.typ == 0) {
			var obj: Object = objects[i];
			if (obj.typ == 0) {
			} else if (obj.typ == 1) {
			} else if (obj.typ == 2) {
				ttests += 1;
				var hit: HitInfo = raySphere(ray, obj);
				if (hit.distance > 0.0 && (closest.index == -1 || hit.distance < closest.distance)) {
					closest = hit;
					closest.index = i;
				}
			}
		} else if (box.typ == 3) {
			var obj: Object = objects[i];
			var tri: BasicTriangle = triangles[i32(box.triangleI)];
			ttests += 1;
			var hit: HitInfo = rayTriangle(ray, tri, obj);
			if (hit.distance > 0.0 && (closest.index == -1 || hit.distance < closest.distance)) {
				closest = hit;
				closest.index = i;
			}
		}
	}
	var dp: f32 = dot(ray.direction, closest.normal);
	if (dp > 0.0) {
		closest.backface = true;
		closest.normal = -closest.normal;
		closest.dot = -dp;
	} else {
		closest.dot = dp;
	}
	closest.ttests = ttests;
	closest.btests = btests;
	return closest;
}

fn rayAABB(ray: Ray, box: AABB) -> f32 {
	var invDir: vec3f = 1.0 / ray.direction;
	var t0s: vec3f = (box.min - ray.position) * invDir;
	var t1s: vec3f = (box.max - ray.position) * invDir;
	var tsmaller: vec3f = min(t0s, t1s);
	var tbigger: vec3f = max(t0s, t1s);
	var tmin: f32 = max(max(tsmaller.x, tsmaller.y), max(tsmaller.z, 0.0));
	var tmax: f32 = min(tbigger.x, min(tbigger.y, tbigger.z));
	if (tmax >= tmin) {
		return tmin;
	}
	return -1.0;
}
fn rayTriangle(ray: Ray, tri: BasicTriangle, obj: Object) -> HitInfo {
	var cancel: HitInfo = HitInfo(vec3f(0.0, 0.0, 0.0), 0.0, -1, 0.0, false, 0, 0);

	var a: vec3f = (tri.points[0] * obj.transform).xyz;
	var b: vec3f = (tri.points[1] * obj.transform).xyz;
	var c: vec3f = (tri.points[2] * obj.transform).xyz;
	var normalA: vec3f = normalize((tri.normals[0] * obj.transform).xyz);
	var normalB: vec3f = normalize((tri.normals[1] * obj.transform).xyz);
	var normalC: vec3f = normalize((tri.normals[2] * obj.transform).xyz);

	var edgeAB = b - a;
	var edgeAC = c - a;
	var normal = cross(edgeAB, edgeAC);
	var ao = ray.position - a;
	var dao = cross(ao, ray.direction);

	var det = -dot(ray.direction, normal);
	if (abs(det) < 0.0001) {
		return cancel;
	}
	var invDet = 1.0 / det;

	var dst = dot(ao, normal) * invDet;
	if (dst < 0.0001) {
		return cancel;
	}

	var u = dot(edgeAC, dao) * invDet;
	var v = -dot(edgeAB, dao) * invDet;
	var w = 1.0 - u - v;
	if (u < 0.0 || v < 0.0 || w < 0.0) {
		return cancel;
	}

	var hitNormal: vec3f = normalize(normalA * w + normalB * u + normalC * v);

	return HitInfo(
		hitNormal,
		dst,
		0,
		0.0,
		false,
		0,
		0
	);
}
fn raySphere(ray: Ray, obj: Object) -> HitInfo {
	// TODO: full matrix
	// var center: vec3f = obj.position;
	var center: vec3f = obj.transform[3].xyz;
	var radius: f32 = obj.data[0];
	var oc: vec3f = ray.position - center;
	var a: f32 = dot(ray.direction, ray.direction);
	var h: f32 = -dot(oc, ray.direction);
	var c: f32 = dot(oc, oc) - radius * radius;
	var discriminant: f32 = h * h - a * c;

	var d: f32 = 0.0;
	var i: i32 = -1;
	
	if (discriminant > 0.0) {
		var t1: f32 = (h - sqrt(discriminant)) / a;
		var t2: f32 = (h + sqrt(discriminant)) / a;
		
		if (t1 > 0.0) {
			d = t1;
			i = 0;
		} else if (t2 > 0.0) {
			d = t2;
			i = 1;
		} else {
			d = -1.0;
		}
	}
	if (i < 0) {
		return HitInfo(vec3f(0.0, 0.0, 0.0), 0.0, -1, 0.0, false, 0, 0);
	}
	var normal: vec3f = normalize(oc + ray.direction * d);
	return HitInfo(
		normal,
		d,
		i,
		0.0,
		false,
		0,
		0
	);
}

fn refract(ray: vec3f, normal: vec3f, ior: f32) -> vec3f {
	var cosT: f32 = min(dot(-ray, normal), 1.0);
	var outPerp: vec3f = ior * (ray + cosT * normal);
	var outParallel: vec3f = -sqrt(abs(1.0 - dot(outPerp, outPerp))) * normal;
	return normalize(outPerp + outParallel);
}
fn reflectance(cosT: f32, ior: f32) -> f32 {
	var r0: f32 = (1.0 - ior) / (1.0 + ior);
	r0 = r0 * r0;
	return r0 + (1.0 - r0) * pow(1.0 - cosT, 5.0);
}

fn rng(state: ptr<function, u32>) -> f32 {
    *state = *state * 747796405 + 2891336453;
	var result: u32 = ((*state >> ((*state >> 28) + 4)) ^ *state) * 277803737;
	result = (result >> 22) ^ result;
	return f32(result) / 4294967295.0;
}
fn rngNormal(state: ptr<function, u32>) -> f32 {
	var theta: f32 = rng(state) * 6.283185307179586;
	var rho: f32 = sqrt(-2 * log(rng(state)));
	return rho * cos(theta);
}
fn randomDir(state: ptr<function, u32>) -> vec3f {
    var num: vec3f = vec3f(rngNormal(state), rngNormal(state), rngNormal(state));
	return normalize(num);
}
fn randomHemisphere(state: ptr<function, u32>, normal: vec3f) -> vec3f {
	var num: vec3f = randomDir(state);
	if (dot(num, normal) < 0.0) {
		num = -num;
	}
	return num;
}