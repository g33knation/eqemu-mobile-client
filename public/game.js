import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';

// ============================================================================
// EQ Race ID → Luclin GLB Model Code Mapping
// Gender: 0 = Male, 1 = Female
// Model codes are 3-char: 2-char race prefix + m/f gender suffix
// ============================================================================
const RACE_MODEL_MAP = {
    // Playable Races (Luclin-era models available)
    1: { m: 'hum', f: 'huf', name: 'Human' },
    2: { m: 'bam', f: 'baf', name: 'Barbarian' },
    3: { m: 'erm', f: 'erf', name: 'Erudite' },
    4: { m: 'elm', f: 'elf', name: 'Wood Elf' },
    5: { m: 'elm', f: 'elf', name: 'High Elf' },       // shares elf body
    6: { m: 'dam', f: 'daf', name: 'Dark Elf' },
    7: { m: 'ham', f: 'haf', name: 'Half Elf' },
    8: { m: 'dwm', f: 'dwf', name: 'Dwarf' },
    9: { m: 'trm', f: 'trf', name: 'Troll' },
    10: { m: 'ogm', f: 'ogf', name: 'Ogre' },
    11: { m: 'ham', f: 'haf', name: 'Halfling' },        // shares halfling body
    12: { m: 'gnm', f: 'gnf', name: 'Gnome' },
    128: { m: 'ikm', f: 'ikf', name: 'Iksar' },
    130: { m: 'vam', f: 'vaf', name: 'Vah Shir' },
    330: { m: 'frm', f: 'frf', name: 'Froglok' },
    522: { m: 'drm', f: 'drf', name: 'Drakkin' },

    // Common NPC Races (use best available model)
    14: { m: 'wlm', f: 'wlm', name: 'Werewolf' },
    21: { m: 'bet', f: 'bet', name: 'Beetle' },
    26: { m: 'bat', f: 'bat', name: 'Bat' },
    42: { m: 'gnm', f: 'gnf', name: 'Gnoll' },
    43: { m: 'com', f: 'com', name: 'Combine' },
    46: { m: 'fis', f: 'fis', name: 'Fish' },
    49: { m: 'hum', f: 'huf', name: 'Guard' },
    56: { m: 'coc', f: 'coc', name: 'Cockatrice' },
    60: { m: 'skm', f: 'skf', name: 'Skeleton' },
    63: { m: 'spe', f: 'spe', name: 'Spectre' },
    69: { m: 'ogm', f: 'ogm', name: 'Golem' },
    75: { m: 'drm', f: 'drm', name: 'Drake' },
    77: { m: 'elm', f: 'elf', name: 'Fairy' },
    85: { m: 'spe', f: 'spe', name: 'Ghost' },
    93: { m: 'hum', f: 'huf', name: 'Merchant' },
    94: { m: 'hum', f: 'huf', name: 'Felguard' },
    106: { m: 'hum', f: 'huf', name: 'Banker' },
    120: { m: 'hum', f: 'huf', name: 'Shopkeeper' },
    127: { m: 'hum', f: 'huf', name: 'Invisible Man' },
    131: { m: 'hum', f: 'huf', name: 'Freeport Citizen' },
    240: { m: 'bam', f: 'baf', name: 'Halas Citizen' },
    330: { m: 'frm', f: 'frf', name: 'Froglok' },
};

// EQ Animation code mapping
const EQ_ANIMS = {
    idle: ['pos'],
    fidget: ['l01', 'l02', 'l03', 'l04', 'l05', 'l06', 'l07', 'l08', 'l09'],
    walk: ['p01'],
    run: ['p02', 'p03'],
    combat: ['c01', 'c02', 'c03', 'c04', 'c05', 'c06', 'c07', 'c08', 'c09', 'c10', 'c11'],
    death: ['d01', 'd02', 'd04', 'd05'],
    cast: ['s01', 's02', 's03', 's04'],
    emote: ['o01'],
    turn: ['t02', 't03', 't04', 't05', 't06', 't07', 't08', 't09'],
};

// Resolve race + gender to a GLB path
function getModelPath(raceId, gender) {
    const entry = RACE_MODEL_MAP[raceId];
    if (entry) {
        const code = (gender === 1) ? entry.f : entry.m;
        return `assets/mobile_ready/${code}.glb`;
    }
    // Fallback to human
    return (gender === 1) ? 'assets/mobile_ready/huf.glb' : 'assets/mobile_ready/hum.glb';
}

function getRaceName(raceId) {
    return RACE_MODEL_MAP[raceId]?.name || `Race ${raceId}`;
}

class GameApp {
    constructor() {
        this.socket = null;
        this.currentPhase = 'Login';
        this.loader = new GLTFLoader();
        this.clock = new THREE.Clock();

        // Player model
        this.playerGroup = null;
        this.playerMixer = null;
        this.playerAnims = {};
        this.currentAnim = null;

        // Spawn tracking: spawnId → { group, mixer, anims, data }
        this.spawnEntities = new Map();
        // Cache loaded GLTFs by model code to avoid re-downloading
        this.modelCache = new Map();
        // All mixers for animation updates
        this.mixers = [];

        // Last known state for diffing
        this.lastSpawnIds = new Set();

        // Screens
        this.screens = {
            'Login': document.getElementById('login-screen'),
            'ServerSelect': document.getElementById('server-screen'),
            'CharSelect': document.getElementById('character-screen'),
            'InWorld': document.getElementById('game-screen')
        };

        this.initUI();
        this.connect();

        window.addEventListener('resize', () => this.onResize());
    }

    initUI() {
        document.getElementById('btn-login').addEventListener('click', () => {
            const user = document.getElementById('login-user').value;
            const pass = document.getElementById('login-pass').value;
            this.send({ type: 'login', user, pass });
        });

        const createModal = document.getElementById('create-modal');
        document.getElementById('btn-show-create').addEventListener('click', () => {
            createModal.style.display = 'flex';
        });
        document.getElementById('btn-create-cancel').addEventListener('click', () => {
            createModal.style.display = 'none';
        });
        document.getElementById('btn-create-submit').addEventListener('click', () => {
            const name = document.getElementById('create-name').value;
            const race = parseInt(document.getElementById('create-race').value);
            const classId = parseInt(document.getElementById('create-class').value);
            if (!name) return alert('Name required');
            this.send({
                type: 'create_character', name, race, class: classId,
                gender: 0, hairstyle: 0, haircolor: 0, beard: 0, beardcolor: 0,
                face: 0, eye_color1: 0, eye_color2: 0, deity: 1, start_zone: 0,
                stats: [10, 10, 10, 10, 10, 10, 10]
            });
            createModal.style.display = 'none';
        });
    }

    showScreen(phase) {
        console.log(`Transitioning to phase: ${phase}`);
        Object.values(this.screens).forEach(s => s.classList.remove('active'));
        if (this.screens[phase]) {
            this.screens[phase].classList.add('active');
            this.currentPhase = phase;
        }
        if (phase === 'InWorld' && !this.renderer) {
            this.initThreeJS();
        }
    }

    initThreeJS() {
        this.scene = new THREE.Scene();
        this.scene.background = new THREE.Color(0x1a1a2e);
        this.scene.fog = new THREE.Fog(0x1a1a2e, 80, 300);
        this.camera = new THREE.PerspectiveCamera(60, window.innerWidth / window.innerHeight, 0.1, 500);
        this.renderer = new THREE.WebGLRenderer({ antialias: true });
        this.renderer.setSize(window.innerWidth, window.innerHeight);
        this.renderer.shadowMap.enabled = true;
        this.renderer.shadowMap.type = THREE.PCFSoftShadowMap;
        this.renderer.toneMapping = THREE.ACESFilmicToneMapping;
        this.renderer.toneMappingExposure = 1.2;
        document.getElementById('canvas-container').appendChild(this.renderer.domElement);

        this.controls = new OrbitControls(this.camera, this.renderer.domElement);
        this.camera.position.set(0, 5, 10);
        this.controls.target.set(0, 1.5, 0);
        this.controls.minDistance = 2;
        this.controls.maxDistance = 80;
        this.controls.update();

        this.initLights();
        this.addGround();
        this.animate();
    }

    initLights() {
        this.scene.add(new THREE.AmbientLight(0xaabbdd, 0.6));
        const sun = new THREE.DirectionalLight(0xffeedd, 1.5);
        sun.position.set(10, 20, 15);
        sun.castShadow = true;
        sun.shadow.mapSize.set(2048, 2048);
        sun.shadow.camera.near = 0.5;
        sun.shadow.camera.far = 100;
        sun.shadow.camera.left = -30;
        sun.shadow.camera.right = 30;
        sun.shadow.camera.top = 30;
        sun.shadow.camera.bottom = -30;
        this.scene.add(sun);
        const fill = new THREE.DirectionalLight(0x4466aa, 0.4);
        fill.position.set(-10, 5, -10);
        this.scene.add(fill);
    }

    addGround() {
        const geo = new THREE.PlaneGeometry(400, 400, 64, 64);
        const mat = new THREE.MeshStandardMaterial({ color: 0x2d5a27, roughness: 0.9 });
        const ground = new THREE.Mesh(geo, mat);
        ground.rotation.x = -Math.PI / 2;
        ground.receiveShadow = true;
        this.scene.add(ground);
    }

    createFallbackCharacter(isNpc = false) {
        const group = new THREE.Group();
        const color = isNpc ? 0xcc6644 : 0x6688cc;
        const body = new THREE.Mesh(
            new THREE.CapsuleGeometry(0.3, 1.0, 8, 16),
            new THREE.MeshStandardMaterial({ color, roughness: 0.6, metalness: 0.2 })
        );
        body.position.y = 0.9;
        body.castShadow = true;
        group.add(body);
        const head = new THREE.Mesh(
            new THREE.SphereGeometry(0.22, 16, 16),
            new THREE.MeshStandardMaterial({ color: 0xddaa88, roughness: 0.7 })
        );
        head.position.y = 1.8;
        head.castShadow = true;
        group.add(head);
        return group;
    }

    // Load a GLB model, using cache to avoid duplicate downloads
    async loadGLB(modelPath) {
        if (this.modelCache.has(modelPath)) {
            return this.modelCache.get(modelPath);
        }

        try {
            const gltf = await this.loader.loadAsync(modelPath + '?v=' + Date.now());
            this.modelCache.set(modelPath, gltf);
            return gltf;
        } catch (e) {
            console.warn(`⚠️ Failed to load ${modelPath}: ${e.message}`);
            return null;
        }
    }

    // Create a spawn entity from a GLTF, with its own mixer and animations
    createEntityFromGLTF(gltf) {
        const group = new THREE.Group();
        const sceneClone = gltf.scene.clone(true);

        sceneClone.traverse((child) => {
            if (child.isMesh) {
                child.castShadow = true;
                if (child.material) {
                    child.material = child.material.clone();
                    child.material.side = THREE.DoubleSide;
                }
            }
        });

        group.add(sceneClone);

        let mixer = null;
        const anims = {};
        if (gltf.animations && gltf.animations.length > 0) {
            mixer = new THREE.AnimationMixer(sceneClone);
            this.mixers.push(mixer);
            for (const clip of gltf.animations) {
                anims[clip.name] = clip;
            }
            // Play idle by default
            const idleClip = anims['pos'];
            if (idleClip) {
                const action = mixer.clipAction(idleClip);
                action.play();
            }
        }

        return { group, mixer, anims };
    }

    playAnimForEntity(entity, category, loop = true) {
        const codes = EQ_ANIMS[category];
        if (!codes || !entity.mixer) return null;
        const code = codes[Math.floor(Math.random() * codes.length)];
        const clip = entity.anims[code];
        if (!clip) return null;

        if (entity.currentAction) entity.currentAction.fadeOut(0.3);
        const action = entity.mixer.clipAction(clip);
        action.reset();
        action.setLoop(loop ? THREE.LoopRepeat : THREE.LoopOnce);
        action.clampWhenFinished = !loop;
        action.fadeIn(0.3);
        action.play();
        entity.currentAction = action;
        return action;
    }

    // Sync all spawns from server state
    async syncSpawns(spawns, mySpawnId) {
        if (!this.scene) return;

        const currentIds = new Set(Object.keys(spawns).map(Number));

        // Remove despawned entities
        for (const [id, entity] of this.spawnEntities) {
            if (!currentIds.has(id)) {
                this.scene.remove(entity.group);
                if (entity.mixer) {
                    const idx = this.mixers.indexOf(entity.mixer);
                    if (idx >= 0) this.mixers.splice(idx, 1);
                }
                this.spawnEntities.delete(id);
            }
        }

        // Add new spawns and update positions
        for (const [idStr, spawn] of Object.entries(spawns)) {
            const id = Number(idStr);
            const isMe = (id === mySpawnId);

            if (this.spawnEntities.has(id)) {
                // Update position of existing spawn
                const entity = this.spawnEntities.get(id);
                const group = entity.group;
                // Smooth lerp toward target
                group.position.lerp(new THREE.Vector3(spawn.x, 0, spawn.y), 0.15);
                // EQ heading: 0-4096 → radians
                const targetRot = -(spawn.heading / 512) * Math.PI;
                group.rotation.y += (targetRot - group.rotation.y) * 0.1;

                // If this is me, track camera
                if (isMe) {
                    this.controls.target.set(group.position.x, 1.5, group.position.z);
                }
            } else {
                // New spawn — load its model
                const modelPath = getModelPath(spawn.race, 0); // Default to male for now
                const gltf = await this.loadGLB(modelPath);

                let entity;
                if (gltf) {
                    entity = this.createEntityFromGLTF(gltf);
                } else {
                    // Fallback capsule
                    entity = {
                        group: this.createFallbackCharacter(spawn.is_npc),
                        mixer: null,
                        anims: {}
                    };
                }

                entity.data = spawn;
                entity.group.position.set(spawn.x, 0, spawn.y);
                this.scene.add(entity.group);
                this.spawnEntities.set(id, entity);

                // Add name label
                this.addNameLabel(entity.group, spawn.name, isMe, spawn.is_npc);

                const raceName = getRaceName(spawn.race);
                console.log(`🧍 Spawned ${spawn.name} (${raceName}) at (${spawn.x.toFixed(0)}, ${spawn.y.toFixed(0)}, ${spawn.z.toFixed(0)})`);

                // If this is me, move camera
                if (isMe) {
                    this.camera.position.set(spawn.x, 5, spawn.y + 10);
                    this.controls.target.set(spawn.x, 1.5, spawn.y);
                    this.controls.update();
                }
            }
        }
    }

    addNameLabel(group, name, isMe, isNpc) {
        const canvas = document.createElement('canvas');
        const ctx = canvas.getContext('2d');
        canvas.width = 256;
        canvas.height = 64;
        ctx.clearRect(0, 0, 256, 64);
        ctx.font = 'bold 28px Inter, sans-serif';
        ctx.textAlign = 'center';
        ctx.fillStyle = isMe ? '#44ff88' : (isNpc ? '#ffaa44' : '#88bbff');
        ctx.fillText(name, 128, 40);

        const texture = new THREE.CanvasTexture(canvas);
        const spriteMat = new THREE.SpriteMaterial({ map: texture, transparent: true, depthTest: false });
        const sprite = new THREE.Sprite(spriteMat);
        sprite.position.y = 2.5;
        sprite.scale.set(3, 0.75, 1);
        group.add(sprite);
    }

    connect() {
        const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        this.socket = new WebSocket(`${wsProtocol}//${window.location.host}/ws`);

        this.socket.onopen = () => {
            document.getElementById('loading-overlay').style.display = 'none';
            console.log('📡 WebSocket Connected');
        };

        this.socket.onmessage = (event) => {
            const data = JSON.parse(event.data);
            this.handleServerMessage(data);
        };

        this.socket.onerror = (err) => {
            console.error('WebSocket Error:', err);
        };
    }

    send(pkt) {
        if (this.socket && this.socket.readyState === WebSocket.OPEN) {
            this.socket.send(JSON.stringify(pkt));
        }
    }

    handleServerMessage(msg) {
        // Update phase
        if (msg.phase && msg.phase !== this.currentPhase) {
            this.showScreen(msg.phase);
        }

        // Populate lists
        if (msg.server_list && this.currentPhase === 'ServerSelect') {
            this.renderServerList(msg.server_list);
        }
        if (msg.char_list && this.currentPhase === 'CharSelect') {
            this.renderCharList(msg.char_list);
        }

        // Update world state
        if (this.currentPhase === 'InWorld') {
            document.getElementById('player-name').innerText = msg.my_name || 'In World';
            document.getElementById('player-zone').innerText = msg.zone_name || 'Loading zone...';

            // Sync spawns from server state
            if (msg.spawns) {
                this.syncSpawns(msg.spawns, msg.my_spawn_id);
            }

            // Update player position from profile
            if (msg.player && this.playerGroup) {
                this.playerGroup.position.set(msg.player.x, 0, msg.player.y);
            }
        }
    }

    renderServerList(servers) {
        const list = document.getElementById('server-list');
        list.innerHTML = '';
        servers.forEach(srv => {
            const el = document.createElement('div');
            el.className = 'list-item';
            el.innerHTML = `
                <div class="name">${srv.name}</div>
                <div class="status ${srv.status > 0 ? 'status-online' : 'status-offline'}">
                    ${srv.status > 0 ? 'Online' : 'Offline'}
                </div>
            `;
            el.onclick = () => this.send({ type: 'select_server', id: srv.id });
            list.appendChild(el);
        });
    }

    renderCharList(chars) {
        const list = document.getElementById('char-list');
        list.innerHTML = '';
        chars.forEach(c => {
            const el = document.createElement('div');
            el.className = 'list-item';
            el.innerHTML = `
                <div class="name">${c.name}</div>
                <div class="details">Lvl ${c.level} | ${getRaceName(c.race)}</div>
            `;
            el.onclick = () => this.send({ type: 'select_character', index: c.index });
            list.appendChild(el);
        });
    }

    onResize() {
        if (this.camera && this.renderer) {
            this.camera.aspect = window.innerWidth / window.innerHeight;
            this.camera.updateProjectionMatrix();
            this.renderer.setSize(window.innerWidth, window.innerHeight);
        }
    }

    animate() {
        requestAnimationFrame(() => this.animate());
        const delta = this.clock.getDelta();
        // Update all animation mixers
        for (const mixer of this.mixers) {
            mixer.update(delta);
        }
        if (this.controls && this.renderer) {
            this.controls.update();
            this.renderer.render(this.scene, this.camera);
        }
    }
}

window.addEventListener('load', () => {
    new GameApp();
});
