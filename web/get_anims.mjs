import fs from 'fs';
const buf = fs.readFileSync(process.argv[2] || '/home/tommy/Desktop/MobileClient/web/assets/models/bam.glb');
// find JSON chunk length
const jsonLength = buf.readUInt32LE(12);
const jsonChunk = buf.subarray(20, 20 + jsonLength).toString('utf8');
const gltf = JSON.parse(jsonChunk);
console.log("Animations:", gltf.animations ? gltf.animations.map((a, i) => a.name || i) : "None");
