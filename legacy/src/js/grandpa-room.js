// ==================== Canvas Setup ====================
const canvas = document.getElementById('game-canvas');
const ctx = canvas.getContext('2d');

function resizeCanvas() {
    canvas.width = window.innerWidth;
    canvas.height = window.innerHeight;
}
resizeCanvas();
window.addEventListener('resize', resizeCanvas);

// ==================== Game State ====================
const GameState = {
    GRANDPA_CABIN: 'grandpa_cabin',
    CRYPTOPOLIS_MENU: 'cryptopolis_menu',
    GAME_WORLD: 'game_world'
};

let currentState = GameState.GRANDPA_CABIN;
let gameStarted = false;
let hasReadLetter = false; // Track if player has read the letter

// ==================== Pixel Art Settings ====================
const PIXEL = 6; // Pixel block size
const CHAR_SIZE = 28; // Character size in pixels (adjusted for better proportion)

// ==================== Player Character ====================
const player = {
    x: window.innerWidth / 2,
    y: window.innerHeight - 200,
    width: CHAR_SIZE * PIXEL,
    height: CHAR_SIZE * PIXEL,
    speed: 10,
    direction: 'down',
    animFrame: 0,
    animTimer: 0,
    isMoving: false
};

// ==================== Old CRT Computer ====================
const computer = {
    x: 0,
    y: 0,
    width: 0,
    height: 0,
    interactRadius: 100
};

function updateComputerPosition() {
    computer.width = 24 * PIXEL;
    computer.height = 22 * PIXEL;
    computer.x = (canvas.width - computer.width) / 2;
    computer.y = canvas.height / 2 - 220;
}
updateComputerPosition();

// ==================== Drawer/Cabinet ====================
const drawer = {
    x: 0,
    y: 0,
    width: 120,
    height: 140,
    interactRadius: 80
};

function updateDrawerPosition() {
    drawer.x = 80;
    drawer.y = 120;
}
updateDrawerPosition();

// Collision boxes for furniture
let collisionRects = [];

function updateCollisionRects() {
    const w = canvas.width;
    const h = canvas.height;

    collisionRects = [
        // Bookshelf (top-left, next to drawer) - only block upper portion, allow walking in front
        { x: 200, y: 120, width: 160, height: 80 },
        // Drawer (top-left corner) - only block the very top part, allow player to stand in front
        { x: drawer.x, y: drawer.y, width: drawer.width, height: 20 },
        // Bed right
        { x: w - 250, y: h/2 - 80, width: 200, height: 150 },
        // Computer (only monitor part, leaving desk area accessible)
        { x: computer.x + 10, y: computer.y, width: computer.width - 20, height: computer.height * 0.4 },
        // Top wall area
        { x: 0, y: 0, width: w, height: 100 }
    ];
}
updateCollisionRects();

// ==================== Keyboard Input ====================
const keys = {};

window.addEventListener('keydown', (e) => {
    keys[e.key] = true;
    if (e.key === 'e' || e.key === 'E') {
        checkInteraction();
    }
});

window.addEventListener('keyup', (e) => {
    keys[e.key] = false;
});

// ==================== Draw Character Sprites ====================
// Direction configurations for dynamic rendering
const CHAR_CONFIGS = {
    'down': {
        hairPos: [8, 4, 8, 4],
        headPos: [9, 6, 6, 5],
        eyes: [[10, 8], [13, 8]],
        bodyPos: [9, 11, 6, 5],
        arms: [[7, 12, 2, 4], [15, 12, 2, 4]],
        letterPos: [15, 13, 3, 4],
        legs: [[10, 16], [12, 16]],
        shoes: [[10, 19], [12, 19]]
    },
    'up': {
        hairPos: [8, 4, 8, 5],
        headPos: [9, 6, 6, 5],
        eyes: null, // back view, no eyes
        bodyPos: [9, 11, 6, 5],
        arms: [[7, 12, 2, 4], [15, 12, 2, 4]],
        letterPos: [6, 13, 3, 4],
        legs: [[10, 16], [12, 16]],
        shoes: [[10, 19], [12, 19]]
    },
    'left': {
        hairPos: [7, 4, 8, 4],
        headPos: [8, 6, 6, 5],
        eyes: [[9, 8]],
        bodyPos: [8, 11, 6, 5],
        arms: [[6, 12, 2, 4], [14, 12, 1, 3]],
        letterPos: [4, 13, 3, 4],
        legs: [[9, 16], [11, 16]],
        shoes: [[9, 19], [11, 19]]
    },
    'right': {
        hairPos: [9, 4, 8, 4],
        headPos: [10, 6, 6, 5],
        eyes: [[14, 8]],
        bodyPos: [10, 11, 6, 5],
        arms: [[9, 12, 1, 3], [16, 12, 2, 4]],
        letterPos: [17, 13, 3, 4],
        legs: [[11, 16], [13, 16]],
        shoes: [[11, 19], [13, 19]]
    },
    'up-left': {
        hairPos: [7, 4, 8, 5],
        headPos: [8, 6, 6, 5],
        eyes: [[9, 8]],
        bodyPos: [8, 11, 6, 5],
        arms: [[6, 12, 2, 4], [14, 12, 1, 3]],
        letterPos: [4, 13, 3, 4],
        legs: [[9, 16], [11, 16]],
        shoes: [[9, 19], [11, 19]]
    },
    'up-right': {
        hairPos: [9, 4, 8, 5],
        headPos: [10, 6, 6, 5],
        eyes: [[14, 8]],
        bodyPos: [10, 11, 6, 5],
        arms: [[10, 12, 1, 3], [16, 12, 2, 4]],
        letterPos: [17, 13, 3, 4],
        legs: [[11, 16], [13, 16]],
        shoes: [[11, 19], [13, 19]]
    },
    'down-left': {
        hairPos: [7, 4, 8, 4],
        headPos: [8, 6, 6, 5],
        eyes: [[9, 8], [11, 8]],
        bodyPos: [8, 11, 6, 5],
        arms: [[6, 12, 2, 4], [14, 12, 1, 3]],
        letterPos: [4, 13, 3, 4],
        legs: [[9, 16], [11, 16]],
        shoes: [[9, 19], [11, 19]]
    },
    'down-right': {
        hairPos: [9, 4, 8, 4],
        headPos: [10, 6, 6, 5],
        eyes: [[12, 8], [15, 8]],
        bodyPos: [10, 11, 6, 5],
        arms: [[10, 12, 1, 3], [16, 12, 2, 4]],
        letterPos: [17, 13, 3, 4],
        legs: [[11, 16], [13, 16]],
        shoes: [[11, 19], [13, 19]]
    }
};

function drawPlayer() {
    const px = Math.floor(player.x - player.width/2);
    const py = Math.floor(player.y - player.height/2);
    const p = PIXEL;
    const walk = player.isMoving && player.animFrame === 1 ? p : 0;

    const config = CHAR_CONFIGS[player.direction] || CHAR_CONFIGS['down'];
    drawCharacter(px, py, walk, config);
}

function drawCharacter(px, py, walk, cfg) {
    const p = PIXEL;

    // Hair
    ctx.fillStyle = '#4a3a2a';
    ctx.fillRect(px + p*cfg.hairPos[0], py + p*cfg.hairPos[1], p*cfg.hairPos[2], p*cfg.hairPos[3]);

    // Head
    ctx.fillStyle = '#d4a574';
    ctx.fillRect(px + p*cfg.headPos[0], py + p*cfg.headPos[1], p*cfg.headPos[2], p*cfg.headPos[3]);

    // Eyes
    if (cfg.eyes) {
        ctx.fillStyle = '#2a2a2a';
        cfg.eyes.forEach(eye => {
            ctx.fillRect(px + p*eye[0], py + p*eye[1], p*1, p*1);
        });
    }

    // Body
    ctx.fillStyle = '#5a6a7a';
    ctx.fillRect(px + p*cfg.bodyPos[0], py + p*cfg.bodyPos[1], p*cfg.bodyPos[2], p*cfg.bodyPos[3]);

    // Arms
    ctx.fillStyle = '#d4a574';
    cfg.arms.forEach(arm => {
        ctx.fillRect(px + p*arm[0], py + p*arm[1], p*arm[2], p*arm[3]);
    });

    // Letter in hand (if read)
    if (hasReadLetter && cfg.letterPos) {
        ctx.fillStyle = '#f5f5dc';
        ctx.fillRect(px + p*cfg.letterPos[0], py + p*cfg.letterPos[1], p*cfg.letterPos[2], p*cfg.letterPos[3]);
        ctx.strokeStyle = '#8a7a5a';
        ctx.lineWidth = 1;
        ctx.strokeRect(px + p*cfg.letterPos[0], py + p*cfg.letterPos[1], p*cfg.letterPos[2], p*cfg.letterPos[3]);
    }

    // Legs
    ctx.fillStyle = '#3a4a5a';
    ctx.fillRect(px + p*cfg.legs[0][0], py + p*cfg.legs[0][1] - walk, p*2, p*4 + walk);
    ctx.fillRect(px + p*cfg.legs[1][0], py + p*cfg.legs[1][1] + walk, p*2, p*4 - walk);

    // Shoes
    ctx.fillStyle = '#2a2a1a';
    ctx.fillRect(px + p*cfg.shoes[0][0], py + p*cfg.shoes[0][1] - (cfg.shoes[0][1] < cfg.shoes[1][1] ? walk : 0), p*2, p*1);
    ctx.fillRect(px + p*cfg.shoes[1][0], py + p*cfg.shoes[1][1] + (cfg.shoes[1][1] > cfg.shoes[0][1] ? walk : 0), p*2, p*1);
}

// ==================== Draw Wooden Cabin ====================
function drawGrandpaCabin() {
    const w = canvas.width;
    const h = canvas.height;

    // Wooden floor
    ctx.fillStyle = '#5a4a3a';
    ctx.fillRect(0, 0, w, h);

    // Floor planks
    ctx.strokeStyle = '#4a3a2a';
    ctx.lineWidth = 3;
    for (let y = 0; y < h; y += PIXEL * 8) {
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(w, y);
        ctx.stroke();
    }

    // Wooden walls
    ctx.fillStyle = '#6a5a4a';
    ctx.fillRect(0, 0, w, 100);
    ctx.fillRect(0, 0, 80, h);
    ctx.fillRect(w - 80, 0, 80, h);

    // Wall planks
    ctx.strokeStyle = '#5a4a3a';
    ctx.lineWidth = 2;
    for (let x = 0; x < w; x += PIXEL * 6) {
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, 100);
        ctx.stroke();
    }

    // Window
    drawWindow(w - 200, 20);

    // Rug under desk
    drawRug(computer.x - 60, computer.y + computer.height - 20, computer.width + 120, 140);

    // Desk
    drawDesk();

    // Collect all objects with Y positions for z-ordering
    const objects = [
        { y: 120, draw: () => drawDrawer() }, // Drawer top-left
        { y: 120, draw: () => drawBookshelf(200, 120) }, // Bookshelf next to drawer
        { y: h/2 - 80, draw: () => drawBed(w - 250, h/2 - 80) }, // Bed
        { y: computer.y + computer.height * 0.75, draw: () => drawCRTComputer() }, // Computer
        { y: player.y, draw: () => drawPlayer() } // Player
    ];

    // Sort by Y position (back to front)
    objects.sort((a, b) => a.y - b.y);

    // Draw all objects in order
    objects.forEach(obj => obj.draw());
}

function drawWindow(x, y) {
    const size = PIXEL * 12;
    // Frame
    ctx.fillStyle = '#4a3a2a';
    ctx.fillRect(x - 4, y - 4, size + 8, size + 8);
    // Glass
    const gradient = ctx.createLinearGradient(x, y, x, y + size);
    gradient.addColorStop(0, '#87CEEB');
    gradient.addColorStop(1, '#B0E0E6');
    ctx.fillStyle = gradient;
    ctx.fillRect(x, y, size, size);
    // Cross
    ctx.fillStyle = '#3a2a1a';
    ctx.fillRect(x + size/2 - 2, y, 4, size);
    ctx.fillRect(x, y + size/2 - 2, size, 4);
}

function drawBookshelf(x, y) {
    const w = 160;
    const h = 200;
    // Shelf body
    ctx.fillStyle = '#5a4a3a';
    ctx.fillRect(x, y, w, h);
    // Shelves
    ctx.fillStyle = '#6a5a4a';
    for (let i = 0; i <= 4; i++) {
        ctx.fillRect(x, y + i * (h/4), w, 4);
    }
    // Books
    const bookColors = ['#8a5a4a', '#5a7a6a', '#7a6a8a', '#6a5a7a'];
    for (let shelf = 0; shelf < 4; shelf++) {
        for (let book = 0; book < 8; book++) {
            ctx.fillStyle = bookColors[(shelf + book) % 4];
            ctx.fillRect(x + 8 + book * 18, y + 8 + shelf * (h/4), 15, h/4 - 12);
        }
    }
}

function drawBed(x, y) {
    const w = 200;
    const h = 150;
    // Bed frame
    ctx.fillStyle = '#5a4a3a';
    ctx.fillRect(x, y + h - 30, w, 30);
    // Mattress
    ctx.fillStyle = '#7a6a5a';
    ctx.fillRect(x + 10, y + 30, w - 20, h - 30);
    // Pillow
    ctx.fillStyle = '#9a8a7a';
    ctx.fillRect(x + 20, y + 40, 60, 30);
    // Blanket
    ctx.fillStyle = '#6a5a4a';
    ctx.fillRect(x + 30, y + 80, w - 60, 50);
}

function drawDrawer() {
    const x = drawer.x;
    const y = drawer.y;
    const w = drawer.width;
    const h = drawer.height;
    const p = PIXEL;

    // Cabinet body
    ctx.fillStyle = '#5a4a3a';
    ctx.fillRect(x, y, w, h);

    // Cabinet top
    ctx.fillStyle = '#6a5a4a';
    ctx.fillRect(x, y, w, 8);

    // Drawer fronts
    const drawerHeight = (h - 20) / 3;
    for (let i = 0; i < 3; i++) {
        const drawerY = y + 12 + i * (drawerHeight + 4);

        // Drawer front
        ctx.fillStyle = '#6a5a4a';
        ctx.fillRect(x + 6, drawerY, w - 12, drawerHeight);

        // Drawer border
        ctx.strokeStyle = '#4a3a2a';
        ctx.lineWidth = 2;
        ctx.strokeRect(x + 6, drawerY, w - 12, drawerHeight);

        // Drawer handle
        ctx.fillStyle = '#3a2a1a';
        ctx.fillRect(x + w/2 - 8, drawerY + drawerHeight/2 - 3, 16, 6);
    }

    // Highlight if near
    if (isNearDrawer() && !hasReadLetter) {
        ctx.strokeStyle = '#8a7a5a';
        ctx.lineWidth = 3;
        ctx.strokeRect(x - 2, y - 2, w + 4, h + 4);
    }
}

function drawRug(x, y, w, h) {
    // Rug
    ctx.fillStyle = '#7a4a3a';
    ctx.fillRect(x, y, w, h);
    // Border
    ctx.strokeStyle = '#5a3a2a';
    ctx.lineWidth = 6;
    ctx.strokeRect(x + 8, y + 8, w - 16, h - 16);
}

function drawDesk() {
    const x = computer.x - 20;
    const y = computer.y + computer.height - 40;
    const w = computer.width + 40;
    const h = 100;

    // Desktop
    ctx.fillStyle = '#6a5a4a';
    ctx.fillRect(x, y, w, 12);

    // Desktop highlight
    ctx.fillStyle = '#7a6a5a';
    ctx.fillRect(x + 4, y + 2, w - 8, 4);

    // Legs
    ctx.fillStyle = '#5a4a3a';
    ctx.fillRect(x + 10, y + 12, 10, h - 12);
    ctx.fillRect(x + w - 20, y + 12, 10, h - 12);
}

function drawCRTComputer() {
    const cx = computer.x;
    const cy = computer.y;
    const p = PIXEL;

    // Big CRT Monitor (old style with depth)
    // Monitor back/depth
    ctx.fillStyle = '#4a4a4a';
    ctx.fillRect(cx, cy, computer.width, computer.height * 0.75);

    // Monitor bezel
    ctx.fillStyle = '#5a5a5a';
    ctx.fillRect(cx + p*2, cy + p*2, computer.width - p*4, computer.height * 0.75 - p*4);

    // Screen (deep inside)
    const isNear = isNearComputer();
    ctx.fillStyle = isNear ? '#2a4a5a' : '#1a1a1a';
    ctx.fillRect(cx + p*3, cy + p*3, computer.width - p*6, computer.height * 0.75 - p*6);

    // Screen glow
    if (isNear) {
        const time = Date.now() / 1000;
        const pulse = Math.sin(time * 2) * 0.3 + 0.7;
        ctx.globalAlpha = pulse * 0.4;
        ctx.fillStyle = '#4a6a8a';
        ctx.fillRect(cx + p*3, cy + p*3, computer.width - p*6, computer.height * 0.75 - p*6);
        ctx.globalAlpha = 1;

        // Scanlines
        for (let i = 0; i < 8; i++) {
            ctx.fillStyle = 'rgba(100, 150, 180, 0.2)';
            ctx.fillRect(cx + p*3, cy + p*3 + i * p*2, computer.width - p*6, p*0.5);
        }

        // Old terminal text
        ctx.fillStyle = '#5a8aaa';
        ctx.font = `${p*2}px monospace`;
        ctx.fillText('READY.', cx + p*5, cy + p*7);
    }

    // Bottom bezel with brand
    ctx.fillStyle = '#6a6a6a';
    ctx.fillRect(cx + p*3, cy + computer.height * 0.75 - p*5, computer.width - p*6, p*3);

    // Power LED
    if (isNear) {
        ctx.fillStyle = '#4aaa4a';
        ctx.fillRect(cx + p*4, cy + computer.height * 0.75 - p*3.5, p*1.5, p*1.5);
    }

    // Monitor base (old style thick base)
    ctx.fillStyle = '#3a3a3a';
    const baseW = computer.width * 0.6;
    const baseX = cx + (computer.width - baseW) / 2;
    ctx.fillRect(baseX, cy + computer.height * 0.75, baseW, p*4);

    // Keyboard
    ctx.fillStyle = '#4a4a3a';
    ctx.fillRect(cx - p*2, cy + computer.height * 0.75 + p*6, computer.width + p*4, p*3);
    // Keys
    ctx.fillStyle = '#3a3a2a';
    for (let i = 0; i < 12; i++) {
        for (let j = 0; j < 3; j++) {
            ctx.fillRect(cx + p*(i*2), cy + computer.height * 0.75 + p*7 + j*p, p*1.5, p*0.8);
        }
    }
}

// ==================== Draw Cryptopolis Menu ====================
function drawCryptopolisMenu() {
    const w = canvas.width;
    const h = canvas.height;

    // Cyberpunk sky
    const gradient = ctx.createLinearGradient(0, 0, 0, h);
    gradient.addColorStop(0, '#0a1a2a');
    gradient.addColorStop(1, '#1a2a3a');
    ctx.fillStyle = gradient;
    ctx.fillRect(0, 0, w, h);

    // Stars
    ctx.fillStyle = '#5a6a7a';
    for (let i = 0; i < 100; i++) {
        const x = (i * 1237) % w;
        const y = (i * 4567) % (h * 0.6);
        const size = (i % 3) + 2;
        ctx.fillRect(x, y, size, size);
    }

    // City skyline
    const buildingCount = Math.floor(w / 150);
    for (let i = 0; i < buildingCount; i++) {
        const bx = i * (w / buildingCount);
        const bw = w / buildingCount - 20;
        const bh = 200 + (i % 3) * 100;
        const by = h - bh - 80;

        ctx.fillStyle = i % 2 === 0 ? '#2a3a4a' : '#1a2a3a';
        ctx.fillRect(bx, by, bw, bh);

        // Windows
        ctx.fillStyle = '#4a5a6a';
        for (let row = 0; row < Math.floor(bh / 30); row++) {
            for (let col = 0; col < Math.floor(bw / 25); col++) {
                if (Math.random() > 0.4) {
                    ctx.fillRect(bx + 8 + col * 25, by + 15 + row * 30, 12, 18);
                }
            }
        }
    }

    // Ground
    ctx.fillStyle = '#0a0a1a';
    ctx.fillRect(0, h - 80, w, 80);

    // Grid
    ctx.strokeStyle = '#2a2a3a';
    ctx.lineWidth = 2;
    for (let x = 0; x < w; x += 40) {
        ctx.beginPath();
        ctx.moveTo(x, h - 80);
        ctx.lineTo(x, h);
        ctx.stroke();
    }

    // Title
    ctx.fillStyle = '#5a7a8a';
    ctx.font = 'bold 64px "Courier New"';
    ctx.textAlign = 'center';
    ctx.fillText('CRYPTOPOLIS', w / 2, h / 4);

    ctx.fillStyle = '#4a6a7a';
    ctx.font = '20px "Courier New"';
    ctx.fillText('The Digital Cypherpunk World', w / 2, h / 4 + 40);

    // Menu buttons
    drawMenuButtons();
}

function drawMenuButtons() {
    const w = canvas.width;
    const h = canvas.height;
    const centerX = w / 2;
    const startY = h / 2;
    const buttonWidth = 300;
    const buttonHeight = 60;
    const buttonSpacing = 80;

    // Continue button (only if game started)
    if (gameStarted) {
        drawButton(centerX - buttonWidth/2, startY, buttonWidth, buttonHeight, 'CONTINUE');
    }

    // New Game button
    drawButton(centerX - buttonWidth/2, startY + (gameStarted ? buttonSpacing : 0), buttonWidth, buttonHeight, 'NEW GAME', true);

    // Settings button
    drawButton(centerX - buttonWidth/2, startY + buttonSpacing * (gameStarted ? 2 : 1), buttonWidth, buttonHeight, 'SETTINGS', true);
}

function drawButton(x, y, w, h, text, disabled = false) {
    if (disabled) {
        ctx.fillStyle = '#1a2a3a';
        ctx.globalAlpha = 0.5;
    } else {
        const gradient = ctx.createLinearGradient(x, y, x, y + h);
        gradient.addColorStop(0, '#2a3a4a');
        gradient.addColorStop(1, '#1a2a3a');
        ctx.fillStyle = gradient;
    }
    ctx.fillRect(x, y, w, h);
    ctx.globalAlpha = 1;

    ctx.strokeStyle = disabled ? '#3a3a4a' : '#5a7a8a';
    ctx.lineWidth = 2;
    ctx.strokeRect(x, y, w, h);

    ctx.fillStyle = disabled ? '#5a5a6a' : '#b0b0b0';
    ctx.font = '24px "Courier New"';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(text, x + w/2, y + h/2);
}

// ==================== Draw Game World Demo ====================
function drawGameWorld() {
    const w = canvas.width;
    const h = canvas.height;

    // Simple forest background
    const gradient = ctx.createLinearGradient(0, 0, 0, h);
    gradient.addColorStop(0, '#2a3a2a');
    gradient.addColorStop(1, '#1a2a1a');
    ctx.fillStyle = gradient;
    ctx.fillRect(0, 0, w, h);

    // Trees
    for (let i = 0; i < 15; i++) {
        const tx = (i * 137) % w;
        const ty = (i * 73) % (h - 200) + 100;
        drawTree(tx, ty);
    }

    // Path
    ctx.fillStyle = '#4a3a2a';
    ctx.fillRect(w/2 - 100, 0, 200, h);

    // Player
    drawPlayer();

    // UI text
    ctx.fillStyle = '#fff';
    ctx.font = '24px "Courier New"';
    ctx.textAlign = 'center';
    ctx.fillText('GAME WORLD DEMO - Explore with WASD', w / 2, 50);
    ctx.font = '16px "Courier New"';
    ctx.fillText('(This is a placeholder for the actual game)', w / 2, 80);
}

function drawTree(x, y) {
    // Trunk
    ctx.fillStyle = '#3a2a1a';
    ctx.fillRect(x - 8, y + 20, 16, 40);
    // Leaves
    ctx.fillStyle = '#2a4a2a';
    ctx.beginPath();
    ctx.arc(x, y, 30, 0, Math.PI * 2);
    ctx.fill();
}

// ==================== Update Player ====================
function updatePlayer() {
    const prevX = player.x;
    const prevY = player.y;
    player.isMoving = false;

    const up = keys['ArrowUp'] || keys['w'] || keys['W'];
    const down = keys['ArrowDown'] || keys['s'] || keys['S'];
    const left = keys['ArrowLeft'] || keys['a'] || keys['A'];
    const right = keys['ArrowRight'] || keys['d'] || keys['D'];

    // Diagonal movement (wa, wd, sa, sd)
    if (up && left) {
        player.y -= player.speed * 0.707; // Diagonal speed
        player.x -= player.speed * 0.707;
        player.direction = 'up-left';
        player.isMoving = true;
    } else if (up && right) {
        player.y -= player.speed * 0.707;
        player.x += player.speed * 0.707;
        player.direction = 'up-right';
        player.isMoving = true;
    } else if (down && left) {
        player.y += player.speed * 0.707;
        player.x -= player.speed * 0.707;
        player.direction = 'down-left';
        player.isMoving = true;
    } else if (down && right) {
        player.y += player.speed * 0.707;
        player.x += player.speed * 0.707;
        player.direction = 'down-right';
        player.isMoving = true;
    }
    // Cardinal directions
    else if (up) {
        player.y -= player.speed;
        player.direction = 'up';
        player.isMoving = true;
    } else if (down) {
        player.y += player.speed;
        player.direction = 'down';
        player.isMoving = true;
    } else if (left) {
        player.x -= player.speed;
        player.direction = 'left';
        player.isMoving = true;
    } else if (right) {
        player.x += player.speed;
        player.direction = 'right';
        player.isMoving = true;
    }

    // Animation
    if (player.isMoving) {
        player.animTimer++;
        if (player.animTimer > 8) {
            player.animFrame = (player.animFrame + 1) % 2;
            player.animTimer = 0;
        }
    } else {
        player.animFrame = 0;
    }

    // Boundaries
    player.x = Math.max(player.width/2, Math.min(canvas.width - player.width/2, player.x));
    player.y = Math.max(player.height/2, Math.min(canvas.height - player.height/2, player.y));

    // Collision detection
    if (currentState === GameState.GRANDPA_CABIN) {
        const playerRect = {
            x: player.x - player.width/2,
            y: player.y - player.height/2,
            width: player.width,
            height: player.height
        };

        for (const obstacle of collisionRects) {
            if (checkCollision(playerRect, obstacle)) {
                player.x = prevX;
                player.y = prevY;
                break;
            }
        }
    }
}

function checkCollision(rect1, rect2) {
    return rect1.x < rect2.x + rect2.width &&
           rect1.x + rect1.width > rect2.x &&
           rect1.y < rect2.y + rect2.height &&
           rect1.y + rect1.height > rect2.y;
}

function isNearComputer() {
    const dx = player.x - (computer.x + computer.width/2);
    const dy = player.y - (computer.y + computer.height * 0.75);
    const distance = Math.sqrt(dx * dx + dy * dy);

    // Must be in front of computer (below it) and facing up
    const isFacingComputer = player.direction === 'up' && player.y > computer.y + computer.height * 0.5;

    return distance < computer.interactRadius && isFacingComputer;
}

function isNearDrawer() {
    const dx = player.x - (drawer.x + drawer.width/2);
    const dy = player.y - (drawer.y + drawer.height/2);
    const distance = Math.sqrt(dx * dx + dy * dy);

    return distance < drawer.interactRadius;
}

function checkInteraction() {
    if (currentState === GameState.GRANDPA_CABIN) {
        if (isNearDrawer() && !hasReadLetter) {
            // Open letter modal
            openLetter();
        } else if (isNearComputer() && hasReadLetter) {
            // Open BIP39 modal
            openBIP39Modal();
        }
    }
}

function openLetter() {
    document.getElementById('letter-modal').classList.add('active');
}

window.closeLetter = function() {
    document.getElementById('letter-modal').classList.remove('active');
    hasReadLetter = true;
};

// ==================== BIP39 Modal Functions ====================
let currentPhrase = '';
let currentPrivateKey = '';
let currentChallenge = '';
let currentSignature = '';

function openBIP39Modal() {
    document.getElementById('bip39-modal').classList.add('active');
}

window.closeBIP39Modal = function() {
    document.getElementById('bip39-modal').classList.remove('active');
};

window.selectPhrase = function(phrase) {
    currentPhrase = phrase;

    // Fill in the word inputs
    const words = phrase.split(' ');
    for (let i = 0; i < 6; i++) {
        const input = document.getElementById(`word${i+1}`);
        if (input && words[i]) {
            input.value = words[i];
            input.classList.add('valid');
        }
    }

    // Auto-scroll to derive button
    setTimeout(() => {
        const deriveBtn = document.querySelector('button[onclick="deriveKey()"]');
        if (deriveBtn) {
            deriveBtn.scrollIntoView({ behavior: 'smooth', block: 'center' });
        }
    }, 300);
};

window.deriveKey = async function() {
    // Wait for BIP39 module to load
    if (!window.bip39 || !window.systemReady) {
        // Retry after a short delay
        setTimeout(() => {
            if (window.bip39 && window.systemReady) {
                window.deriveKey();
            } else {
                alert('System not ready. Please wait a moment and try again...');
            }
        }, 500);
        return;
    }

    // Get words from inputs
    const words = [];
    for (let i = 1; i <= 6; i++) {
        const input = document.getElementById(`word${i}`);
        if (input && input.value.trim()) {
            words.push(input.value.trim().toLowerCase());
        }
    }

    if (words.length !== 6) {
        alert('Please enter all 6 words.');
        return;
    }

    const mnemonic = words.join(' ');

    // Validate each word against BIP39 wordlist (6-word game mode)
    try {
        const wordlist = window.bip39.wordlists.EN;
        for (let word of words) {
            if (!wordlist.includes(word)) {
                alert(`Invalid word: "${word}". Please check your words against the BIP39 wordlist.`);
                return;
            }
        }
    } catch (e) {
        alert('Error validating words: ' + e.message);
        return;
    }

    // Show key section
    document.getElementById('keySection').classList.add('show');

    // Auto-scroll to key section
    setTimeout(() => {
        document.getElementById('keySection').scrollIntoView({ behavior: 'smooth', block: 'start' });
    }, 100);

    // Animate derivation process
    await animateDerivation();

    // Derive the key
    try {
        const seed = await window.bip39.mnemonicToSeed(mnemonic);
        const seedHex = Array.from(new Uint8Array(seed.slice(0, 32)))
            .map(b => b.toString(16).padStart(2, '0'))
            .join('');

        currentPrivateKey = seedHex;
        currentPhrase = mnemonic;

        document.getElementById('privateKeyDisplay').textContent = currentPrivateKey;
    } catch (e) {
        alert('Error deriving key: ' + e.message);
    }
};

async function animateDerivation() {
    const steps = ['step1', 'step2', 'step3', 'step4'];
    for (let step of steps) {
        await new Promise(resolve => setTimeout(resolve, 400));
        const el = document.getElementById(step);
        if (el) {
            el.style.color = '#7a9a7a';
        }
    }
}

window.requestAccess = async function() {
    if (!currentPrivateKey) {
        alert('Please derive your key first.');
        return;
    }

    // Generate a random challenge
    const randomBytes = new Uint8Array(16);
    crypto.getRandomValues(randomBytes);
    currentChallenge = Array.from(randomBytes)
        .map(b => b.toString(16).padStart(2, '0'))
        .join('');

    document.getElementById('challengeDisplay').textContent = currentChallenge;
    document.getElementById('authSection').classList.add('show');

    // Auto-scroll to auth section
    setTimeout(() => {
        document.getElementById('authSection').scrollIntoView({ behavior: 'smooth', block: 'start' });
    }, 100);

    document.getElementById('authStatus').innerHTML =
        '<div class="status-message status-info">Challenge issued. Please sign to authenticate.</div>';
};

window.signChallenge = async function() {
    if (!currentPrivateKey || !currentChallenge) {
        alert('Missing authentication data.');
        return;
    }

    try {
        currentSignature = await window.crypto_utils.simpleSign(currentPrivateKey, currentChallenge);
        document.getElementById('signatureDisplay').textContent = currentSignature;
        document.getElementById('signatureSection').style.display = 'block';

        // Auto-scroll to signature
        setTimeout(() => {
            document.getElementById('signatureSection').scrollIntoView({ behavior: 'smooth', block: 'center' });
        }, 100);
    } catch (e) {
        alert('Error signing challenge: ' + e.message);
    }
};

window.verifyAndEnter = async function() {
    if (!currentPrivateKey || !currentChallenge || !currentSignature) {
        alert('Missing authentication data.');
        return;
    }

    try {
        const isValid = await window.crypto_utils.simpleVerify(
            currentPrivateKey,
            currentChallenge,
            currentSignature
        );

        if (isValid) {
            document.getElementById('authStatus').innerHTML =
                '<div class="status-message status-success">Signature verified. Access granted.</div>';

            // Show access granted screen
            setTimeout(() => {
                document.getElementById('accessSection').classList.add('show');
                document.getElementById('authSection').style.display = 'none';
                document.getElementById('keySection').style.display = 'none';

                // Auto-scroll to access granted
                setTimeout(() => {
                    document.getElementById('accessSection').scrollIntoView({ behavior: 'smooth', block: 'center' });
                }, 100);
            }, 1000);

            // Transition to Cryptopolis after delay
            setTimeout(() => {
                closeBIP39Modal();
                transitionToCryptopolis();
            }, 3000);

            console.log('Authentication successful - Access granted to Cryptopolis');
        } else {
            alert('Signature verification failed. Access denied.');
        }
    } catch (e) {
        alert('Error verifying signature: ' + e.message);
    }
};

// Real-time word validation
for (let i = 1; i <= 6; i++) {
    const input = document.getElementById(`word${i}`);
    if (input) {
        input.addEventListener('input', async function() {
            const word = this.value.trim().toLowerCase();
            if (word.length > 0) {
                if (window.bip39 && window.bip39.wordlists && window.bip39.wordlists.EN) {
                    const isValid = window.bip39.wordlists.EN.includes(word);
                    this.classList.remove('valid', 'invalid');
                    this.classList.add(isValid ? 'valid' : 'invalid');
                }
            } else {
                this.classList.remove('valid', 'invalid');
            }
        });
    }
}

function updateInteractionPrompt() {
    const prompt = document.getElementById('interaction-prompt');
    if (currentState === GameState.GRANDPA_CABIN) {
        if (isNearDrawer() && !hasReadLetter) {
            prompt.textContent = 'Press E to open drawer';
            prompt.style.display = 'block';
        } else if (isNearComputer() && hasReadLetter) {
            prompt.textContent = 'Press E to access computer';
            prompt.style.display = 'block';
        } else {
            prompt.style.display = 'none';
        }
    } else {
        prompt.style.display = 'none';
    }
}

// ==================== Scene Transitions ====================
function transitionToCryptopolis() {
    const overlay = document.getElementById('transition-overlay');
    const sceneName = document.getElementById('scene-name');

    overlay.classList.add('active');

    setTimeout(() => {
        currentState = GameState.CRYPTOPOLIS_MENU;
        sceneName.textContent = 'Cryptopolis';
        gameStarted = true;

        setTimeout(() => {
            overlay.classList.remove('active');
        }, 500);
    }, 1500);
}

function transitionToGameWorld() {
    const overlay = document.getElementById('transition-overlay');
    const sceneName = document.getElementById('scene-name');

    overlay.classList.add('active');

    setTimeout(() => {
        currentState = GameState.GAME_WORLD;
        sceneName.textContent = 'Game World';
        player.x = canvas.width / 2;
        player.y = canvas.height / 2;

        setTimeout(() => {
            overlay.classList.remove('active');
        }, 500);
    }, 1500);
}

// Allow clicking Continue button
canvas.addEventListener('click', (e) => {
    if (currentState === GameState.CRYPTOPOLIS_MENU && gameStarted) {
        const rect = canvas.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;

        const w = canvas.width;
        const h = canvas.height;
        const buttonX = w / 2 - 150;
        const buttonY = h / 2;
        const buttonW = 300;
        const buttonH = 60;

        if (x >= buttonX && x <= buttonX + buttonW &&
            y >= buttonY && y <= buttonY + buttonH) {
            // Continue button transitions to game world
            transitionToGameWorld();
        }
    }
});

// ==================== Game Loop ====================
function gameLoop() {
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    if (currentState === GameState.GRANDPA_CABIN) {
        updatePlayer();
        drawGrandpaCabin(); // Includes player in z-order
        updateInteractionPrompt();
    } else if (currentState === GameState.CRYPTOPOLIS_MENU) {
        drawCryptopolisMenu();
        updateInteractionPrompt();
    } else if (currentState === GameState.GAME_WORLD) {
        drawGameWorld();
        updatePlayer();
    }

    requestAnimationFrame(gameLoop);
}

// ==================== Initialize ====================
window.addEventListener('resize', () => {
    updateComputerPosition();
    updateDrawerPosition();
    updateCollisionRects();
});

// Easter egg: Pressing C goes to Cryptopolis menu (for testing)
window.addEventListener('keydown', (e) => {
    if (e.key === 'c' || e.key === 'C') {
        if (currentState === GameState.GRANDPA_CABIN) {
            transitionToCryptopolis();
        }
    }
});

// Wait for BIP39 module to load before starting game
function initGame() {
    console.log('Checking BIP39 module...', { bip39: !!window.bip39, systemReady: !!window.systemReady });

    if (window.bip39 && window.systemReady) {
        console.log('🎮 Cypherpunk Legacy - Game Started');
        console.log('📝 Use WASD or arrow keys to move');
        console.log('📨 Press E at drawer to read letter');
        console.log('💻 Press E at computer to access BIP39 authentication');
        console.log('🏙️ In Cryptopolis menu: Press E or click Continue to enter Encryption Demo');
        console.log('🎯 Press C to skip to Cryptopolis menu (testing)');

        gameLoop();
    } else {
        // Wait for modules to load
        console.log('Waiting for BIP39 module to load...');
        setTimeout(initGame, 100);
    }
}

// Start initialization when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initGame);
} else {
    initGame();
}
