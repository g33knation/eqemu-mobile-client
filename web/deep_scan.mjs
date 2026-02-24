import fs from 'fs';
import { S3DDecoder } from '/home/tommy/eqsage/sage/lib/s3d/s3d-decoder.js';

async function scanDeep() {
    const eqPath = '/home/tommy/RoF2_Client';
    const files = fs.readdirSync(eqPath).filter(f => f.toLowerCase().endsWith('_chr.s3d') || f.toLowerCase().endsWith('_amr.s3d'));

    console.log(`Deep scanning ${files.length} archives...`);

    for (const f of files) {
        const filePath = `${eqPath}/${f}`;
        const buf = fs.readFileSync(filePath).buffer;

        // We catch logs from wld.js
        console.log(`[FILE] ${f}`);
        global.currentArchive = f;
        const d = new S3DDecoder(null, { forceWrite: false });

        global.window = { imageProcessor: { createTextureFromBitmapOrDDS: () => { } } };
        global.document = { createElement: () => ({ getContext: () => ({}) }) };

        try {
            await d.processS3D({
                name: f,
                arrayBuffer: async () => buf,
                text: async () => Buffer.from(buf).toString('utf8')
            }, true);
        } catch (e) { }
    }
}

scanDeep();
