import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { Buffer } from 'buffer';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
console.log("DEBUG: extract_sage.mjs starting...");

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

    const rootPath = path.join(process.cwd(), 'web/assets/models');
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

    Object.defineProperty(global, 'navigator', { value: {}, writable: true, configurable: true });
    Object.defineProperty(global, 'location', { value: { href: '' }, writable: true, configurable: true });
} catch (e) {
    console.warn("Global mock warning:", e.message);
}

// Local eqsage library paths
const SAGE_LIB = '/home/tommy/eqsage/sage/lib';
const globalsPath = path.join(SAGE_LIB, 'globals.js');
const decoderPath = path.join(SAGE_LIB, 's3d/s3d-decoder.js');
console.log(`DEBUG: Importing globals from: ${globalsPath}`);
console.log(`DEBUG: Importing decoder from: ${decoderPath}`);

const { globals } = await import(globalsPath);
const { S3DDecoder } = await import(decoderPath);

// Re-assign globals as expected by S3DDecoder
globals.gameController = global.window.gameController;
globals.GlobalStore = global.window.sageGlobals.GlobalStore;

class S3D {
    static async load(filePath) {
        if (!fs.existsSync(filePath)) throw new Error(`File not found: ${filePath}`);
        const arrayBuffer = fs.readFileSync(filePath).buffer;
        const fileMock = {
            name: path.basename(filePath),
            arrayBuffer: async () => arrayBuffer
        };

        const decoder = new S3DDecoder(null, { forceWrite: true });
        await decoder.processS3D(fileMock, true); // skipImages=true for speed
        return new S3DArchive(decoder);
    }
}

class S3DArchive {
    constructor(decoder) {
        this.decoder = decoder;
    }

    async exportToGLB() {
        // High-level export logic
        for (const wld of this.decoder.wldFiles) {
            await this.decoder.exportModels(wld, true, 'root');
        }
    }
}

async function extractLuclinCharacter(archiveName) {
    const eqPath = '/home/tommy/Desktop/RuinsofDunscaith';
    const inputFile = path.join(eqPath, `${archiveName}.s3d`);
    const outputDir = '/home/tommy/Desktop/MobileClient/web/assets/models';

    if (!fs.existsSync(inputFile)) {
        console.error(`❌ Luclin model not found: ${inputFile}`);
        return;
    }

    console.log(`\n🔍 High-Level Luclin Extraction: ${archiveName}.s3d...`);

    try {
        const archive = await S3D.load(inputFile);
        console.log(`📦 Archive ${archiveName} loaded. WLD count: ${archive.decoder.wldFiles.length}`);

        // Handle shared animations
        const globalChrFile = path.join(eqPath, 'global_chr.s3d');
        if (fs.existsSync(globalChrFile)) {
            const globalArchive = await S3D.load(globalChrFile);
            console.log(`📦 Global Archive loaded. WLD count: ${globalArchive.decoder.wldFiles.length}`);
            if (globalArchive.decoder.wldFiles.length > 0) {
                const globalWld = globalArchive.decoder.wldFiles[0];
                await globalArchive.decoder.exportModels(globalWld, false);
                archive.decoder.globalWld = globalWld;
            } else {
                console.warn("⚠️ No WLD files found in global_chr.s3d");
            }
        }

        await archive.exportToGLB();
        console.log(`✅ Completed extraction for: ${archiveName}`);
    } catch (e) {
        console.error("💥 Luclin Extraction Failed:", e);
    }
}

const targets = [
    'globalbam_chr', 'globalhum_chr', 'globalerf_chr', 'globalerm_chr'
];

async function runPipeline() {
    for (const target of targets) {
        await extractLuclinCharacter(target);
    }
}

runPipeline();
