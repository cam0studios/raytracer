// @ts-ignore-next-line
import rawShader from "../public/shader.wgsl";
import rawScene from "./scene.json";
import rawMaterials from "./materials.json";
import rawModels from "./models.json";
import rawTextures from "./textures.json";

type vec2 = [number, number];
type vec3 = [number, number, number];
type vec4 = [number, number, number, number];
type mat2x4 = [vec4, vec4];
type mat3x2 = [vec2, vec2, vec2];
type mat3x3 = [vec3, vec3, vec3];
type mat4x4 = [vec4, vec4, vec4, vec4];
function matrixMultiply(mat1: mat4x4, mat2: mat4x4): mat4x4 {
	const result: mat4x4 = [
		[0, 0, 0, 0],
		[0, 0, 0, 0],
		[0, 0, 0, 0],
		[0, 0, 0, 0],
	];
	for (let i = 0; i < 4; i++) {
		for (let j = 0; j < 4; j++) {
			result[i][j] =
				mat1[i][0] * mat2[0][j] +
				mat1[i][1] * mat2[1][j] +
				mat1[i][2] * mat2[2][j] +
				mat1[i][3] * mat2[3][j];
		}
	}
	return result;
}
function vectorMatrixMultiply(vec: vec4, mat: mat4x4): vec4 {
	const result: vec4 = [0, 0, 0, 0];
	for (let i = 0; i < 4; i++) {
		result[i] =
			vec[0] * mat[i][0] +
			vec[1] * mat[i][1] +
			vec[2] * mat[i][2] +
			vec[3] * mat[i][3];
	}
	return result;
}

interface BasicObject {
	type: string;
	position: vec3;
	rotation: vec3;
	scale: vec3;
	transform: mat4x4;
	material: string;
	materialI: number;
	index: number;
}
interface Mesh extends BasicObject {
	type: "mesh";
	model: string;
	modelI: number;
}
interface Sphere extends BasicObject {
	type: "sphere";
	radius: number;
}
type SceneObject = Mesh | Sphere;
const scene = rawScene as SceneObject[];

interface Texture {
	id: string;
	index: number;
	source?: string;
	lerpMode?: "nearest" | "linear";
	data: {
		width: number;
		height: number;
		pixels: vec4[];
	};
}
const textures = rawTextures as Texture[];
function getTexture(id: string) {
	return textures.find(t => t.id === id);
}
function numToTexture(data: number | string | undefined) {
	if (typeof data === "string") {
		let name = data.split("|")[0].trim();
		let channelStr = data.split("|")[1]?.trim() || "0";
		let channel = parseInt(channelStr);
		if (isNaN(channel)) {
			if (channelStr === "r" || channelStr === "red") channel = 0;
			else if (channelStr === "g" || channelStr === "green") channel = 1;
			else if (channelStr === "b" || channelStr === "blue") channel = 2;
			else if (channelStr === "a" || channelStr === "alpha") channel = 3;
			else if (channelStr === "x") channel = 0;
			else if (channelStr === "y") channel = 1;
			else if (channelStr === "z") channel = 2;
			else if (channelStr === "w") channel = 3;
		}
		let i = getTexture(name)?.index;
		if (i === undefined) i = -1;
		return -i - 1 - (channel + 1) / 8;
	}
	return data || 0;
}
function vec3ToTexture(data: vec3 | string | undefined) {
	if (typeof data === "string") {
		let i = getTexture(data)?.index;
		if (i === undefined) i = -1;
		return [-i - 1, 0, 0];
	}
	return data || [0, 0, 0];
}

interface BasicMaterial {
	type: "basic" | string;
	id: string;
	emission?: vec4 | string;
}
interface Diffuse extends BasicMaterial {
	type: "diffuse";
	color: vec3 | string;
	smoothness: number | string;
	coating?: {
		color: vec3 | string;
		smoothness: number | string;
		strength: number | string;
	};
}
interface Dielectric extends BasicMaterial {
	type: "dielectric";
	color: vec3 | string;
	tint: vec3 | string;
	reflect: vec3 | string;
	smoothness: number | string;
	ior: number | string;
}
type Material = BasicMaterial | Diffuse | Dielectric;
const materials = rawMaterials as Material[];
function materialToRaw(i: number) {
	const material = materials[i];
	let light = [...(material.emission || [0, 0, 0, 0]), 0, 0, 0, 0];
	if (material.type == "basic") {
		return [
			...new Array(24).fill(0),
			...light
		]
	} else if (material.type == "diffuse") {
		const diffuseMaterial = material as Diffuse;
		return [
			...vec3ToTexture(diffuseMaterial.color), 0,
			numToTexture(diffuseMaterial.smoothness), numToTexture(diffuseMaterial.coating?.strength), 0, 0,
			...vec3ToTexture(diffuseMaterial.coating?.color), numToTexture(diffuseMaterial.coating?.smoothness),
			0, 0, 0, 0,
			0, 0, 0, 0,
			0, 0, 0, 0,
			...light
		];
	} else if (material.type == "dielectric") {
		const dielectricMaterial = material as Dielectric;
		return [
			...vec3ToTexture(dielectricMaterial.color), 1,
			...vec3ToTexture(dielectricMaterial.tint), numToTexture(dielectricMaterial.ior),
			...vec3ToTexture(dielectricMaterial.reflect), 0,
			0, 0, 0, 0,
			0, 0, 0, 0,
			0, 0, 0, 0,
			...light
		];
	} else {
		return [...new Array(32).fill(0)];
	}
}

interface Model {
	id: string;
	triangles: { points: mat3x3, uvs?: mat3x2, normals?: mat3x3, index: number }[];
}
const models = rawModels as Model[];

let animationFrameId: number | null = null;

const queryParams = new URLSearchParams(window.location.search)

if (queryParams.has("reload-in")) {
	const reloadIn = parseInt(queryParams.get("reload-in") || "0");
	if (reloadIn > 0) {
		setTimeout(() => {
			window.location.reload();
		}, reloadIn * 1000);
	}
}

async function init() {
	console.log("Initializing WebGPU...");

	if (!navigator.gpu) {
		throw Error("WebGPU not supported.");
	}

	const adapter = await navigator.gpu.requestAdapter();
	if (!adapter) throw Error("Couldn't request WebGPU adapter.");
	
	const device = await adapter.requestDevice();
	if (!device) throw Error("Couldn't request WebGPU device.");
	
	console.log("WebGPU initialized.");

	device.pushErrorScope("validation");

	const SIZE = 256;
	const RAYS_PER_PIXEL = 5;
	const OUTPUT_LEN = SIZE * SIZE * 4;
	const OUTPUT_SIZE = OUTPUT_LEN * 4;
	const VARS_LEN = 16;
	const VARS_SIZE = VARS_LEN * 4;
	const CLEAR_FRAME = false;
	const BOUNCES = 10;
	const MAX_DEPTH = 100;
	const SCENE_TESTS_PER_AXIS = 32;
	const MESH_TESTS_PER_AXIS = 32;
	let objs: number[] = [];
	let boxNums: number[] = [];
	let triangles: number[] = [];
	let materialNums: number[] = [];
	let textureNums: number[] = [];

	var cam = {
		position: [0, 0, 25],
		rotation: [0, 0],
		fov: 35,
		focusDistance: 1,
		defocusStrength: 0,
		direction: [0, 0, 1],
		updateDir: function() {
			const radX = -(this.rotation[0] * Math.PI) / 180;
			const radY = -(this.rotation[1] * Math.PI) / 180;
			this.direction[0] = Math.cos(radY) * Math.sin(radX);
			this.direction[1] = Math.sin(radY);
			this.direction[2] = Math.cos(radY) * Math.cos(radX);
		}
	}
	cam.updateDir();

	function fixTriangle(obj: {points?: mat3x3, normals?: mat3x3, uvs?: mat3x2}): {points: mat3x3, normals: mat3x3, uvs: mat3x2} {
		if (!obj.normals) {
			if (obj.points) {
				let edgeAB = obj.points[1].map((v, i) => v - obj.points![0][i]);
				let edgeAC = obj.points[2].map((v, i) => v - obj.points![0][i]);
				let normal = [
					edgeAB[1] * edgeAC[2] - edgeAB[2] * edgeAC[1],
					edgeAB[2] * edgeAC[0] - edgeAB[0] * edgeAC[2],
					edgeAB[0] * edgeAC[1] - edgeAB[1] * edgeAC[0]
				];
				const normLength = Math.sqrt(normal.reduce((a, b) => a + b * b, 0));
				const normalized = normal.map(n => n / normLength) as vec3;
				obj.normals = [[...normalized], [...normalized], [...normalized]];
			} else {
				obj.normals = [
					[0, 0, 1],
					[0, 0, 1],
					[0, 0, 1]
				];
			}
		}
		if (!obj.points) {
			obj.points = [
				[0, 0, 0],
				[0, 0, 0],
				[0, 0, 0]
			];
		}
		if (!obj.uvs) {
			obj.uvs = [
				[0, 0],
				[0, 0],
				[0, 0]
			]
		}
		return obj as {points: mat3x3, normals: mat3x3, uvs: mat3x2};
	}
	for (let i = 0; i < models.length; i++) {
		const model = models[i];
		for (let i = 0; i < model.triangles.length; i++) {
			model.triangles[i].index = Math.floor(triangles.length / 32);
			// obj.triangles[i].index = i;
			let t = fixTriangle(model.triangles[i]);
			triangles.push(...[
				...t.points.map(p => [...p, 1]).flat(),
				...t.normals.map(p => [...p, 0]).flat(),
				...t.uvs.flat(), 0, 0
			]);
		}
	}

	function add(obj: SceneObject) {
		if (!obj.position) obj.position = [0, 0, 0];
		if (!obj.rotation) obj.rotation = [0, 0, 0];
		if (!obj.scale) obj.scale = [1, 1, 1];
		obj.transform = [
			[1, 0, 0, 0],
			[0, 1, 0, 0],
			[0, 0, 1, 0],
			[0, 0, 0, 1]
		];

		obj.transform = matrixMultiply(obj.transform, [
			[1, 0, 0, obj.position[0]],
			[0, 1, 0, obj.position[1]],
			[0, 0, 1, obj.position[2]],
			[0, 0, 0, 1]
		]);

		obj.transform = matrixMultiply(obj.transform, [
			[obj.scale[0], 0, 0, 0],
			[0, obj.scale[1], 0, 0],
			[0, 0, obj.scale[2], 0],
			[0, 0, 0, 1]
		]);

		const radX = (obj.rotation[0] * Math.PI) / 180;
		const radY = (obj.rotation[1] * Math.PI) / 180;
		const radZ = (obj.rotation[2] * Math.PI) / 180;
		const rotX: mat4x4 = [
			[1, 0, 0, 0],
			[0, Math.cos(radX), -Math.sin(radX), 0],
			[0, Math.sin(radX), Math.cos(radX), 0],
			[0, 0, 0, 1]
		];
		const rotY: mat4x4 = [
			[Math.cos(radY), 0, Math.sin(radY), 0],
			[0, 1, 0, 0],
			[-Math.sin(radY), 0, Math.cos(radY), 0],
			[0, 0, 0, 1]
		];
		const rotZ: mat4x4 = [
			[Math.cos(radZ), -Math.sin(radZ), 0, 0],
			[Math.sin(radZ), Math.cos(radZ), 0, 0],
			[0, 0, 1, 0],
			[0, 0, 0, 1]
		];
		obj.transform = matrixMultiply(obj.transform, rotX);
		obj.transform = matrixMultiply(obj.transform, rotY);
		obj.transform = matrixMultiply(obj.transform, rotZ);

		for (let i = 0; i < 4; i++) {
			for (let j = 0; j < 4; j++) {
				if (Math.abs(obj.transform[i][j]) < 1e-6) {
					obj.transform[i][j] = 0;
				}
			}
		}

		if (obj.type === "mesh") {
			objs.push(...[
				1, obj.materialI, 0, 0,
				obj.modelI, 0, 0, 0,
				...obj.transform.flat(),
			]);
		} else if (obj.type === "sphere") {
			objs.push(...[
				2, obj.materialI, 0, 0,
				obj.radius || 0, 0, 0,
				...obj.transform.flat(),
			]);
		}
	}
	for (let i = 0; i < scene.length; i++) {
		scene[i].index = i;
		scene[i].materialI = materials.findIndex(m => m.id === scene[i].material);
		if (scene[i].type == "mesh") {
			(scene[i] as Mesh).modelI = models.findIndex(m => m.id === (scene[i] as Mesh).model);
		}
		add(scene[i]);
	}

	let boxes: AABB[] = [];

	class BasicTriangle {
		constructor(
			public points: mat3x3,
			public normals: mat3x3,
			public meshIndex: number,
			public index: number
		) {}
		get center() {
			return this.points.reduce((acc, p) => acc.map((v, i) => v + p[i] / 3) as vec3, [0, 0, 0]) as vec3;
		}
	}

	class AABB {
		min: vec3 = [Infinity, Infinity, Infinity];
		max: vec3 = [-Infinity, -Infinity, -Infinity];
		index: number;
		constructor(
			public addToList: boolean = true,
			public depth: number = 0,
			public childI1: number = -1,
			public childI2: number = -1
		) {
			if (addToList) {
				this.index = boxes.length;
				boxes.push(this);
			} else {
				this.index = -1;
			}
		}
		get size(): vec3 {
			return this.max.map((v, i) => v - this.min[i]) as vec3;
		}
		get corners(): vec3[] {
			return [
				[this.min[0], this.min[1], this.min[2]],
				[this.min[0], this.min[1], this.max[2]],
				[this.min[0], this.max[1], this.min[2]],
				[this.min[0], this.max[1], this.max[2]],
				[this.max[0], this.min[1], this.min[2]],
				[this.max[0], this.min[1], this.max[2]],
				[this.max[0], this.max[1], this.min[2]],
				[this.max[0], this.max[1], this.max[2]],
			];
		}
		get center(): vec3 {
			return this.min.map((v, i) => v / 2 + this.max[i] / 2) as vec3;
		}
		addPoint(point: vec3) {
			this.min = this.min.map((v, i) => Math.min(v, point[i])) as vec3;
			this.max = this.max.map((v, i) => Math.max(v, point[i])) as vec3;
		}
		addPoints(points: vec3[] | vec3[][]) {
			if (points.length === 0) return;
			if (0 in points[0] && Array.isArray(points[0][0])) {
				points.forEach(p => this.addPoints(p as vec3[]));
			} else {
				points.forEach(p => this.addPoint(p as vec3));
			}
		}
		contains(point: vec3): boolean {
			return (
				point[0] >= this.min[0] && point[0] <= this.max[0] &&
				point[1] >= this.min[1] && point[1] <= this.max[1] &&
				point[2] >= this.min[2] && point[2] <= this.max[2]
			);
		}
		pad(amount: number) {
			this.min = this.min.map(v => v - amount) as vec3;
			this.max = this.max.map(v => v + amount) as vec3;
		}
	}
	class SceneAABB extends AABB {
		constructor(addToList: boolean = false, depth: number = 0, childI1: number = -1, childI2: number = -1, public objects: ObjectAABB[] = []) {
			super(addToList, depth, childI1, childI2);
			this.addPoints(objects.map(o => o.min));
			this.addPoints(objects.map(o => o.max));
		}
		get cost(): number {
			return this.objects.map(o => o instanceof MeshAABB ? o.triangles.length : 1).reduce((a, b) => a + b, 0) * this.size.reduce((a, b) => a + b, 0);
		}
		addObject(object: ObjectAABB) {
			this.objects.push(object);
			this.addPoints([object.min, object.max]);
		}
		split() {
			let minCost = Infinity;
			let objs1: ObjectAABB[] = [];
			let objs2: ObjectAABB[] = [];
			for (let axis = 0; axis < 3; axis++) {
				for (let point = 0; point <= 1; point += 1 / (SCENE_TESTS_PER_AXIS - 1)) {
					if (point == 0 || point == 1) continue;
					let splitPoint = this.min[axis] + point * this.size[axis];
					// let axis = this.size.indexOf(Math.max(...this.size));
					// let point = this.center[splitAxis];
					let sorted = this.objects.toSorted((a, b) => a.center[axis] - b.center[axis]);
					let testObjs1 = sorted.filter((o, i) => o.center[axis] < splitPoint || (o.center[axis] == splitPoint && i < sorted.length / 2));
					let testObjs2 = sorted.filter(o => !testObjs1.includes(o));
					// let testObjs2 = this.objects.filter((o, i) => o.center[axis] > point || (i >= this.objects.length / 2));
					let cost = 0;
					if (testObjs1.length == 1) {
						cost += testObjs1[0].cost;
					} else if (testObjs1.length > 1) {
						cost += new SceneAABB(false, this.depth + 1, -1, -1, testObjs1).cost;
					}
					if (testObjs2.length == 1) {
						cost += testObjs2[0].cost;
					} else if (testObjs2.length > 1) {
						cost += new SceneAABB(false, this.depth + 1, -1, -1, testObjs2).cost;
					}
					if (cost < minCost) {
						minCost = cost;
						objs1 = testObjs1;
						objs2 = testObjs2;
					}
				}
			}
			let box1: AABB | undefined = undefined;
			let box2: AABB | undefined = undefined;
			if (objs1.length == 1) {
				box1 = objs1[0];
				this.childI1 = objs1[0].index;
			} else if (objs1.length > 1) {
				box1 = new SceneAABB(this.addToList, this.depth + 1, -1, -1, objs1);
				this.childI1 = box1.index;
			}
			if (objs2.length == 1) {
				box2 = objs2[0];
				this.childI2 = objs2[0].index;
			} else if (objs2.length > 1) {
				box2 = new SceneAABB(this.addToList, this.depth + 1, -1, -1, objs2);
				this.childI2 = box2.index;
			}
			if (box1 && this.depth < MAX_DEPTH && (box1 instanceof SceneAABB || box1 instanceof MeshAABB)) {
				box1.split();
			}
			if (box2 && this.depth < MAX_DEPTH && (box2 instanceof SceneAABB || box2 instanceof MeshAABB)) {
				box2.split();
			}
		}
	}
	class ObjectAABB extends AABB {
		transform: mat4x4;
		constructor(addToList: boolean = false, depth: number = 0, public objectI: number) {
			super(addToList, depth, -1, -1);
			this.transform = scene[objectI].transform;
		}
		get cost(): number {
			return this.size.reduce((a, b) => a + b, 0);
		}
	}
	class TriAABB extends AABB {
		objectI: number;
		triangleI: number;
		constructor(addToList: boolean = false, depth: number = 0, public triangle: BasicTriangle) {
			super(addToList, depth, -1, -1);
			this.addPoints(triangle.points);
			this.objectI = triangle.meshIndex;
			this.triangleI = triangle.index;
			this.pad(0.0001);
		}
		get cost(): number {
			return this.size.reduce((a, b) => a + b, 0);
		}
	}
	class MeshAABB extends ObjectAABB {
		constructor(addToList: boolean = false, depth: number = 0, public childI1: number = -1, public childI2: number = -1, public objectI: number, public triangles: BasicTriangle[] = []) {
			super(addToList, depth, objectI);
			this.addPoints(triangles.map(t => t.points));
			this.pad(0.0001);
		}
		get cost(): number {
			return this.triangles.length * this.size.reduce((a, b) => a + b, 0);
		}
		addTriangle(triangle: BasicTriangle) {
			this.triangles.push(triangle);
			this.addPoints(triangle.points);
		}
		split() {
			let minCost = Infinity;
			let tris1: BasicTriangle[] = [];
			let tris2: BasicTriangle[] = [];
			for (let axis = 0; axis < 3; axis++) {
				for (let point = 0; point <= 1; point += 1 / (MESH_TESTS_PER_AXIS - 1)) {
					if (point == 0 || point == 1) continue;
					let splitPoint = this.min[axis] + point * this.size[axis];
					let sorted = this.triangles.toSorted((a, b) => a.center[axis] - b.center[axis]);
					let testTris1 = sorted.filter((t, i) => t.center[axis] < splitPoint || (t.center[axis] == splitPoint && i < sorted.length / 2));
					let testTris2 = sorted.filter(t => !testTris1.includes(t));
					let cost = 0;
					if (testTris1.length == 1) {
						cost += new TriAABB(false, this.depth + 1, testTris1[0]).cost;
					} else if (testTris1.length > 1) {
						cost += testTris1.length;
						cost += new MeshAABB(false, this.depth + 1, -1, -1, this.objectI, testTris1).cost;
					}
					if (testTris2.length == 1) {
						cost += new TriAABB(false, this.depth + 1, testTris2[0]).cost;
					} else if (testTris2.length > 1) {
						cost += testTris2.length;
						cost += new MeshAABB(false, this.depth + 1, -1, -1, this.objectI, testTris2).cost;
					}
					if (cost < minCost) {
						minCost = cost;
						tris1 = testTris1;
						tris2 = testTris2;
					}
				}
			}
			let box1: AABB | undefined = undefined;
			let box2: AABB | undefined = undefined;
			if (tris1.length == 1) {
				box1 = new TriAABB(this.addToList, this.depth + 1, tris1[0]);
				this.childI1 = box1.index;
			} else if (tris1.length > 1) {
				box1 = new MeshAABB(this.addToList, this.depth + 1, -1, -1, this.objectI, tris1);
				this.childI1 = box1.index;
			}
			if (tris2.length == 1) {
				box2 = new TriAABB(this.addToList, this.depth + 1, tris2[0]);
				this.childI2 = box2.index;
			} else if (tris2.length > 1) {
				box2 = new MeshAABB(this.addToList, this.depth + 1, -1, -1, this.objectI, tris2);
				this.childI2 = box2.index;
			}
			if (box1 && this.depth < MAX_DEPTH && box1 instanceof MeshAABB) {
				box1.split();
			}
			if (box2 && this.depth < MAX_DEPTH && box2 instanceof MeshAABB) {
				box2.split();
			}
		}
	}

	let root = new SceneAABB(true, 0);
	scene.forEach(obj => {
		let box: ObjectAABB;
		if (obj.type === "mesh") {
			box = new MeshAABB(true, 0, -1, -1, obj.index, models[obj.modelI].triangles.map(t => new BasicTriangle(t.points.map(p => vectorMatrixMultiply([...p, 1], obj.transform).slice(0, 3)) as mat3x3, t.normals?.map(n => vectorMatrixMultiply([...n, 0], obj.transform).slice(0, 3)) as mat3x3 || [[0, 0, 0], [0, 0, 0], [0, 0, 0]], obj.index, t.index)));
		} else {
			box = new ObjectAABB(true, 0, obj.index);
			if (obj.type === "sphere") {
				box.addPoints([[obj.position[0] - obj.radius, obj.position[1] - obj.radius, obj.position[2] - obj.radius], [obj.position[0] + obj.radius, obj.position[1] + obj.radius, obj.position[2] + obj.radius]]);
			}
		}
		root.addObject(box);
	});
	root.split();
	boxes.forEach(box => {
		boxNums.push(...[
			...box.min, box.index,
			...box.max, box instanceof TriAABB ? 3 : box instanceof MeshAABB ? 2 : box instanceof SceneAABB ? 1 : box instanceof ObjectAABB ? 0 : -1,
			box.childI1, box.childI2,
			(box instanceof ObjectAABB || box instanceof TriAABB) ? box.objectI : 0, box instanceof TriAABB ? box.triangleI : 0,
			...new Array(4).fill(0)
		]);
	});

	textures.forEach(tex => {
		tex.index = textureNums.length / 4;
		textureNums.push(...[
			tex.data.width, tex.data.height, tex.lerpMode == "nearest" ? 0 : tex.lerpMode == "linear" ? 1 : tex.lerpMode == "cubic" ? 2 : 0, 0,
			...tex.data.pixels.flat()
		]);
	});

	materials.forEach((m, i) => {
		materialNums.push(...materialToRaw(i));
	});

	// console.log(root);
	// console.log(boxes);
	// console.log(objs);
	// console.log(triangles);
	// console.log(materials);

	// let stack: (AABB | undefined)[] = [...new Array(32).fill(undefined)];
	// stack[0] = boxes[0];
	// let stackEnd = 1;
	// let maxLen = 0;
	// for (let i = 0; i < 10000; i++) {
	// 	if (stackEnd > maxLen) {
	// 		maxLen = stackEnd;
	// 	}
	// 	stackEnd--;
	// 	if (stackEnd >= 0 && stack[stackEnd] instanceof AABB) {
	// 		let box = stack[stackEnd];
	// 		stack[stackEnd] = undefined;
	// 		if (box.childI1 !== -1) {
	// 			stack[stackEnd] = boxes[box.childI1];
	// 			stackEnd += 1;
	// 		}
	// 		if (box.childI2 !== -1) {
	// 			stack[stackEnd] = boxes[box.childI2];
	// 			stackEnd += 1;
	// 		}
	// 		if (box instanceof ObjectAABB) {
	// 			console.log("test object " + box.objectI);
	// 		}
	// 		if (box instanceof TriAABB) {
	// 			console.log("test triangle " + box.triangleI + " from mesh " + box.objectI);
	// 		}
	// 	} else {
	// 		console.log("broke at " + i + " with max length " + maxLen);
	// 		break;
	// 	}
	// }

	const OBJS_LEN = objs.length;
	const OBJS_SIZE = OBJS_LEN * 4;
	let rawObjs = new Float32Array(objs);

	const BOXES_LEN = boxNums.length;
	const BOXES_SIZE = BOXES_LEN * 4;
	let rawBoxes = new Float32Array(boxNums);

	const TRIANGLES_LEN = triangles.length;
	const TRIANGLES_SIZE = TRIANGLES_LEN * 4;
	let rawTriangles = new Float32Array(triangles);

	const MATERIALS_LEN = materialNums.length;
	const MATERIALS_SIZE = MATERIALS_LEN * 4;
	let rawMaterials = new Float32Array(materialNums);

	const TEXTURES_LEN = textureNums.length;
	const TEXTURES_SIZE = TEXTURES_LEN * 4;
	let rawTextures = new Float32Array(textureNums);

	let shader: string = rawShader;

	shader = shader.replaceAll("$OBJECT_CT", (OBJS_LEN / 8).toString());
	shader = shader.replaceAll("$OUTPUT_LEN", OUTPUT_LEN.toString());
	shader = shader.replaceAll("$BOX_CT", (BOXES_LEN / 16).toString());
	shader = shader.replaceAll("$TRIANGLE_CT", (TRIANGLES_LEN / 24).toString());
	shader = shader.replaceAll("$MATERIAL_CT", (MATERIALS_LEN / 32).toString());
	shader = shader.replaceAll("$SIZE", SIZE.toString());
	shader = shader.replaceAll("$BOUNCES", BOUNCES.toString());
	shader = shader.replaceAll("$RAYS_PER_PIXEL", RAYS_PER_PIXEL.toString());
	shader = shader.replaceAll("$CLEAR_FRAME", CLEAR_FRAME ? "true" : "false");

	const shaderModule = device.createShaderModule({
		code: shader,
	});

	const objsBuffer = device.createBuffer({
		size: OBJS_SIZE,
		usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
	});
	device.queue.writeBuffer(objsBuffer, 0, rawObjs, 0, OBJS_LEN);

	const boxesBuffer = device.createBuffer({
		size: BOXES_SIZE,
		usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
	});
	device.queue.writeBuffer(boxesBuffer, 0, rawBoxes, 0, BOXES_LEN);

	const trianglesBuffer = device.createBuffer({
		size: TRIANGLES_SIZE,
		usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
	});
	device.queue.writeBuffer(trianglesBuffer, 0, rawTriangles, 0, TRIANGLES_LEN);

	const materialsBuffer = device.createBuffer({
		size: MATERIALS_SIZE,
		usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
	});
	device.queue.writeBuffer(materialsBuffer, 0, rawMaterials, 0, MATERIALS_LEN);

	const texturesBuffer = device.createBuffer({
		size: TEXTURES_SIZE,
		usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
	});
	device.queue.writeBuffer(texturesBuffer, 0, rawTextures, 0, TEXTURES_LEN);

	const output = device.createBuffer({
		size: OUTPUT_SIZE,
		usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
	});

	const inputVars = device.createBuffer({
		size: VARS_SIZE,
		usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
	});

	const stagingBuffer = device.createBuffer({
		size: OUTPUT_SIZE,
		usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
	});

	const bindGroupLayout = device.createBindGroupLayout({
		entries: [
			{
				binding: 0,
				visibility: GPUShaderStage.COMPUTE,
				buffer: {
					type: "storage",
				},
			},
			{
				binding: 1,
				visibility: GPUShaderStage.COMPUTE,
				buffer: {
					type: "storage",
				},
			},
			{
				binding: 2,
				visibility: GPUShaderStage.COMPUTE,
				buffer: {
					type: "storage",
				},
			},
			{
				binding: 3,
				visibility: GPUShaderStage.COMPUTE,
				buffer: {
					type: "storage",
				},
			},
			{
				binding: 4,
				visibility: GPUShaderStage.COMPUTE,
				buffer: {
					type: "storage",
				},
			},
			{
				binding: 5,
				visibility: GPUShaderStage.COMPUTE,
				buffer: {
					type: "storage",
				},
			},
			{
				binding: 6,
				visibility: GPUShaderStage.COMPUTE,
				buffer: {
					type: "storage",
				},
			},
		],
	});

	const bindGroup = device.createBindGroup({
		layout: bindGroupLayout,
		entries: [
			{
				binding: 0,
				resource: {
					buffer: output,
				},
			},
			{
				binding: 1,
				resource: {
					buffer: objsBuffer,
				}
			},
			{
				binding: 2,
				resource: {
					buffer: inputVars,
				}
			},
			{
				binding: 3,
				resource: {
					buffer: boxesBuffer,
				}
			},
			{
				binding: 4,
				resource: {
					buffer: trianglesBuffer,
				}
			},
			{
				binding: 5,
				resource: {
					buffer: materialsBuffer,
				}
			},
			{
				binding: 6,
				resource: {
					buffer: texturesBuffer,
				}
			},
		],
	});

	const computePipeline = device.createComputePipeline({
		layout: device.createPipelineLayout({
			bindGroupLayouts: [bindGroupLayout],
		}),
		compute: {
			module: shaderModule,
			entryPoint: "main"
		},
	});

	device.popErrorScope().then((error) => {
		if (error) {
			throw Error(error.message);
		}
	});

	var data = new Float32Array(OUTPUT_LEN).fill(0);
	
	const canvas = document.createElement("canvas");
	canvas.width = SIZE;
	canvas.height = SIZE;
	document.body.appendChild(canvas);

	const ctx = canvas.getContext("2d");
	const imageData = ctx!.createImageData(SIZE, SIZE);

	let frame = 0;
	let framesSinceUpdate = 0;

	async function update() {
		data.fill(0);
		device.pushErrorScope("validation");
		for (let i = 0; i < RAYS_PER_PIXEL; i++) {
			device.queue.writeBuffer(inputVars, 0, new Float32Array([
				CLEAR_FRAME ? i : frame, i, 0, 0,
				...cam.position, cam.fov,
				...cam.direction, 0,
				cam.focusDistance, cam.defocusStrength, 0, 0
			]), 0, VARS_LEN);
			const commandEncoder = device.createCommandEncoder();
			const passEncoder = commandEncoder.beginComputePass();
			passEncoder.setPipeline(computePipeline);
			passEncoder.setBindGroup(0, bindGroup);
			passEncoder.dispatchWorkgroups(Math.ceil(SIZE / 2), Math.ceil(SIZE));
			passEncoder.end();
			device.queue.submit([commandEncoder.finish()]);
		}
		let commandEncoder = device.createCommandEncoder();
		commandEncoder.copyBufferToBuffer(output, 0, stagingBuffer, 0, OUTPUT_SIZE);
		device.queue.submit([commandEncoder.finish()]);
		await stagingBuffer.mapAsync(GPUMapMode.READ, 0, OUTPUT_SIZE);
		const arrayBuffer = stagingBuffer.getMappedRange(0, OUTPUT_SIZE);
		data = new Float32Array(arrayBuffer.slice());
		stagingBuffer.unmap();

		// if (frame == 0) console.log([...data]);

		device.popErrorScope().then((error) => {
			if (error) {
				cancelAnimationFrame(animationFrameId!)
				throw Error(error.message);
			}
		});

		for (let i = 0; i < OUTPUT_LEN; i++) {
			let r = data[i * 4 + 0];
			let g = data[i * 4 + 1];
			let b = data[i * 4 + 2];
			// let a = data[i * 4 + 3];
			let a = 1;
			let brightness = Math.max(r, g, b);
			if (brightness > 1) {
				r /= brightness;
				g /= brightness;
				b /= brightness;
				r = Math.pow(r, 1 / brightness);
				g = Math.pow(g, 1 / brightness);
				b = Math.pow(b, 1 / brightness);
			}

			if (!imageData) continue

			imageData.data[i * 4 + 0] = r * 255;
			imageData.data[i * 4 + 1] = g * 255;
			imageData.data[i * 4 + 2] = b * 255;
			imageData.data[i * 4 + 3] = a * 255;
		}
		
		ctx!.putImageData(imageData, 0, 0);
		
		frame++;
		framesSinceUpdate++;
		
		animationFrameId = requestAnimationFrame(update);
	}
	await update();
	setInterval(() => {
		console.log(`FPS: ${(framesSinceUpdate / 5).toFixed(1)} (${frame * RAYS_PER_PIXEL} rays)`);
		framesSinceUpdate = 0;
	}, 5000);
}
init();
