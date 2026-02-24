
window.auditMeshes = async (indices, rotation, flipY) => {
    const THREE = window.THREE || (await import('https://cdn.jsdelivr.net/npm/three@0.170.0/build/three.module.js'));
    const scene = window.scene || window.mainScene;
    if (!scene) {
        console.error("No scene found");
        return;
    }

    // Find the player model
    let playerModel;
    scene.traverse(node => {
        if (node.isGroup && node.children.some(c => c.isMesh && c.name.includes('globalbam'))) {
            playerModel = node;
        }
    });

    if (!playerModel) {
        console.error("Player model not found");
        return;
    }

    // Hide all, then show targets
    playerModel.traverse(node => {
        if (node.isMesh) {
            node.visible = false;
            const idxMatch = node.name.match(/\d+/);
            const idx = idxMatch ? parseInt(idxMatch[0]) : -1;
            if (indices.includes(idx)) {
                node.visible = true;
                node.scale.set(1, 1, 1);
                node.position.set(0, 0, 0);
            }
        }
    });

    // Apply texture to target mesh(es)
    const textureLoader = new THREE.TextureLoader();
    textureLoader.load('assets/Textures/bamhesk01.png', (tex) => {
        tex.magFilter = THREE.NearestFilter;
        tex.minFilter = THREE.NearestFilter;
        tex.flipY = flipY;
        tex.rotation = rotation;
        tex.center.set(0.5, 0.5);

        indices.forEach(idx => {
            const mesh = playerModel.children.find(c => c.name.includes(idx));
            if (mesh) {
                mesh.material = new THREE.MeshBasicMaterial({
                    map: tex,
                    transparent: true,
                    side: THREE.DoubleSide
                });
            }
        });
    });

    // Focus camera
    window.camDist = 4;
    window.camAngleH = 0;
    window.camAngleV = 0.5;
    console.log(`Auditing indices ${indices} with rot ${rotation}, flipY ${flipY}`);
};
