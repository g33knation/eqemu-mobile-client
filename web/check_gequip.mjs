import fs from 'fs';
import path from 'path';
import { S3DDecoder } from '/home/tommy/eqsage/sage/lib/s3d/s3d-decoder.js';

async function run() {
    const buf = fs.readFileSync('/home/tommy/RoF2_Client/lgequip_amr.s3d').buffer;
    const d = new S3DDecoder(null, { forceWrite: false });

    global.window = { imageProcessor: { createTextureFromBitmapOrDDS: () => { } } };
    global.document = { createElement: () => ({ getContext: () => ({}) }) };

    await d.processS3D({
        name: 'lgequip_amr.s3d',
        arrayBuffer: async () => buf,
        text: async () => Buffer.from(buf).toString('utf8')
    }, true);

    const wld = d.wldFiles.find(w => w.name.includes('.wld'));
    if (!wld) return;

    const bamParts = wld.meshes.filter(m => m.name && (m.name.toLowerCase().startsWith('bamch') || m.name.toLowerCase().startsWith('bamlg') || m.name.toLowerCase().startsWith('bamfa') || m.name.toLowerCase().startsWith('bamhn') || m.name.toLowerCase().startsWith('bamft') || m.name.toLowerCase().startsWith('bamua')));

    if (bamParts.length > 0) {
        console.log(`\n✅ FOUND BAM BODY PARTS IN: lgequip_amr.s3d!`);
        bamParts.forEach(m => console.log(`  - ${m.name}: ${m.vertices.length} verts`));
    } else {
        console.log("NOT HERE EITHER");
    }
}
run();
