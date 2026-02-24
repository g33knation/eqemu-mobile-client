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

let primIndex = 0;
for (const mesh of gltf.meshes) {
    for (const prim of mesh.primitives) {
        if (headMaterialIndices.includes(prim.material) && 'POSITION' in prim.attributes) {
            const idx = prim.attributes['POSITION'];
            const accessor = gltf.accessors[idx];
            const bufferView = gltf.bufferViews[accessor.bufferView];

            const offset = binChunkOffset + (bufferView.byteOffset || 0) + (accessor.byteOffset || 0);

            let minX = Infinity, maxX = -Infinity;
            let minY = Infinity, maxY = -Infinity;
            let minZ = Infinity, maxZ = -Infinity;

            const stride = bufferView.byteStride || 12;

            for (let i = 0; i < accessor.count; i++) {
                const vertOffset = offset + i * stride;
                const x = buf.readFloatLE(vertOffset);
                const y = buf.readFloatLE(vertOffset + 4);
                const z = buf.readFloatLE(vertOffset + 8);

                if (x < minX) minX = x;
                if (x > maxX) maxX = x;
                if (y < minY) minY = y;
                if (y > maxY) maxY = y;
                if (z < minZ) minZ = z;
                if (z > maxZ) maxZ = z;
            }

            console.log(`Head Prim ${primIndex++} (Mat ${prim.material}): Points=${accessor.count}`);
            console.log(`  X: [${minX.toFixed(3)}, ${maxX.toFixed(3)}]`);
            console.log(`  Y: [${minY.toFixed(3)}, ${maxY.toFixed(3)}]`);
            console.log(`  Z: [${minZ.toFixed(3)}, ${maxZ.toFixed(3)}]`);
        }
    }
}
