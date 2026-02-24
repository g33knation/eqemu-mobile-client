import fs from 'fs';

const buf = fs.readFileSync('/home/tommy/Desktop/MobileClient/public/assets/chars/bam.glb');
const magic = buf.toString('utf8', 0, 4);
if (magic !== 'glTF') {
    console.error('Not a GLB file');
    process.exit(1);
}

const jsonLen = buf.readUint32LE(12);
const jsonChunk = buf.toString('utf8', 20, 20 + jsonLen);
const json = JSON.parse(jsonChunk);

console.log(JSON.stringify(json, (key, value) => {
    if (Array.isArray(value) && value.length > 50) return `Array(${value.length})`;
    return value;
}, 2));
