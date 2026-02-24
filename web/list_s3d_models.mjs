import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { Buffer } from 'buffer';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

import { globals } from '/home/tommy/eqsage/sage/lib/globals.js';

// --- BROWSER / ELECTRON MOCKS ---
try {
    const createDirMock = (dirPath) => ({
        kind: 'directory',
        name: path.basename(dirPath),
        async getDirectoryHandle(name, opts) {
            const newPath = path.join(dirPath, name);
            if (opts?.create && !fs.existsSync(newPath)) fs.mkdirSync(newPath, { recursive: true });
            return createDirMock(newPath);
        },
        async getFileHandle(fileName, options) {
            const filePath = path.join(dirPath, fileName);
            return {
                async getFile() {
                    if (!fs.existsSync(filePath)) return { name: fileName, arrayBuffer: async () => new ArrayBuffer(0) };
                    const buffer = fs.readFileSync(filePath);
                    return { name: fileName, arrayBuffer: async () => buffer.buffer };
                },
            };
        },
    });

    global.navigator = { storage: { getDirectory: async () => createDirMock('/home/tommy/Desktop/MobileClient/tmp_assets') } };
    global.window = {
        gameController: { appState: { activeEqPath: '/home/tommy/RoF2_Client' } }
    };
    global.document = {
        createElement: () => ({
            getContext: () => ({ fillRect: () => { }, getImageData: () => ({ data: new Uint8Array(4) }) })
        })
    };
    global.fetch = async (url) => { throw new Error('fetch mock not implemented'); };

} catch (e) {
    console.warn("Mocks already defined or failed:", e.message);
}

// Ensure sage init
globals.GlobalStore = {
    actions: { setLoadingText: (t) => console.log(t) }
};

import { S3DDecoder } from '/home/tommy/eqsage/sage/lib/s3d/s3d-decoder.js';

class S3D {
    static async load(filePath) {
        if (!fs.existsSync(filePath)) throw new Error(`File not found: ${filePath}`);
        const arrayBuffer = fs.readFileSync(filePath).buffer;
        const fileMock = {
            name: path.basename(filePath),
            arrayBuffer: async () => arrayBuffer
        };

        const decoder = new S3DDecoder(null, { forceWrite: true });
        await decoder.processS3D(fileMock, true); // skipImages=true for speed if needed
        return decoder;
    }
}

async function listModels() {
    console.log("Loading globalbam_chr.s3d...");
    try {
        const decoder = await S3D.load('/home/tommy/RoF2_Client/globalbam_chr.s3d');
        for (const wld of decoder.wldFiles) {
            console.log(`WLD: ${wld.name}`);
            for (const [name, frag] of Object.entries(wld.fragments)) {
                if (frag.type === 'Mesh') {
                    console.log(`  Mesh: ${name}`);
                } else if (frag.type === 'Model') {
                    console.log(`  Model: ${name}`);
                }
            }
        }
    } catch (e) {
        console.error("Error:", e);
    }
}

listModels().catch(console.error);
