import fs from 'fs';

const filename = process.argv[2];
if (!filename) {
    console.error('Usage: node get_glb_json.mjs <filename.glb>');
    process.exit(1);
}

const buffer = fs.readFileSync(filename);
const length = buffer.readUInt32LE(12);
const jsonData = buffer.toString('utf8', 20, 20 + length);
console.log(jsonData);
