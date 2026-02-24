import fs from 'fs';

const glbPath = process.argv[2] || '/home/tommy/Desktop/MobileClient/web/assets/models/bamhe00.glb';
if (!fs.existsSync(glbPath)) {
    console.error(`File not found: ${glbPath}`);
    process.exit(1);
}

const buf = fs.readFileSync(glbPath);
const jsonLen = buf.readUInt32LE(12);
const jsonChunk = buf.toString('utf8', 20, 20 + jsonLen);
const gltf = JSON.parse(jsonChunk);

const binChunkOffset = 20 + jsonLen + 8;

console.log(`GLB: ${glbPath}`);
console.log(`Meshes: ${gltf.meshes?.length || 0}`);
if (gltf.meshes) {
    gltf.meshes.forEach((m, i) => {
        console.log(`Mesh [${i}]: ${m.name}`);
        m.primitives.forEach((p, pi) => {
            const mat = gltf.materials[p.material];
            console.log(`  Prim [${pi}]: Mat=${mat?.name || 'Unnamed'} (Index ${p.material})`);
            if (p.attributes.POSITION !== undefined) {
                const acc = gltf.accessors[p.attributes.POSITION];
                console.log(`    Points: ${acc.count}`);

                const bv = gltf.bufferViews[acc.bufferView];
                const stride = bv.byteStride || 12;
                const offset = binChunkOffset + (bv.byteOffset || 0) + (acc.byteOffset || 0);

                let minX = Infinity, maxX = -Infinity;
                let minY = Infinity, maxY = -Infinity;
                let minZ = Infinity, maxZ = -Infinity;

                for (let j = 0; j < acc.count; j++) {
                    const x = buf.readFloatLE(offset + j * stride);
                    const y = buf.readFloatLE(offset + j * stride + 4);
                    const z = buf.readFloatLE(offset + j * stride + 8);

                    if (x < minX) minX = x; if (x > maxX) maxX = x;
                    if (y < minY) minY = y; if (y > maxY) maxY = y;
                    if (z < minZ) minZ = z; if (z > maxZ) maxZ = z;
                }
                console.log(`    X: [${minX.toFixed(3)}, ${maxX.toFixed(3)}]`);
                console.log(`    Y: [${minY.toFixed(3)}, ${maxY.toFixed(3)}]`);
                console.log(`    Z: [${minZ.toFixed(3)}, ${maxZ.toFixed(3)}]`);
            }
        });
    });
}
