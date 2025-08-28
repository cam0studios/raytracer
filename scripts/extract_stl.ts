import StlReader from "stl-reader";

const pathToSTL = process.argv[2];
const pathToDestination = process.argv[3];
const file = Bun.file(pathToSTL);
let fileAsAB = await file.arrayBuffer();
const read = new StlReader().read(fileAsAB);
const triangles: {points: number[][], normals: number[][]}[] = [];
for (let i = 0; i < read.vertices.length; i += 9) {
    triangles.push({
        points: [
			[read.vertices[i], read.vertices[i + 1], read.vertices[i + 2]],
			[read.vertices[i + 3], read.vertices[i + 4], read.vertices[i + 5]],
			[read.vertices[i + 6], read.vertices[i + 7], read.vertices[i + 8]],
		],
		normals: [
			[read.normals[i], read.normals[i + 1], read.normals[i + 2]],
			[read.normals[i + 3], read.normals[i + 4], read.normals[i + 5]],
			[read.normals[i + 6], read.normals[i + 7], read.normals[i + 8]]
		]
    });
}
await Bun.write(pathToDestination, JSON.stringify(triangles));