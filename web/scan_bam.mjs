import fs from 'fs';
import path from 'path';
import { S3DDecoder } from '/home/tommy/eqsage/sage/lib/s3d/s3d-decoder.js';

async function scanBAM() {
    const eqPath = '/home/tommy/RoF2_Client';
    const archivesToCheck = fs.readdirSync(eqPath).filter(f => f.endsWith('_chr.s3d'));

    global.window = { imageProcessor: { createTextureFromBitmapOrDDS: () => { } } };
    global.document = { createElement: () => ({ getContext: () => ({}) }) };

    for (const arch of archivesToCheck) {
        try {
            const buf = fs.readFileSync(path.join(eqPath, arch)).buffer;
            const d = new S3DDecoder(null, { forceWrite: false });
            await d.processS3D({
                name: arch,
                arrayBuffer: async () => buf,
                text: async () => Buffer.from(buf).toString('utf8')
            }, true);

            const wld = d.wldFiles.find(w => w.name.includes('.wld'));
            if (!wld) continue;

            const bamParts = wld.meshes.filter(m => m.name && (m.name.toLowerCase().startsWith('bamch') || m.name.toLowerCase().startsWith('bamlg')));

            if (bamParts.length > 0) {
                console.log(`\n✅ FOUND BAM BODY PARTS IN: ${arch}!`);
                bamParts.forEach(m => console.log(`  - ${m.name}: ${m.vertices.length} verts`));
            }

        } catch (e) { }
    }
}
scanBAM();
