import fs from 'fs';

const buf = fs.readFileSync('/home/tommy/Desktop/MobileClient/web/assets/models/bam.glb');
const jsonLen = buf.readUInt32LE(12);
const jsonChunk = buf.toString('utf8', 20, 20 + jsonLen);
const gltf = JSON.parse(jsonChunk);

console.log("Meshes:");
for (let i = 0; i < gltf.meshes.length; i++) {
    console.log(`[${i}] ${gltf.meshes[i].name}`);
}

console.log("\nNodes:");
for (let i = 0; i < gltf.nodes.length; i++) {
    const node = gltf.nodes[i];
    if (node.mesh !== undefined) {
        console.log(`[${i}] ${node.name} -> Mesh ${node.mesh}`);
    }
}
