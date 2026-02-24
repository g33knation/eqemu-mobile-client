import fs from 'fs';
import { S3DDecoder } from '/home/tommy/eqsage/sage/lib/s3d/s3d-decoder.js';

async function inspectActor() {
    const archiveName = 'globalbam_chr.s3d';
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
        const mesh = wld.meshes.find(m => m.name === 'BAM_DMSPRITEDEF');
        if (mesh) {
            console.log(`Found Mesh: ${mesh.name}`);
            console.log(`Vertices: ${mesh.vertices.length}`);
            if (mesh.materialList) {
                const materials = mesh.materialList.materialList;
                console.log(`Material List: ${mesh.materialList.name} (${materials.length} materials)`);
                const matCounts = new Array(materials.length).fill(0);
                mesh.materialGroups.forEach(group => {
                    matCounts[group.materialIndex] += group.polygonCount;
                });
                materials.forEach((mat, i) => {
                    console.log(`  - [${i}] ${mat ? mat.name : 'NULL'} (Polygons: ${matCounts[i]})`);
                });
            } else {
                console.log("NO MATERIAL LIST");
            }
        }
    }
}

inspectActor();
