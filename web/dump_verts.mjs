import fs from 'fs';
import { S3DDecoder } from '/home/tommy/eqsage/sage/lib/s3d/s3d-decoder.js';

async function dumpVerts(archiveName) {
    console.log(`\n--- VERTS IN ${archiveName} ---`);
    const buf = fs.readFileSync(`/home/tommy/RoF2_Client/${archiveName}`).buffer;
    const d = new S3DDecoder(null, { forceWrite: false });

    global.window = { imageProcessor: { createTextureFromBitmapOrDDS: () => { } } };
    global.document = { createElement: () => ({ getContext: () => ({}) }) };

    await d.processS3D({
        name: archiveName,
        arrayBuffer: async () => buf,
        text: async () => Buffer.from(buf).toString('utf8')
    }, true);

    for (const wld of d.wldFiles) {
        for (const f of wld.fragments) {
            if (f && f.vertices) {
                console.log(`  Mesh: ${f.name}, Verts: ${f.vertices.length}`);
            }
        }
    }
}

async function run() {
    await dumpVerts('globalbam_chr.s3d');
    await dumpVerts('globalelf_chr.s3d');
}

run();
