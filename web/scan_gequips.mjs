import fs from 'fs';
import { S3DDecoder } from '/home/tommy/eqsage/sage/lib/s3d/s3d-decoder.js';

async function scanGEquip() {
    const eqPath = '/home/tommy/RoF2_Client';
    const files = [
        'gequip.s3d', 'gequip2.s3d', 'gequip3.s3d', 'gequip4.s3d',
        'gequip5.s3d', 'gequip6.s3d', 'gequip8.s3d',
        'lgequip.s3d', 'lgequip2.s3d', 'lgequip_amr.s3d', 'lgequip_amr2.s3d'
    ];

    console.log(`Scanning ${files.length} equip archives...`);

    for (const f of files) {
        const filePath = `${eqPath}/${f}`;
        if (!fs.existsSync(filePath)) continue;

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
                const bamFrags = wld.fragments.filter(frag => frag && frag.name && frag.name.toUpperCase().includes('BAM'));
                if (bamFrags.length > 0) {
                    console.log(`[FILE] ${f} - found ${bamFrags.length} BAM fragments`);
                    bamFrags.forEach(frag => {
                        console.log(`  - ${frag.constructor.name}: ${frag.name}`);
                    });
                }
            }
        } catch (e) {
            console.error(`Error processing ${f}: ${e.message}`);
        }
    }
}

scanGEquip();
