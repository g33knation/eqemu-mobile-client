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
            const nextPath = path.join(dirPath, name);
            if (!fs.existsSync(nextPath)) fs.mkdirSync(nextPath, { recursive: true });
            return createDirMock(nextPath);
        },
        async getFileHandle(fileName, opts) {
            const filePath = path.join(dirPath, fileName);
            return {
                kind: 'file',
                name: fileName,
                async getFile() {
                    if (!fs.existsSync(filePath)) return { name: fileName, arrayBuffer: async () => new ArrayBuffer(0) };
                    const buffer = fs.readFileSync(filePath);
                    return { name: fileName, arrayBuffer: async () => buffer.buffer };
                },
                async createWritable() {
                    return {
                        write: async (data) => fs.writeFileSync(filePath, data),
                        getWriter: () => ({ releaseLock: () => { } }),
                        close: async () => { }
                    };
                }
            };
        },
        async removeEntry(name, opts) {
            const entryPath = path.join(dirPath, name);
            if (fs.existsSync(entryPath)) {
                fs.rmSync(entryPath, { recursive: true, force: true });
            }
        },
        async entries() { return []; }
    });

    const rootPath = path.join(__dirname, '../public/assets/chars');
    if (!fs.existsSync(rootPath)) fs.mkdirSync(rootPath, { recursive: true });

    global.window = {
        gameController: {
            rootFileSystemHandle: createDirMock(rootPath)
        },
        imageProcessor: { parseImages: async () => { } },
        sageGlobals: {
            GlobalStore: {
                actions: {
                    setLoadingTitle: () => { },
                    setLoadingText: () => { },
                }
            }
        }
    };

    // Also need to set them on the imported globals
    globals.GlobalStore = {
        actions: {
            setLoadingTitle: () => { },
            setLoadingText: () => { },
        }
    };
    globals.gameController = global.window.gameController;

    Object.defineProperty(global, 'navigator', { value: {}, writable: true, configurable: true });
    Object.defineProperty(global, 'location', { value: { href: '' }, writable: true, configurable: true });
} catch (e) {
    console.warn("Global mock warning:", e.message);
}

// Import discovered classes from the local package
// We use the full path to avoid resolution issues with ESM/CJS mix
import { S3DDecoder } from '/home/tommy/eqsage/sage/lib/s3d/s3d-decoder.js';

// --- WRAPPER to match USER'S API (@eqsage/sage v2.0 style) ---
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
        return new S3DArchive(decoder);
    }
}

class S3DArchive {
    constructor(decoder) {
        this.decoder = decoder;
    }

    async exportToGLB(options = {}) {
        // We'll capture the first GLB exported or handle it as requested
        // Note: S3DDecoder.exportModels writes directly to the mock FS we set up above
        for (const wld of this.decoder.wldFiles) {
            await this.decoder.exportModels(wld, true, 'root');
        }
        // Since the requirement is to return a buffer, we read the result back
        // (Just a placeholder logic as the real S3DDecoder writes files)
        return Buffer.from([]);
    }
}

async function extractCharacter(archiveName) {
    const eqPath = '/home/tommy/RoF2_Client';
    const inputFile = path.join(eqPath, `${archiveName}.s3d`);
    const outputDir = '/home/tommy/Desktop/MobileClient/web/assets/models';

    if (!fs.existsSync(inputFile)) {
        console.error(`❌ File not found: ${inputFile}`);
        return;
    }

    // Ensure output dir exists
    if (!fs.existsSync(outputDir)) {
        fs.mkdirSync(outputDir, { recursive: true });
    }

    console.log(`🔮 Sage Extracting: ${archiveName}...`);

    try {
        const archive = await S3D.load(inputFile);
        console.log(`📦 Archive loaded. WLD count: ${archive.decoder.wldFiles.length}`);

        for (const wld of archive.decoder.wldFiles) {
            console.log(`  📄 WLD: ${wld.name}`);

            // Extract all BAMHE meshes
            const headMeshes = wld.meshes.filter(m => m && m.name && m.name.toUpperCase().includes('BAMHE00'));
            if (headMeshes.length > 0) {
                console.log(`Injecting fake skeleton for ${headMeshes.length} head meshes...`);
                // Inject a fake skeleton so exportModels processes it
                wld.skeletons.push({
                    modelBase: 'bamhe00_dmspritedef',
                    meshes: headMeshes,
                    secondaryMeshes: [],
                    animations: { pos: { tracksCleanedStripped: [] } },
                    skeleton: [{
                        index: 0,
                        name: 'dummy_root',
                        cleanedName: 'dummy_root',
                        parent: null,
                        children: []
                    }],
                    buildSkeletonData: () => { }
                });
            }

            console.log(`    - Skeletons: ${wld.skeletons.length}`);
            console.log(`    - Meshes: ${wld.meshes.length}`);
            console.log(`    - Tracks: ${wld.tracks.length}`);

            for (const skel of wld.skeletons) {
                console.log(`      🦴 Skeleton: ${skel.modelBase} (${skel.animations ? Object.keys(skel.animations).length : 0} anims)`);
            }
        }

        await archive.exportToGLB({
            includeAnimations: true,
            embedTextures: true
        });

        console.log(`✅ Completed extraction for archive: ${archiveName}`);
    } catch (e) {
        console.error("💥 Extraction Failed:", e);
    }
}

// Extract major global archives
async function extractAll() {
    await extractCharacter('global_chr');
}

extractAll();
