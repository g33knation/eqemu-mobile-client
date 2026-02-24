import fs from 'fs';

const buf = fs.readFileSync('/home/tommy/Desktop/MobileClient/web/assets/models/bam.glb');
const jsonLen = buf.readUInt32LE(12);
const jsonChunk = buf.toString('utf8', 20, 20 + jsonLen);
const gltf = JSON.parse(jsonChunk);

const binChunkOffset = 20 + jsonLen + 8; // skip chunk length and type

let totalZeros = 0;
let totalUvs = 0;

for (const mesh of gltf.meshes) {
    for (const prim of mesh.primitives) {
        if ('TEXCOORD_0' in prim.attributes) {
            const idx = prim.attributes['TEXCOORD_0'];
            const accessor = gltf.accessors[idx];
            const bufferView = gltf.bufferViews[accessor.bufferView];

            const offset = binChunkOffset + (bufferView.byteOffset || 0) + (accessor.byteOffset || 0);
            for (let i = 0; i < accessor.count; i++) {
                const u = buf.readFloatLE(offset + i * 8);
                const v = buf.readFloatLE(offset + i * 8 + 4);
                totalUvs++;
                if (Math.abs(u) < 0.001 && Math.abs(v) < 0.001) {
                    totalZeros++;
                }
            }
        }
    }
}

console.log(`Total UVs: ${totalUvs}`);
console.log(`Total Zeros: ${totalZeros} (${((totalZeros / totalUvs) * 100).toFixed(2)}%)`);
