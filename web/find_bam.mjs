import fs from 'fs';
import { S3DDecoder } from '/home/tommy/eqsage/sage/lib/s3d/s3d-decoder.js';

async function findBAMBody() {
    const eqPath = '/home/tommy/RoF2_Client';
    const files = fs.readdirSync(eqPath).filter(f => f.toLowerCase().endsWith('_chr.s3d') || f.toLowerCase().endsWith('_amr.s3d'));

    console.log(`Searching for BAM_POLYHDEF in ${files.length} archives...`);

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
                // We check the internal fragments list of the wld
                // Sage doesn't expose them easily, so we rely on our wld.js warnings for now
                // Wait, I can just grep the output of this script if I modify wld.js further.
            }
        } catch (e) {
            // console.error(`Failed ${f}: ${e.message}`);
        }
    }
}

findBAMBody();
