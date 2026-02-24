import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';

class GameApp {
    constructor() {
        this.scene = new THREE.Scene();
        this.camera = new THREE.PerspectiveCamera(75, window.innerWidth / window.innerHeight, 0.1, 1000);
        this.renderer = new THREE.WebGLRenderer({ antialias: true });
        this.renderer.setSize(window.innerWidth, window.innerHeight);
        document.getElementById('canvas-container').appendChild(this.renderer.domElement);

        this.controls = new OrbitControls(this.camera, this.renderer.domElement);
        this.camera.position.set(0, 5, 10);
        this.controls.update();

        this.loader = new GLTFLoader();
        this.models = new Map();

        this.initLights();
        this.animate();
        this.connect();

        this.loadModel('bam', 'assets/models/bam.glb');

        window.addEventListener('resize', () => this.onResize());
    }

    initLights() {
        const ambientLight = new THREE.AmbientLight(0xffffff, 0.7);
        this.scene.add(ambientLight);
        const sunLight = new THREE.DirectionalLight(0xffffff, 1.2);
        sunLight.position.set(10, 20, 15);
        this.scene.add(sunLight);
    }

    async loadModel(name, path) {
        try {
            const gltf = await this.loader.loadAsync(path);
            gltf.scene.scale.set(0.1, 0.1, 0.1); // EQ models are often large
            this.scene.add(gltf.scene);
            this.models.set(name, gltf);
            console.log(`✅ Loaded Luclin model: ${name}`);

            // Auto-center camera
            const box = new THREE.Box3().setFromObject(gltf.scene);
            const center = box.getCenter(new THREE.Vector3());
            this.controls.target.copy(center);
            this.camera.position.set(center.x, center.y + 5, center.z + 10);
            this.controls.update();

        } catch (e) {
            console.error(`❌ Failed to load model: ${name}`, e);
        }
    }

    connect() {
        const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        this.socket = new WebSocket(`${wsProtocol}//${window.location.host}/ws`);

        this.socket.onopen = () => {
            document.getElementById('status').innerText = 'Connected';
            console.log('📡 WebSocket Connected');
        };

        this.socket.onmessage = (event) => {
            const data = JSON.parse(event.data);
            this.handleServerMessage(data);
        };
    }

    handleServerMessage(msg) {
        // High-level message handling
        console.log('📩 Message from server:', msg);
    }

    onResize() {
        this.camera.aspect = window.innerWidth / window.innerHeight;
        this.camera.updateProjectionMatrix();
        this.renderer.setSize(window.innerWidth, window.innerHeight);
    }

    animate() {
        requestAnimationFrame(() => this.animate());
        this.controls.update();
        this.renderer.render(this.scene, this.camera);
    }
}

// Entry point
window.addEventListener('load', () => {
    new GameApp();
});
