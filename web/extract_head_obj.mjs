import fs from 'fs';

const buf = fs.readFileSync('/home/tommy/Desktop/MobileClient/web/assets/models/bam.glb');
const jsonLen = buf.readUInt32LE(12);
const jsonChunk = buf.toString('utf8', 20, 20 + jsonLen);
const gltf = JSON.parse(jsonChunk);

const binChunkOffset = 20 + jsonLen + 8;

const headMaterialIndices = [];
for (let i = 0; i < gltf.materials.length; i++) {
    if (gltf.materials[i].name && gltf.materials[i].name.toUpperCase().includes('BAMHE')) {
        headMaterialIndices.push(i);
    }
}

let objData = "";
let vertexOffset = 1; // OBJ is 1-indexed

for (const mesh of gltf.meshes) {
    for (const prim of mesh.primitives) {
        if (headMaterialIndices.includes(prim.material) && 'POSITION' in prim.attributes && prim.indices !== undefined) {

            // 1. Read Vertices
            const posIdx = prim.attributes['POSITION'];
            const posAccessor = gltf.accessors[posIdx];
            const posBufferView = gltf.bufferViews[posAccessor.bufferView];
            const posOffset = binChunkOffset + (posBufferView.byteOffset || 0) + (posAccessor.byteOffset || 0);
            const stride = posBufferView.byteStride || 12;

            for (let i = 0; i < posAccessor.count; i++) {
                const vertOffset = posOffset + i * stride;
                const x = buf.readFloatLE(vertOffset);
                const y = buf.readFloatLE(vertOffset + 4);
                const z = buf.readFloatLE(vertOffset + 8);
                objData += `v ${x} ${y} ${z}\n`;
            }

            // 2. Read Indices
            const idxAccessor = gltf.accessors[prim.indices];
            const idxBufferView = gltf.bufferViews[idxAccessor.bufferView];
            const idxOffset = binChunkOffset + (idxBufferView.byteOffset || 0) + (idxAccessor.byteOffset || 0);

            // Assume unsigned short (componentType 5123) or unsigned int (5125)
            const isUint32 = idxAccessor.componentType === 5125;

            for (let i = 0; i < idxAccessor.count; i += 3) {
                let v1, v2, v3;
                if (isUint32) {
                    v1 = buf.readUInt32LE(idxOffset + i * 4);
                    v2 = buf.readUInt32LE(idxOffset + (i + 1) * 4);
                    v3 = buf.readUInt32LE(idxOffset + (i + 2) * 4);
                } else {
                    v1 = buf.readUInt16LE(idxOffset + i * 2);
                    v2 = buf.readUInt16LE(idxOffset + (i + 1) * 2);
                    v3 = buf.readUInt16LE(idxOffset + (i + 2) * 2);
                }

                // OBJ faces
                objData += `f ${v1 + vertexOffset} ${v2 + vertexOffset} ${v3 + vertexOffset}\n`;
            }

            vertexOffset += posAccessor.count;
        }
    }
}

fs.writeFileSync('/home/tommy/Desktop/MobileClient/web/head_only.obj', objData);
console.log('Saved head_only.obj');
