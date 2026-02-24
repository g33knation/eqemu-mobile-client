import fs from 'fs';
import { S3DDecoder } from '/home/tommy/eqsage/sage/lib/s3d/s3d-decoder.js';

async function scanArmor() {
    const eqPath = '/home/tommy/RoF2_Client';
    const files = fs.readdirSync(eqPath).filter(f => f.toLowerCase().endsWith('_amr.s3d'));

    console.log(`Scanning ${files.length} _amr.s3d files...`);

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

            // The warnings will be logged by the modified wld.js
        } catch (e) {
            // console.error(`Failed to process ${f}`, e);
        }
    }
}

scanArmor();
