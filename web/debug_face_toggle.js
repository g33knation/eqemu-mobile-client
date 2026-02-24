
(function () {
    if (!window.game || !window.game.scene) {
        console.error("Game not ready");
        return;
    }

    const headVariantIds = [18, 22, 23, 24]; // Face, Ears, Features, Beard
    let meshes = {};

    // 1. Find the meshes
    window.game.scene.traverse((node) => {
        if (node.isMesh && node.name.includes("globalbam_")) {
            const id = parseInt(node.name.match(/globalbam_(\d+)/)[1]);
            if (headVariantIds.includes(id)) {
                meshes[id] = node;
            }
        }
    });

    console.log("Found meshes:", Object.keys(meshes));

    // 2. Sequential Toggle Routine
    let step = 0;
    const steps = [
        { label: "ALL_ON", action: () => Object.values(meshes).forEach(m => m.visible = true) },
        {
            label: "ONLY_18_FACE", action: () => {
                Object.values(meshes).forEach(m => m.visible = false);
                if (meshes[18]) meshes[18].visible = true;
            }
        },
        {
            label: "ONLY_23_NOSE", action: () => {
                Object.values(meshes).forEach(m => m.visible = false);
                if (meshes[23]) meshes[23].visible = true;
            }
        },
        {
            label: "ONLY_24_BEARD", action: () => {
                Object.values(meshes).forEach(m => m.visible = false);
                if (meshes[24]) meshes[24].visible = true;
            }
        },
        {
            label: "18_AND_23", action: () => {
                Object.values(meshes).forEach(m => m.visible = false);
                if (meshes[18]) meshes[18].visible = true;
                if (meshes[23]) meshes[23].visible = true;
            }
        },
        {
            label: "18_AND_24", action: () => {
                Object.values(meshes).forEach(m => m.visible = false);
                if (meshes[18]) meshes[18].visible = true;
                if (meshes[24]) meshes[24].visible = true;
            }
        },
        {
            label: "DEBUG_COLORS", action: () => {
                // Apply bright distinct colors to identify them easily
                Object.values(meshes).forEach(m => m.visible = true);
                if (meshes[18]) meshes[18].material.color.setHex(0xffaaaa); // Red tint
                if (meshes[23]) meshes[23].material.color.setHex(0xaaffaa); // Green tint
                if (meshes[24]) meshes[24].material.color.setHex(0xaaaaff); // Blue tint
            }
        }
    ];

    window.debugStep = 0;
    window.debugRoutine = () => {
        if (window.debugStep < steps.length) {
            const current = steps[window.debugStep];
            console.log(`Running Debug Step: ${current.label}`);
            current.action();
            window.lastDebugLabel = current.label;
            window.debugStep++;
            return current.label;
        } else {
            return "DONE";
        }
    };
})();
