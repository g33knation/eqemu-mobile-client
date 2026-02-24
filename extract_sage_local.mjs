import Jimp from '/home/tommy/eqsage/node_modules/jimp/es/index.js';
import dxt from '/home/tommy/eqsage/node_modules/dxt-js/src/dxt.js';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { S3DDecoder } from '/home/tommy/eqsage/sage/lib/s3d/s3d-decoder.js';
import { globals as sageGlobals, setGlobals } from '/home/tommy/eqsage/sage/lib/globals.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// --- MOCKS for Node environment ---
function createDirMock(dirPath) {
    if (!fs.existsSync(dirPath)) fs.mkdirSync(dirPath, { recursive: true });
    return {
        async getDirectoryHandle(name, opts) {
            const newPath = path.join(dirPath, name);
            return createDirMock(newPath);
        },
        async getFileHandle(name, opts) {
            const filePath = path.join(dirPath, name);
            return {
                async createWritable() {
                    return {
                        async write(data) {
                            fs.writeFileSync(filePath, Buffer.from(data));
                        },
                        async close() { },
                        getWriter() {
                            return {
                                async releaseLock() { }
                            };
                        }
                    };
                },
                async getFile() {
                    const buffer = fs.readFileSync(filePath);
                    return new File([buffer], name);
                }
            };
        },
        async removeEntry(name) {
            const entryPath = path.join(dirPath, name);
            if (fs.existsSync(entryPath)) {
                fs.rmSync(entryPath, { recursive: true, force: true });
            }
        },
        async entries() { return []; }
    };
}

const outputDir = path.join(__dirname, 'web/assets/models');
if (!fs.existsSync(outputDir)) fs.mkdirSync(outputDir, { recursive: true });

// Setup Global Mocks
global.window = {
    gameController: {
        rootFileSystemHandle: createDirMock(outputDir)
    },
    imageProcessor: {
        parseImages: async (images) => {
            console.log(`🖼️ Processing ${images.length} images...`);
            for (const img of images) {
                try {
                    let pngBuffer;
                    if (img.name.toLowerCase().endsWith('.dds')) {
                        const data = new Uint8Array(img.data);
                        // Parse DDS header (128 bytes)
                        const width = data[12] | (data[13] << 8) | (data[14] << 16) | (data[15] << 24);
                        const height = data[16] | (data[17] << 8) | (data[18] << 16) | (data[19] << 24);
                        const fourCC = String.fromCharCode(data[84], data[85], data[86], data[87]);

                        let format;
                        if (fourCC === 'DXT1') format = dxt.flags.DXT1;
                        else if (fourCC === 'DXT3') format = dxt.flags.DXT3;
                        else if (fourCC === 'DXT5') format = dxt.flags.DXT5;

                        if (format !== undefined) {
                            const rgba = dxt.decompress(data.slice(128), width, height, format);
                            // Jimp can take a buffer and width/height
                            const jimpImg = new Jimp(width, height);
                            jimpImg.bitmap.data = Buffer.from(rgba);
                            pngBuffer = await jimpImg.getBufferAsync(Jimp.MIME_PNG);
                        } else {
                            console.warn(`  ⚠️ Unsupported DDS format ${fourCC} for ${img.name}`);
                        }
                    } else if (img.name.toLowerCase().endsWith('.bmp')) {
                        const jimpImg = await Jimp.read(Buffer.from(img.data));
                        pngBuffer = await jimpImg.getBufferAsync(Jimp.MIME_PNG);
                    }

                    if (pngBuffer) {
                        img.pngData = pngBuffer;
                        console.log(`  ✅ Processed ${img.name} -> PNG (${pngBuffer.byteLength} bytes)`);
                    }
                } catch (err) {
                    console.error(`  ❌ Failed to process ${img.name}:`, err.message);
                }
            }
        }
    },
    sageGlobals: {
        GlobalStore: {
            actions: {
                setLoadingTitle: () => { },
                setLoadingText: () => { },
            }
        }
    }
};

setGlobals({
    GlobalStore: global.window.sageGlobals.GlobalStore,
    gameController: global.window.gameController
});

Object.defineProperty(global, 'navigator', { value: {}, writable: true, configurable: true });
Object.defineProperty(global, 'location', { value: { href: '' }, writable: true, configurable: true });

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
        await decoder.processS3D(fileMock, false); // skipImages=false now!
        return new S3DArchive(decoder, fileMock.name);
    }
}

class S3DArchive {
    constructor(decoder, name) {
        this.decoder = decoder;
        this.archiveName = name;
    }

    async exportToGLB(options = {}) {
        // Find first WLD file
        const wld = this.decoder.wldFiles[0];
        if (!wld) throw new Error("No WLD files found in archive");

        const name = path.basename(this.archiveName, '.s3d');

        // Export Skinner Meshes (Characters usually use this)
        // We'll just export the first skeleton we find for now
        const skeleton = wld.skeletons[0];
        if (!skeleton) {
            console.log("No skeleton found, exporting as static mesh...");
            return null;
        }

        console.log(`📦 Exporting GLB for ${name} (Skeleton: ${skeleton.modelBase})...`);

        // Prep the target WLD: Parse tracks and assign to skeletons
        await this.decoder.exportModels(wld, false);

        const glbData = await this.decoder.exportSkinnedMeshes(
            wld,
            wld.meshes,
            name,
            skeleton,
            '', // path
            true // isCharacterAnimation
        );

        return glbData;
    }
}

// --- Main Execution ---
async function extractCharacter(archiveName) {
    const eqPath = '/home/tommy/Desktop/MobileClient/tmp_assets';
    const inputFile = path.join(eqPath, `${archiveName}.s3d`);

    if (!fs.existsSync(inputFile)) {
        console.error(`❌ File not found: ${inputFile}`);
        return;
    }

    try {
        console.log(`\n🔍 Loading ${archiveName}.s3d...`);
        const archive = await S3D.load(inputFile);

        // Load global_chr for shared animations if needed
        const globalChrFile = path.join(eqPath, 'global_chr.s3d');
        if (fs.existsSync(globalChrFile)) {
            console.log(`🔍 Loading shared animations from global_chr.s3d...`);
            const globalArchive = await S3D.load(globalChrFile);
            const globalWld = globalArchive.decoder.wldFiles[0];

            // Prep the shared WLD: Parse tracks and assign to skeletons
            await globalArchive.decoder.exportModels(globalWld, false);

            archive.decoder.globalWld = globalWld;

            // Merge global textures into the model's decoder for shared heads, etc.
            await globalArchive.decoder.export(false);
            Object.assign(archive.decoder.textureMap, globalArchive.decoder.textureMap);
        }

        const glbData = await archive.exportToGLB();

        if (glbData) {
            // Rename globalbam_chr to bam for the client
            const outputName = archiveName === 'globalbam_chr' ? 'bam' : archiveName;
            const outputFile = path.join(outputDir, `${outputName}.glb`);
            fs.writeFileSync(outputFile, Buffer.from(glbData));
            console.log(`✅ Exported ${outputName}.glb to ${outputFile}`);

            console.log(`🦴 Post-export Skeletons for ${archiveName}:`);
            for (const wld of archive.decoder.wldFiles) {
                for (const skel of wld.skeletons) {
                    console.log(`      🦴 Skeleton: ${skel.modelBase} (${skel.animations ? Object.keys(skel.animations).length : 0} anims)`);
                }
            }
        }
    } catch (e) {
        console.error("💥 Extraction Failed:", e);
    }
}

const target = process.argv[2] || 'globalbam_chr';
extractCharacter(target);
