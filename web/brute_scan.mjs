import fs from 'fs';
import { S3DDecoder } from '/home/tommy/eqsage/sage/lib/s3d/s3d-decoder.js';

async function scanAll() {
    const eqPath = '/home/tommy/RoF2_Client';
    const files = fs.readdirSync(eqPath).filter(f => f.toLowerCase().endsWith('_chr.s3d'));

    console.log(`Scanning ${files.length} _chr.s3d files...`);

    for (const f of files) {
        const filePath = `${eqPath}/${f}`;
        const buf = fs.readFileSync(filePath).buffer;
        const d = new S3DDecoder(null, { forceWrite: false });

        global.window = { imageProcessor: { createTextureFromBitmapOrDDS: () => { } } };
        global.document = { createElement: () => ({ getContext: () => ({}) }) };

        try {
            await d.processS3D({
                name: f,
                arrayBuffer: async () => buf,
                text: async () => Buffer.from(buf).toString('utf8')
            }, true);

            for (const wld of d.wldFiles) {
                const matches = wld.meshes.filter(m => m.name && m.name.toLowerCase().includes('bam'));
                if (matches.length > 0) {
                    console.log(`\n✅ Matches in ${f} (${wld.name}):`);
                    matches.forEach(m => console.log(`  - ${m.name}: ${m.vertices.length} verts`));
                }
            }
        } catch (e) {
            // console.error(`Failed to process ${f}`);
        }
    }
}

scanAll();
