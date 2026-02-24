import fs from 'fs';

const buf = fs.readFileSync('/home/tommy/Desktop/MobileClient/web/assets/models/bam.glb');
const magic = buf.toString('utf8', 0, 4);
if (magic !== 'glTF') process.exit(1);

const jsonLen = buf.readUInt32LE(12);
const jsonChunk = buf.toString('utf8', 20, 20 + jsonLen);
const gltf = JSON.parse(jsonChunk);

const prim = gltf.meshes[0].primitives[0];
if ('TEXCOORD_0' in prim.attributes) {
    const idx = prim.attributes['TEXCOORD_0'];
    const accessor = gltf.accessors[idx];
    console.log(`Accessor min: ${accessor.min}`);
    console.log(`Accessor max: ${accessor.max}`);
    console.log(`Count: ${accessor.count}`);
} else {
    console.log('No TEXCOORD_0');
}
