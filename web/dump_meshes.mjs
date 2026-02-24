import fs from 'fs';
import path from 'path';
import { S3DDecoder } from '/home/tommy/eqsage/sage/lib/s3d/s3d-decoder.js';

async function run() {
    const buf = fs.readFileSync('/home/tommy/RoF2_Client/global4_chr.s3d').buffer;
    const d = new S3DDecoder(null, { forceWrite: false });

    global.window = { imageProcessor: { createTextureFromBitmapOrDDS: () => { } } };
    global.document = { createElement: () => ({ getContext: () => ({}) }) };

    await d.processS3D({
        name: 'global4_chr.s3d',
        arrayBuffer: async () => buf,
        text: async () => Buffer.from(buf).toString('utf8')
    }, true);

    const wld = d.wldFiles.find(w => w.name.includes('.wld'));
    if (!wld) return;

    console.log(`Total Meshes in global4_chr: ${wld.meshes.length}`);
    for (let i = 0; i < Math.min(15, wld.meshes.length); i++) {
        console.log(`- ${wld.meshes[i].name}: ${wld.meshes[i].vertices.length} vertices`);
    }
}
run();
