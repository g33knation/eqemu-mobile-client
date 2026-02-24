import { createCanvas } from 'canvas';
import fs from 'fs';

const width = 256;
const height = 256;
const canvas = createCanvas(width, height);
const ctx = canvas.getContext('2d');

const numSquares = 16;
const squareSize = width / numSquares;

for (let y = 0; y < numSquares; y++) {
    for (let x = 0; x < numSquares; x++) {
        if ((x + y) % 2 === 0) {
            ctx.fillStyle = '#FF0000'; // Red
        } else {
            ctx.fillStyle = '#0000FF'; // Blue
        }
        ctx.fillRect(x * squareSize, y * squareSize, squareSize, squareSize);
    }
}

const buffer = canvas.toBuffer('image/png');
fs.writeFileSync('/home/tommy/Desktop/MobileClient/web/assets/Textures/checkerboard.png', buffer);
console.log('Checkerboard generated.');
