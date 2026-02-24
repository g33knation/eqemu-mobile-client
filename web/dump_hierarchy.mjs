import fs from 'fs';
import { WebIO } from '@gltf-transform/core';

async function run() {
    const file = process.argv[2] || '/home/tommy/Desktop/MobileClient/web/assets/models/bam.glb';
    const io = new WebIO();
    const doc = await io.read(file);
    const root = doc.getRoot();

    console.log("--- SCENES ---");
    for (const scene of root.listScenes()) {
        console.log(`Scene: ${scene.getName()}`);
        for (const child of scene.listChildren()) {
            printNode(child, "  ");
        }
    }
}

function printNode(node, indent) {
    let info = `${indent}- Node: ${node.getName() || 'unnamed'}`;
    const mesh = node.getMesh();
    if (mesh) {
        info += ` [MESH: ${mesh.getName() || 'unnamed'}]`;
        info += ` (Primitives: ${mesh.listPrimitives().length})`;
        for (let i = 0; i < mesh.listPrimitives().length; i++) {
            const prim = mesh.listPrimitives()[i];
            info += `\n${indent}    Prim ${i} material: ${prim.getMaterial()?.getName()}`;
            info += `\n${indent}    Prim ${i} POSITION count: ${prim.getAttribute('POSITION')?.getCount()}`;
        }
    }
    const skin = node.getSkin();
    if (skin) info += ` [SKIN: ${skin.getName() || 'unnamed'}]`;

    const extras = node.getExtras();
    if (extras && Object.keys(extras).length > 0) info += ` [EXTRAS: ${JSON.stringify(extras)}]`;

    console.log(info);

    for (const child of node.listChildren()) {
        printNode(child, indent + "  ");
    }
}

run().catch(console.error);
