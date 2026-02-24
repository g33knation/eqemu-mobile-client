import fs from 'fs';

const buf = fs.readFileSync('/home/tommy/Desktop/MobileClient/web/assets/models/bam.glb');
const jsonLen = buf.readUInt32LE(12);
const jsonChunk = buf.toString('utf8', 20, 20 + jsonLen);
const gltf = JSON.parse(jsonChunk);

const binChunkOffset = 20 + jsonLen + 8; // skip chunk length and type

const prim = gltf.meshes[0].primitives[0];
const idx = prim.attributes['TEXCOORD_0'];
const accessor = gltf.accessors[idx];
const bufferView = gltf.bufferViews[accessor.bufferView];

const offset = binChunkOffset + (bufferView.byteOffset || 0) + (accessor.byteOffset || 0);
let uvs = [];
for (let i = 0; i < Math.min(accessor.count, 5); i++) {
    const u = buf.readFloatLE(offset + i * 8);
    const v = buf.readFloatLE(offset + i * 8 + 4);
    uvs.push(`[${u.toFixed(3)}, ${v.toFixed(3)}]`);
}
console.log(`First 5 UVs: ${uvs.join(', ')}`);
