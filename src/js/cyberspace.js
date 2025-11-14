// ==================== Cyberspace - First Person Lagrange Interpolation ====================

const canvas = document.getElementById('gameCanvas');
const ctx = canvas.getContext('2d');

// Set canvas size
function resizeCanvas() {
    const container = canvas.parentElement;
    canvas.width = container.clientWidth - 6;
    canvas.height = container.clientHeight - 6;
}
resizeCanvas();
window.addEventListener('resize', resizeCanvas);

// Game constants
const GRID_SIZE = 1; // Size of each grid cell in world units
const MOVE_SPEED = 0.05; // Player movement speed
const TURN_SPEED = 0.03; // Player turning speed
const VIEW_DISTANCE = 15; // How far player can see

// Game state
let gameState = 'start'; // 'start', 'playing', 'paused', 'gameover'
let score = 0;
let lives = 3;
let level = 1;
let animFrame = 0;

// Player (third person 2D top-down view)
const player = {
    x: 0, // Position in world X
    y: 0, // Position in world Y
    angle: 0 // Facing direction (for sprite orientation)
};

// Challenge data
let challengePoints = []; // Points for current polynomial (secret shares)
let queryPoint = null; // The point to find
let correctAnswer = null;
let isNearQuery = false;
let secretValue = null; // The secret (f(0)) in Shamir Secret Sharing

// Input handling
const keys = {};
window.addEventListener('keydown', (e) => {
    keys[e.key] = true;

    if (e.key === ' ' && gameState === 'playing' && isNearQuery) {
        e.preventDefault();
        openSolveModal();
    }

    if (e.key === 'p' || e.key === 'P') {
        if (gameState === 'playing') {
            pauseGame();
        } else if (gameState === 'paused') {
            resumeGame();
        }
    }
});

window.addEventListener('keyup', (e) => {
    keys[e.key] = false;
});

// UI Elements
const startScreen = document.getElementById('startScreen');
const pauseScreen = document.getElementById('pauseScreen');
const gameOverScreen = document.getElementById('gameOverScreen');
const startBtn = document.getElementById('startBtn');
const restartBtn = document.getElementById('restartBtn');
const resumeBtn = document.getElementById('resumeBtn');
const exitBtn = document.getElementById('exitBtn');

startBtn.addEventListener('click', startGame);
restartBtn.addEventListener('click', restartGame);
resumeBtn.addEventListener('click', resumeGame);
exitBtn.addEventListener('click', () => {
    window.location.href = '../../index.html';
});

// ==================== Game Functions ====================

function startGame() {
    startScreen.classList.remove('active');
    gameState = 'playing';
    initGame();
}

function initGame() {
    score = 0;
    lives = 3;
    level = 1;

    // Start player at origin
    player.x = 0;
    player.y = 0;
    player.angle = 0;

    generateChallenge();
    updateUI();
}

function pauseGame() {
    gameState = 'paused';
    pauseScreen.classList.add('active');
}

function resumeGame() {
    gameState = 'playing';
    pauseScreen.classList.remove('active');
}

function restartGame() {
    gameOverScreen.classList.remove('active');
    gameState = 'playing';
    initGame();
}

function gameOver() {
    gameState = 'gameover';
    document.getElementById('finalScore').textContent = score;
    document.getElementById('finalWave').textContent = level;
    gameOverScreen.classList.add('active');
}

function updateUI() {
    document.getElementById('score').textContent = score;
    document.getElementById('lives').textContent = lives;
    document.getElementById('wave').textContent = level;
}

// ==================== Challenge Generation ====================

function generateChallenge() {
    // Shamir Secret Sharing: Generate a quadratic polynomial y = ax² + bx + c
    // The secret is c (the value at x=0, i.e., f(0))
    // We distribute 3 shares (points on the polynomial)

    challengePoints = [];

    // Generate random coefficients for the secret polynomial
    const a = (Math.random() * 0.4 + 0.1) * (Math.random() > 0.5 ? 1 : -1); // 0.1 to 0.5
    const b = (Math.random() * 2 - 1) * 0.5; // -0.5 to 0.5
    const c = Math.floor(Math.random() * 20) - 10; // SECRET: -10 to 10

    // Store the secret value (f(0) = c)
    secretValue = c;

    // Generate 3 shares (points) on this polynomial
    // In Shamir's scheme, shares are at x = 1, 2, 3, etc. (NEVER x = 0)
    // x = 0 is the secret f(0), which we must recover!
    const xValues = [1, 2, 3]; // Three shares at x=1, x=2, x=3
    for (const x of xValues) {
        const y = Math.round((a * x * x + b * x + c) * 10) / 10;
        challengePoints.push({ x, y });
    }

    // Choose query point (not at the given x values)
    // Player must calculate this share to verify they understand the polynomial
    const possibleX = [-4, -2, -1, 1, 2, 4];
    const queryX = possibleX[Math.floor(Math.random() * possibleX.length)];
    const queryY = lagrangeInterpolate(challengePoints, queryX);
    queryPoint = { x: queryX, y: queryY };
    correctAnswer = queryY;
}

// Lagrange interpolation
function lagrangeInterpolate(points, x) {
    let result = 0;
    const n = points.length;

    for (let i = 0; i < n; i++) {
        let term = points[i].y;
        for (let j = 0; j < n; j++) {
            if (j !== i) {
                term *= (x - points[j].x) / (points[i].x - points[j].x);
            }
        }
        result += term;
    }

    return Math.round(result * 100) / 100;
}

// ==================== Update Functions ====================

function update() {
    if (gameState !== 'playing') return;

    animFrame++;

    // Player movement in 2D space (top-down)
    const moveSpeed = 0.15;

    if (keys['w'] || keys['W'] || keys['ArrowUp']) {
        // Move up
        player.y -= moveSpeed;
        player.angle = -Math.PI / 2;
    }
    if (keys['s'] || keys['S'] || keys['ArrowDown']) {
        // Move down
        player.y += moveSpeed;
        player.angle = Math.PI / 2;
    }
    if (keys['a'] || keys['A'] || keys['ArrowLeft']) {
        // Move left
        player.x -= moveSpeed;
        player.angle = Math.PI;
    }
    if (keys['d'] || keys['D'] || keys['ArrowRight']) {
        // Move right
        player.x += moveSpeed;
        player.angle = 0;
    }

    // Keep player in reasonable bounds
    player.x = Math.max(-20, Math.min(20, player.x));
    player.y = Math.max(-20, Math.min(20, player.y));

    // Check if near query point (in 2D space)
    if (queryPoint) {
        const dist = Math.sqrt(
            Math.pow(player.x - queryPoint.x, 2) +
            Math.pow(player.y - queryPoint.y, 2)
        );
        isNearQuery = dist < 1.5;
    }
}

// ==================== Rendering Functions ====================

// Camera settings for 2D top-down view
const camera = {
    scale: 30, // Pixels per world unit
    offsetX: 0, // Camera offset X
    offsetY: 0  // Camera offset Y
};

function draw() {
    // Clear screen
    ctx.fillStyle = '#000011';
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    // Update camera to follow player
    camera.offsetX = player.x;
    camera.offsetY = player.y;

    // Draw 2D top-down view
    drawTopDownGrid();
    drawPolynomialCurve2D();
    drawChallengePoints2D();
    drawQueryPoint2D();
    drawPlayer2D();

    // Draw HUD overlay
    drawHUD();
}

// Convert world coordinates to screen coordinates (2D top-down)
function worldToScreen2D(worldX, worldY) {
    const screenX = canvas.width / 2 + (worldX - camera.offsetX) * camera.scale;
    const screenY = canvas.height / 2 + (worldY - camera.offsetY) * camera.scale;
    return { x: screenX, y: screenY };
}

function drawTopDownGrid() {
    // Draw 2D grid in top-down view
    const gridRange = 25; // Extended range to cover polynomial curve

    // Draw grid lines
    ctx.strokeStyle = 'rgba(0, 255, 255, 0.15)';
    ctx.lineWidth = 1;

    // Vertical grid lines
    for (let x = -gridRange; x <= gridRange; x++) {
        const top = worldToScreen2D(x, -gridRange);
        const bottom = worldToScreen2D(x, gridRange);
        ctx.beginPath();
        ctx.moveTo(top.x, top.y);
        ctx.lineTo(bottom.x, bottom.y);
        ctx.stroke();
    }

    // Horizontal grid lines
    for (let y = -gridRange; y <= gridRange; y++) {
        const left = worldToScreen2D(-gridRange, y);
        const right = worldToScreen2D(gridRange, y);
        ctx.beginPath();
        ctx.moveTo(left.x, left.y);
        ctx.lineTo(right.x, right.y);
        ctx.stroke();
    }

    // Draw integer lattice points
    ctx.fillStyle = 'rgba(0, 255, 255, 0.3)';
    for (let x = -gridRange; x <= gridRange; x++) {
        for (let y = -gridRange; y <= gridRange; y++) {
            const screen = worldToScreen2D(x, y);
            ctx.beginPath();
            ctx.arc(screen.x, screen.y, 2, 0, 2 * Math.PI);
            ctx.fill();
        }
    }

    // Highlight axes
    ctx.strokeStyle = 'rgba(0, 255, 255, 0.6)';
    ctx.lineWidth = 2;

    // X-axis
    const xLeft = worldToScreen2D(-gridRange, 0);
    const xRight = worldToScreen2D(gridRange, 0);
    ctx.beginPath();
    ctx.moveTo(xLeft.x, xLeft.y);
    ctx.lineTo(xRight.x, xRight.y);
    ctx.stroke();

    // Y-axis
    const yTop = worldToScreen2D(0, -gridRange);
    const yBottom = worldToScreen2D(0, gridRange);
    ctx.beginPath();
    ctx.moveTo(yTop.x, yTop.y);
    ctx.lineTo(yBottom.x, yBottom.y);
    ctx.stroke();

    // Draw axis labels
    ctx.fillStyle = 'rgba(0, 255, 255, 0.8)';
    ctx.font = '12px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'top';

    // X-axis labels
    for (let x = -gridRange; x <= gridRange; x += 5) {
        const screen = worldToScreen2D(x, 0);
        if (screen.y > 0 && screen.y < canvas.height - 20) {
            ctx.fillText(x.toString(), screen.x, screen.y + 5);
        }
    }

    // Y-axis labels
    ctx.textAlign = 'right';
    ctx.textBaseline = 'middle';
    for (let y = -gridRange; y <= gridRange; y += 5) {
        if (y !== 0) {
            const screen = worldToScreen2D(0, y);
            if (screen.x > 40 && screen.x < canvas.width) {
                ctx.fillText(y.toString(), screen.x - 8, screen.y);
            }
        }
    }
}

function drawPolynomialCurve2D() {
    if (challengePoints.length < 2) return;

    ctx.strokeStyle = '#ff00ff';
    ctx.lineWidth = 3;
    ctx.shadowBlur = 15;
    ctx.shadowColor = '#ff00ff';

    ctx.beginPath();
    let started = false;

    // Draw curve across extended range
    for (let wx = -15; wx <= 15; wx += 0.1) {
        const wy = lagrangeInterpolate(challengePoints, wx);
        const screen = worldToScreen2D(wx, wy);

        if (!started) {
            ctx.moveTo(screen.x, screen.y);
            started = true;
        } else {
            ctx.lineTo(screen.x, screen.y);
        }
    }

    ctx.stroke();
    ctx.shadowBlur = 0;
}

function drawChallengePoints2D() {
    challengePoints.forEach((point, index) => {
        const screen = worldToScreen2D(point.x, point.y);

        // Draw point (secret share)
        ctx.fillStyle = '#00ffff';
        ctx.shadowBlur = 15;
        ctx.shadowColor = '#00ffff';
        ctx.beginPath();
        ctx.arc(screen.x, screen.y, 12, 0, 2 * Math.PI);
        ctx.fill();
        ctx.shadowBlur = 0;

        // Draw border
        ctx.strokeStyle = '#ffffff';
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.arc(screen.x, screen.y, 12, 0, 2 * Math.PI);
        ctx.stroke();

        // Draw label
        ctx.fillStyle = '#ffffff';
        ctx.font = 'bold 12px monospace';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'top';
        ctx.fillText(`Share ${index + 1}`, screen.x, screen.y + 18);
        ctx.font = '11px monospace';
        ctx.fillStyle = '#00ffff';
        ctx.fillText(`(${point.x}, ${point.y})`, screen.x, screen.y + 32);
    });
}

function drawQueryPoint2D() {
    if (!queryPoint) return;

    const screen = worldToScreen2D(queryPoint.x, queryPoint.y);
    const pulse = isNearQuery ? (Math.sin(animFrame * 0.1) * 0.5 + 0.5) : 0.5;

    // Draw pulsing circle
    ctx.fillStyle = `rgba(255, 255, 0, ${0.3 + pulse * 0.4})`;
    ctx.shadowBlur = isNearQuery ? 30 : 15;
    ctx.shadowColor = '#ffff00';
    ctx.beginPath();
    ctx.arc(screen.x, screen.y, isNearQuery ? 20 : 16, 0, 2 * Math.PI);
    ctx.fill();
    ctx.shadowBlur = 0;

    // Draw border
    ctx.strokeStyle = '#ffff00';
    ctx.lineWidth = 3;
    ctx.beginPath();
    ctx.arc(screen.x, screen.y, isNearQuery ? 20 : 16, 0, 2 * Math.PI);
    ctx.stroke();

    // Draw question mark
    ctx.fillStyle = '#000000';
    ctx.font = 'bold 24px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText('?', screen.x, screen.y);

    // Draw label
    ctx.fillStyle = '#ffff00';
    ctx.font = 'bold 14px monospace';
    ctx.textBaseline = 'top';
    ctx.fillText(`f(${queryPoint.x}) = ?`, screen.x, screen.y + 28);

    if (isNearQuery) {
        ctx.fillStyle = '#00ffff';
        ctx.font = 'bold 12px monospace';
        ctx.fillText('Press SPACE', screen.x, screen.y + 46);
    }
}

function drawPlayer2D() {
    const screen = worldToScreen2D(player.x, player.y);

    // Draw player circle (character)
    ctx.fillStyle = '#00ff00';
    ctx.shadowBlur = 20;
    ctx.shadowColor = '#00ff00';
    ctx.beginPath();
    ctx.arc(screen.x, screen.y, 10, 0, 2 * Math.PI);
    ctx.fill();
    ctx.shadowBlur = 0;

    // Draw border
    ctx.strokeStyle = '#ffffff';
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.arc(screen.x, screen.y, 10, 0, 2 * Math.PI);
    ctx.stroke();

    // Draw direction indicator
    const dirX = Math.cos(player.angle) * 15;
    const dirY = Math.sin(player.angle) * 15;
    ctx.strokeStyle = '#00ff00';
    ctx.lineWidth = 3;
    ctx.beginPath();
    ctx.moveTo(screen.x, screen.y);
    ctx.lineTo(screen.x + dirX, screen.y + dirY);
    ctx.stroke();
}

// ==================== OLD 3D RENDERING (KEEP FOR REFERENCE) ====================

function drawPerspectiveGrid() {
    // Draw integer lattice grid on the ground
    const gridRange = 15;

    // Draw grid lines
    ctx.strokeStyle = 'rgba(0, 255, 255, 0.2)';
    ctx.lineWidth = 1;

    // Vertical lines (parallel to Y-axis in world space)
    for (let x = -gridRange; x <= gridRange; x++) {
        let lastScreen = null;
        for (let y = -gridRange; y <= gridRange; y++) {
            const screen = worldToScreen(x, y);
            if (screen && lastScreen) {
                ctx.beginPath();
                ctx.moveTo(lastScreen.x, lastScreen.y);
                ctx.lineTo(screen.x, screen.y);
                ctx.stroke();
            }
            lastScreen = screen;
        }
    }

    // Horizontal lines (parallel to X-axis in world space)
    for (let y = -gridRange; y <= gridRange; y++) {
        let lastScreen = null;
        for (let x = -gridRange; x <= gridRange; x++) {
            const screen = worldToScreen(x, y);
            if (screen && lastScreen) {
                ctx.beginPath();
                ctx.moveTo(lastScreen.x, lastScreen.y);
                ctx.lineTo(screen.x, screen.y);
                ctx.stroke();
            }
            lastScreen = screen;
        }
    }

    // Draw integer lattice points
    ctx.fillStyle = 'rgba(0, 255, 255, 0.4)';
    for (let x = -gridRange; x <= gridRange; x++) {
        for (let y = -gridRange; y <= gridRange; y++) {
            const screen = worldToScreen(x, y);
            if (screen) {
                ctx.beginPath();
                ctx.arc(screen.x, screen.y, 2, 0, 2 * Math.PI);
                ctx.fill();
            }
        }
    }

    // Highlight axes (X=0 and Y=0)
    ctx.strokeStyle = 'rgba(0, 255, 255, 0.6)';
    ctx.lineWidth = 2;

    // X-axis (Y=0)
    let lastScreen = null;
    for (let x = -gridRange; x <= gridRange; x++) {
        const screen = worldToScreen(x, 0);
        if (screen && lastScreen) {
            ctx.beginPath();
            ctx.moveTo(lastScreen.x, lastScreen.y);
            ctx.lineTo(screen.x, screen.y);
            ctx.stroke();
        }
        lastScreen = screen;
    }

    // Y-axis (X=0)
    lastScreen = null;
    for (let y = -gridRange; y <= gridRange; y++) {
        const screen = worldToScreen(0, y);
        if (screen && lastScreen) {
            ctx.beginPath();
            ctx.moveTo(lastScreen.x, lastScreen.y);
            ctx.lineTo(screen.x, screen.y);
            ctx.stroke();
        }
        lastScreen = screen;
    }

    // Draw axis labels
    ctx.fillStyle = 'rgba(0, 255, 255, 0.8)';
    ctx.font = '12px monospace';
    ctx.textAlign = 'center';

    // Label some X coordinates on the X-axis
    for (let x = -8; x <= 8; x += 2) {
        const screen = worldToScreen(x, 0);
        if (screen && screen.y < canvas.height - 30) {
            ctx.fillText(x.toString(), screen.x, screen.y + 15);
        }
    }

    // Label some Y coordinates on the Y-axis
    ctx.textAlign = 'right';
    for (let y = -8; y <= 8; y += 2) {
        if (y !== 0) {
            const screen = worldToScreen(0, y);
            if (screen && screen.x > 30) {
                ctx.fillText(y.toString(), screen.x - 10, screen.y + 5);
            }
        }
    }
}

function drawPolynomialCurve3D() {
    if (challengePoints.length < 2) return;

    ctx.strokeStyle = '#ff00ff';
    ctx.lineWidth = 4;
    ctx.shadowBlur = 20;
    ctx.shadowColor = '#ff00ff';

    ctx.beginPath();
    let started = false;

    // Draw curve on the ground grid (y-coordinate represents the second dimension)
    for (let wx = -8; wx <= 8; wx += 0.2) {
        const wy = lagrangeInterpolate(challengePoints, wx);

        // Curve is on the ground: X axis is worldX, Y axis is the polynomial value
        const screen = worldToScreen(wx, wy);

        if (screen) {
            if (!started) {
                ctx.moveTo(screen.x, screen.y);
                started = true;
            } else {
                ctx.lineTo(screen.x, screen.y);
            }
        }
    }

    ctx.stroke();
    ctx.shadowBlur = 0;
}

function drawChallengePoints3D() {
    challengePoints.forEach((point, index) => {
        // Points on the ground grid at (x, y) coordinates
        const screen = worldToScreen(point.x, point.y);

        if (screen) {
            // Draw vertical line from grid to point (to show height on grid)
            const baseScreen = worldToScreen(point.x, 0);
            if (baseScreen) {
                ctx.strokeStyle = 'rgba(0, 255, 255, 0.3)';
                ctx.lineWidth = 2;
                ctx.setLineDash([5, 5]);
                ctx.beginPath();
                ctx.moveTo(baseScreen.x, baseScreen.y);
                ctx.lineTo(screen.x, screen.y);
                ctx.stroke();
                ctx.setLineDash([]);
            }

            // Draw point (secret share)
            ctx.fillStyle = '#00ffff';
            ctx.shadowBlur = 20;
            ctx.shadowColor = '#00ffff';
            ctx.beginPath();
            ctx.arc(screen.x, screen.y, 10, 0, 2 * Math.PI);
            ctx.fill();
            ctx.shadowBlur = 0;

            // Draw label with share number
            ctx.fillStyle = '#ffffff';
            ctx.font = 'bold 14px monospace';
            ctx.textAlign = 'center';
            ctx.fillText(`Share ${index + 1}`, screen.x, screen.y - 28);
            ctx.font = 'bold 12px monospace';
            ctx.fillStyle = '#00ffff';
            ctx.fillText(`(${point.x}, ${point.y})`, screen.x, screen.y - 14);
        }
    });
}

function drawQueryPoint3D() {
    if (!queryPoint) return;

    // Query point on the ground grid
    const screen = worldToScreen(queryPoint.x, queryPoint.y);

    if (screen) {
        const pulse = isNearQuery ? (Math.sin(animFrame * 0.1) * 0.5 + 0.5) : 0.5;

        // Draw vertical line from grid to point
        const baseScreen = worldToScreen(queryPoint.x, 0);
        if (baseScreen) {
            ctx.strokeStyle = `rgba(255, 255, 0, ${pulse * 0.6})`;
            ctx.lineWidth = 3;
            ctx.setLineDash([10, 10]);
            ctx.beginPath();
            ctx.moveTo(baseScreen.x, baseScreen.y);
            ctx.lineTo(screen.x, screen.y);
            ctx.stroke();
            ctx.setLineDash([]);
        }

        // Draw pulsing circle
        ctx.fillStyle = `rgba(255, 255, 0, ${0.2 + pulse * 0.6})`;
        ctx.shadowBlur = isNearQuery ? 40 : 20;
        ctx.shadowColor = '#ffff00';
        ctx.beginPath();
        ctx.arc(screen.x, screen.y, isNearQuery ? 18 : 14, 0, 2 * Math.PI);
        ctx.fill();
        ctx.shadowBlur = 0;

        // Draw question mark
        ctx.fillStyle = '#000000';
        ctx.font = 'bold 28px monospace';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText('?', screen.x, screen.y);

        // Draw label
        ctx.fillStyle = '#ffff00';
        ctx.font = 'bold 18px monospace';
        ctx.textBaseline = 'top';
        ctx.fillText(`f(${queryPoint.x}) = ?`, screen.x, screen.y + 28);

        if (isNearQuery) {
            ctx.fillStyle = '#00ffff';
            ctx.font = 'bold 16px monospace';
            ctx.fillText('Press SPACE', screen.x, screen.y + 52);
        }
    }
}

function drawHUD() {
    // Draw Shamir Secret Sharing info panel (top left)
    const panelX = 20;
    const panelY = 20;
    const panelWidth = 320;
    const panelHeight = 140;

    // Panel background
    ctx.fillStyle = 'rgba(0, 0, 0, 0.8)';
    ctx.strokeStyle = '#00ffff';
    ctx.lineWidth = 2;
    ctx.fillRect(panelX, panelY, panelWidth, panelHeight);
    ctx.strokeRect(panelX, panelY, panelWidth, panelHeight);

    // Title
    ctx.fillStyle = '#00ffff';
    ctx.font = 'bold 16px monospace';
    ctx.textAlign = 'left';
    ctx.fillText('🔐 SHAMIR SECRET SHARING', panelX + 10, panelY + 25);

    // Secret info
    ctx.fillStyle = '#00ff00';
    ctx.font = 'bold 14px monospace';
    ctx.fillText(`Secret Value: f(0) = ${secretValue !== null ? secretValue : '???'}`, panelX + 10, panelY + 50);

    // Shares info
    ctx.fillStyle = '#00ffff';
    ctx.font = '12px monospace';
    ctx.fillText(`Known Shares: ${challengePoints.length}`, panelX + 10, panelY + 75);

    // Display share coordinates
    if (challengePoints.length > 0) {
        ctx.fillStyle = '#0088ff';
        ctx.font = '11px monospace';
        let shareText = 'Share Points: ';
        for (let i = 0; i < challengePoints.length; i++) {
            const pt = challengePoints[i];
            shareText += `(${pt.x}, ${pt.y})`;
            if (i < challengePoints.length - 1) shareText += ', ';
        }
        ctx.fillText(shareText, panelX + 10, panelY + 95);
    }

    // Quest objective
    ctx.fillStyle = '#ffff00';
    ctx.font = 'bold 11px monospace';
    ctx.fillText(`Quest: Calculate f(${queryPoint ? queryPoint.x : '?'})`, panelX + 10, panelY + 115);

    // Hint
    ctx.fillStyle = '#ff00ff';
    ctx.font = '10px monospace';
    ctx.fillText('💡 Use Lagrange interpolation to recover!', panelX + 10, panelY + 132);

    // Draw player position (bottom left)
    ctx.fillStyle = 'rgba(0, 255, 255, 0.7)';
    ctx.font = '14px monospace';
    ctx.textAlign = 'left';
    ctx.fillText(`Position: (${player.x.toFixed(1)}, ${player.y.toFixed(1)})`, 20, canvas.height - 30);
}

// ==================== Coordinate Transformation ====================

function worldToScreen(worldX, worldY) {
    // Points on the ground plane (z = 0)
    return worldToScreen3D(worldX, worldY, 0);
}

function worldToScreen3D(worldX, worldY, worldZ) {
    // Transform 3D world coordinates to 2D screen coordinates
    // World: X (horizontal), Y (depth), Z (height)
    // Camera at (player.x, player.y, player.z)

    // Translate to camera space
    let dx = worldX - player.x;
    let dy = worldY - player.y;
    let dz = worldZ - player.z;

    // Rotate around Z-axis (horizontal rotation - yaw)
    const cosH = Math.cos(-player.angleH);
    const sinH = Math.sin(-player.angleH);
    const rotX = dx * cosH - dy * sinH;
    const rotY = dx * sinH + dy * cosH;

    // Rotate around X-axis (vertical rotation - pitch)
    const cosV = Math.cos(-player.angleV);
    const sinV = Math.sin(-player.angleV);
    const rotZ = dz * cosV - rotY * sinV;
    const depth = dz * sinV + rotY * cosV;

    // Check if behind camera
    if (depth <= 0.1) return null;

    // Perspective projection
    const fov = 800; // Field of view factor
    const scale = fov / depth;

    const screenX = canvas.width / 2 + rotX * scale;
    const screenY = canvas.height / 2 - rotZ * scale;

    // Check if on screen
    if (screenX < -100 || screenX > canvas.width + 100) return null;
    if (screenY < -100 || screenY > canvas.height + 100) return null;

    return { x: screenX, y: screenY, depth: depth };
}

// ==================== Modal Functions ====================

function openSolveModal() {
    gameState = 'paused';

    const modal = document.createElement('div');
    modal.id = 'interpolationModal';
    modal.style.cssText = `
        position: fixed;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        background: rgba(0, 0, 0, 0.95);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 1000;
    `;

    const modalContent = document.createElement('div');
    modalContent.style.cssText = `
        background: linear-gradient(135deg, #001133 0%, #002244 100%);
        border: 3px solid #00ffff;
        border-radius: 15px;
        padding: 40px;
        max-width: 600px;
        box-shadow: 0 0 50px rgba(0, 255, 255, 0.8);
    `;

    modalContent.innerHTML = `
        <h2 style="color: #00ffff; text-align: center; margin-bottom: 20px; text-shadow: 0 0 10px #00ffff; font-size: 28px;">
            🔐 Shamir Secret Sharing Challenge
        </h2>
        <div style="color: #ffffff; margin-bottom: 20px; font-family: monospace; font-size: 14px;">
            <div style="padding: 15px; background: rgba(0, 255, 0, 0.1); border: 1px solid #00ff00; border-radius: 8px; margin-bottom: 20px;">
                <p style="color: #00ff00; margin-bottom: 8px;"><strong>📜 About Shamir's Scheme (1979):</strong></p>
                <p style="font-size: 12px; line-height: 1.5;">
                    Adi Shamir's secret sharing splits a secret into <em>n</em> shares. Any <em>k</em> shares can reconstruct the secret, but fewer shares reveal nothing.
                    The secret is hidden as <strong style="color: #00ffff;">f(0)</strong> in a polynomial of degree <em>k-1</em>.
                </p>
            </div>
            <p style="margin-bottom: 15px; color: #00ffff;"><strong>Known Secret Shares:</strong></p>
            <div style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; margin-bottom: 20px;">
                ${challengePoints.map((p, i) => `
                    <div style="padding: 10px; background: rgba(0, 255, 255, 0.1); border: 1px solid #00ffff; border-radius: 5px; text-align: center;">
                        <div style="font-size: 11px; color: #0088ff;">Share ${i + 1}</div>
                        <div style="font-size: 14px; font-weight: bold; color: #00ffff;">(${p.x}, ${p.y})</div>
                    </div>
                `).join('')}
            </div>
            <div style="padding: 12px; background: rgba(0, 255, 0, 0.1); border-left: 3px solid #00ff00; margin-bottom: 15px;">
                <p style="color: #00ff00; font-size: 13px;">
                    <strong>🔓 Secret Value:</strong> f(0) = <span style="font-size: 16px; color: #00ffff;">${secretValue}</span>
                </p>
            </div>
            <p style="margin-top: 20px; font-size: 20px; color: #ffff00; text-align: center; padding: 15px; background: rgba(255, 255, 0, 0.1); border-radius: 8px;">
                <strong>Verify by calculating: f(${queryPoint.x}) = ?</strong>
            </p>
        </div>
        <div style="margin-bottom: 20px;">
            <input type="number" id="answerInput" step="0.01"
                style="width: 100%; padding: 15px; font-size: 24px; background: #000011;
                       color: #00ffff; border: 2px solid #00ffff; border-radius: 8px;
                       font-family: monospace; text-align: center; font-weight: bold;"
                placeholder="Enter your answer"
                autofocus>
        </div>
        <div style="display: flex; gap: 15px;">
            <button id="submitAnswer" style="flex: 1; padding: 15px; font-size: 18px;
                    background: linear-gradient(135deg, #00ffff 0%, #0088ff 100%);
                    color: #000; border: none; border-radius: 8px; font-weight: bold;
                    cursor: pointer; box-shadow: 0 0 20px rgba(0, 255, 255, 0.5);">
                Submit
            </button>
            <button id="cancelAnswer" style="flex: 1; padding: 15px; font-size: 18px;
                    background: linear-gradient(135deg, #ff0088 0%, #ff00ff 100%);
                    color: #fff; border: none; border-radius: 8px; font-weight: bold;
                    cursor: pointer; box-shadow: 0 0 20px rgba(255, 0, 255, 0.5);">
                Cancel
            </button>
        </div>
        <div style="margin-top: 20px; padding: 12px; background: rgba(255, 0, 255, 0.1);
                    border: 1px solid #ff00ff; border-radius: 8px; color: #ff00ff;
                    font-size: 12px; font-family: monospace; line-height: 1.6;">
            <strong>💡 Lagrange Interpolation Formula:</strong><br>
            <div style="text-align: center; margin-top: 8px; font-size: 13px; color: #ffaaff;">
                L(x) = Σ y<sub>i</sub> · Π<sub>j≠i</sub> (x - x<sub>j</sub>)/(x<sub>i</sub> - x<sub>j</sub>)
            </div>
        </div>
    `;

    modal.appendChild(modalContent);
    document.body.appendChild(modal);

    document.getElementById('submitAnswer').addEventListener('click', () => {
        const answer = parseFloat(document.getElementById('answerInput').value);
        if (!isNaN(answer)) {
            checkAnswer(answer);
            document.body.removeChild(modal);
        }
    });

    document.getElementById('cancelAnswer').addEventListener('click', () => {
        gameState = 'playing';
        document.body.removeChild(modal);
    });

    document.getElementById('answerInput').addEventListener('keypress', (e) => {
        if (e.key === 'Enter') {
            const answer = parseFloat(document.getElementById('answerInput').value);
            if (!isNaN(answer)) {
                checkAnswer(answer);
                document.body.removeChild(modal);
            }
        }
    });
}

function checkAnswer(userAnswer) {
    const tolerance = 0.5;
    const isCorrect = Math.abs(userAnswer - correctAnswer) < tolerance;

    if (isCorrect) {
        // Show success message with secret recovery confirmation
        showFeedback(
            '✅ SECRET SHARE VERIFIED!',
            `<div style="text-align: center; color: #00ff00; font-family: monospace;">
                <p style="font-size: 18px; margin-bottom: 15px;">Correct! f(${queryPoint.x}) = ${correctAnswer.toFixed(2)}</p>
                <div style="padding: 15px; background: rgba(0, 255, 0, 0.2); border: 2px solid #00ff00; border-radius: 10px; margin: 15px 0;">
                    <p style="font-size: 16px; margin-bottom: 10px;"><strong>🔓 Secret Recovered:</strong></p>
                    <p style="font-size: 24px; font-weight: bold; color: #00ffff;">f(0) = ${secretValue}</p>
                </div>
                <p style="font-size: 14px; margin-top: 15px;">You successfully used Lagrange interpolation to verify the polynomial!</p>
                <p style="font-size: 14px; color: #ffff00; margin-top: 10px;">+${100 * level} points</p>
            </div>`,
            '#00ff00'
        );
        score += 100 * level;
        level++;
        generateChallenge();
    } else {
        // Show error message
        showFeedback(
            '❌ INCORRECT CALCULATION',
            `<div style="text-align: center; color: #ff0000; font-family: monospace;">
                <p style="font-size: 16px; margin-bottom: 10px;">Your answer: ${userAnswer.toFixed(2)}</p>
                <p style="font-size: 16px;">Correct answer: ${correctAnswer.toFixed(2)}</p>
                <p style="font-size: 14px; margin-top: 15px; color: #ffff00;">Review the Lagrange interpolation formula!</p>
                <p style="font-size: 14px; color: #ff0000; margin-top: 10px;">-1 shield</p>
            </div>`,
            '#ff0000'
        );
        lives--;
        if (lives <= 0) {
            setTimeout(gameOver, 2000);
            return;
        }
    }

    updateUI();
    setTimeout(() => {
        gameState = 'playing';
    }, 3000);
}

function showFeedback(title, message, color) {
    const feedback = document.createElement('div');
    feedback.style.cssText = `
        position: fixed;
        top: 50%;
        left: 50%;
        transform: translate(-50%, -50%);
        background: rgba(0, 0, 0, 0.95);
        border: 3px solid ${color};
        border-radius: 15px;
        padding: 30px 40px;
        z-index: 2000;
        box-shadow: 0 0 50px ${color};
        animation: fadeIn 0.3s;
    `;

    feedback.innerHTML = `
        <h3 style="color: ${color}; text-align: center; font-size: 24px; margin-bottom: 20px; text-shadow: 0 0 10px ${color};">
            ${title}
        </h3>
        ${message}
    `;

    document.body.appendChild(feedback);

    setTimeout(() => {
        feedback.style.animation = 'fadeOut 0.3s';
        setTimeout(() => {
            document.body.removeChild(feedback);
        }, 300);
    }, 2500);
}

// ==================== Game Loop ====================

function gameLoop() {
    update();
    draw();
    requestAnimationFrame(gameLoop);
}

// Start game loop
gameLoop();
