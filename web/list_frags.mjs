import { S3DDecoder } from '/home/tommy/eqsage/sage/lib/s3d/s3d-decoder.js';
import fs from 'fs';
import path from 'path';

// Minimal mock environment
global.createDirMock = () => ({});
Object.defineProperty(global, 'navigator', {
    value: { storage: { getDirectory: async () => ({}) } },
    writable: true,
    configurable: true
});
Object.defineProperty(global, 'window', {
    value: {},
    writable: true,
    configurable: true
});

async function processArchive(filePath, outputObj) {
    if (!fs.existsSync(filePath)) return;
    const arrayBuffer = fs.readFileSync(filePath).buffer;
    const decoder = new S3DDecoder(null, { forceWrite: false });
    await decoder.processS3D({ name: path.basename(filePath), arrayBuffer: async () => arrayBuffer }, true);
    for (const wld of decoder.wldFiles) {
        outputObj.str += `WLD: ${wld.name}\n`;
        const frags = wld.fragments;
        for (const [id, frag] of Object.entries(frags)) {
            const type = frag.constructor?.name;
            outputObj.str += `  ${type} [${id}]: ${frag.name || 'Unnamed'}\n`;
        }
    }
}

async function run() {
    let outputObj = { str: '' };
    await processArchive('/home/tommy/RoF2_Client/global_chr.s3d', outputObj);
    fs.writeFileSync('/home/tommy/Desktop/MobileClient/web/global_chr_frags.txt', outputObj.str);
    console.log("Done");
}
run().catch(console.error);
