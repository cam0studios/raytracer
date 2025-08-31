import { Jimp } from "jimp";

const pathToPNG = process.argv[2];
const pathToDestination = process.argv[3];
const data = {
	width: 0,
	height: 0,
	pixels: [] as [number, number, number, number][]
};

const image = await Jimp.read(pathToPNG);
data.width = image.bitmap.width;
data.height = image.bitmap.height;
for (let i = 0; i < image.bitmap.data.length; i += 4) {
	data.pixels.push([
		image.bitmap.data[i] / 255,
		image.bitmap.data[i + 1] / 255,
		image.bitmap.data[i + 2] / 255,
		image.bitmap.data[i + 3] / 255
	]);
}
await Bun.write(pathToDestination, JSON.stringify(data));