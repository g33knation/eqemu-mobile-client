import fs from 'fs';

const filename = process.argv[2];
if (!filename) {
    console.error('Usage: node extract_json.mjs <filename.glb>');
    process.exit(1);
}

const buffer = fs.readFileSync(filename);
const jsonData = JSON.parse(buffer.toString('utf8', 20, 20 + buffer.readUInt32LE(12)));

let bakedImages = 0;
let externalImages = 0;
let emptyImages = 0;

jsonData.images?.forEach((img, i) => {
    if (img.bufferView !== undefined) {
        bakedImages++;
    } else if (img.uri) {
        externalImages++;
    } else {
        emptyImages++;
    }
});

console.log(`GLB Summary for ${filename}:`);
console.log(`- Total Images: ${jsonData.images?.length || 0}`);
console.log(`- Baked (bufferView): ${bakedImages}`);
console.log(`- External (uri): ${externalImages}`);
console.log(`- Invalid (no bufferView or uri): ${emptyImages}`);

if (bakedImages > 0) {
    const firstBaked = jsonData.images.find(img => img.bufferView !== undefined);
    console.log(`\nExample Baked Image:`, JSON.stringify(firstBaked, null, 2));
    console.log(`BufferView details:`, JSON.stringify(jsonData.bufferViews[firstBaked.bufferView], null, 2));
}
