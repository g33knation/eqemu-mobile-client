import fs from 'fs';
import { S3DDecoder } from '/home/tommy/eqsage/sage/lib/s3d/s3d-decoder.js';

async function scan(archiveName) {
    const filePath = `/home/tommy/RoF2_Client/${archiveName}.s3d`;
    if (!fs.existsSync(filePath)) return;

    const buf = fs.readFileSync(filePath).buffer;
    const d = new S3DDecoder(null, { forceWrite: false });

    global.window = { imageProcessor: { createTextureFromBitmapOrDDS: () => { } } };
    global.document = { createElement: () => ({ getContext: () => ({}) }) };

    await d.processS3D({
        name: `${archiveName}.s3d`,
        arrayBuffer: async () => buf,
        text: async () => Buffer.from(buf).toString('utf8')
    }, true);

    for (const wld of d.wldFiles) {
        const matches = wld.meshes.filter(m => m.name && m.name.toLowerCase().includes('bam'));
        if (matches.length > 0) {
            console.log(`\n✅ Matches in ${archiveName} (${wld.name}):`);
            matches.forEach(m => console.log(`  - ${m.name}: ${m.vertices.length} verts`));
        }
    }
}

async function run() {
    await scan('global_chr');
    await scan('global2_chr');
    await scan('global3_chr');
    await scan('global4_chr');
    await scan('globalbam_chr');
    await scan('globalbam_chr2');
}
run();
