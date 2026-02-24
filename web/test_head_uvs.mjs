import fs from 'fs';

const buf = fs.readFileSync('/home/tommy/Desktop/MobileClient/web/assets/models/bam.glb');
const jsonLen = buf.readUInt32LE(12);
const jsonChunk = buf.toString('utf8', 20, 20 + jsonLen);
const gltf = JSON.parse(jsonChunk);

const binChunkOffset = 20 + jsonLen + 8; // skip chunk length and type

// Find which materials are "BAMHE"
const headMaterialIndices = [];
for (let i = 0; i < gltf.materials.length; i++) {
    if (gltf.materials[i].name && gltf.materials[i].name.toUpperCase().includes('BAMHE')) {
        headMaterialIndices.push(i);
    }
}
console.log(`Head Material Indices: ${headMaterialIndices.join(', ')}`);

// Find primitives using those materials and check their UVs
let headPrims = 0;
let totalHeadUvs = 0;
let zeroHeadUvs = 0;
let nonZeroCount = 0;

for (const mesh of gltf.meshes) {
    for (const prim of mesh.primitives) {
        if (headMaterialIndices.includes(prim.material)) {
            headPrims++;
            if ('TEXCOORD_0' in prim.attributes) {
                const idx = prim.attributes['TEXCOORD_0'];
                const accessor = gltf.accessors[idx];
                const bufferView = gltf.bufferViews[accessor.bufferView];

                const offset = binChunkOffset + (bufferView.byteOffset || 0) + (accessor.byteOffset || 0);
                for (let i = 0; i < accessor.count; i++) {
                    const u = buf.readFloatLE(offset + i * 8);
                    const v = buf.readFloatLE(offset + i * 8 + 4);
                    totalHeadUvs++;
                    if (Math.abs(u) < 0.001 && Math.abs(v) < 0.001) {
                        zeroHeadUvs++;
                    } else if (nonZeroCount < 20) {
                        console.log(`Non-zero Head UV: [${u.toFixed(3)}, ${v.toFixed(3)}]`);
                        nonZeroCount++;
                    }
                }
            }
        }
    }
}

console.log(`Found ${headPrims} primitives using head materials.`);
console.log(`Head UVs: ${totalHeadUvs}`);
console.log(`Zero Head UVs: ${zeroHeadUvs}`);
