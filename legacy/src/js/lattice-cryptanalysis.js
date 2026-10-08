// ==================== Lattice Cryptanalysis - LLL Algorithm Lab ====================

// Game State
let gameState = {
    currentSection: 'intro',
    unlockedSections: ['intro'],
    puzzlesSolved: 0,
    playerLevel: 1
};

// Lattice visualization state
let lattice = {
    basis: [[4, 1], [1, 3]], // Default basis vectors
    scale: 40,
    origin: [250, 250],
    dragging: null,
    hoveredVector: null,
    showGrid: true,
    animating: false
};

// Initialize
window.addEventListener('DOMContentLoaded', init);

function init() {
    setupNav();
    setupIntroCanvas();
    setupBasicsCanvas();
    setupSVPCanvas();
    setupLLLDemo();
    setupAttackDemo();
    loadGameState();
}

// ==================== Navigation ====================

function setupNav() {
    document.querySelectorAll('.nav-item').forEach(item => {
        item.addEventListener('click', () => {
            const section = item.dataset.section;
            if (gameState.unlockedSections.includes(section)) {
                switchSection(section);
            }
        });
    });
}

function switchSection(sectionName) {
    // Update nav
    document.querySelectorAll('.nav-item').forEach(item => item.classList.remove('active'));
    document.querySelector(`[data-section="${sectionName}"]`).classList.add('active');

    // Update content
    document.querySelectorAll('.section').forEach(section => section.classList.remove('active'));
    document.getElementById(sectionName).classList.add('active');

    gameState.currentSection = sectionName;
    saveGameState();
}

function unlockSection(sectionName) {
    if (!gameState.unlockedSections.includes(sectionName)) {
        gameState.unlockedSections.push(sectionName);
        document.querySelector(`[data-section="${sectionName}"]`).classList.remove('locked');
        gameState.puzzlesSolved++;
        updateStats();
        saveGameState();
    }
    switchSection(sectionName);
}

function updateStats() {
    document.getElementById('puzzlesSolved').textContent = gameState.puzzlesSolved;
    document.getElementById('playerLevel').textContent = Math.floor(gameState.puzzlesSolved / 2) + 1;
}

// ==================== Canvas Drawing ====================

function drawLattice(canvas, basis, scale, origin, options = {}) {
    const ctx = canvas.getContext('2d');
    const [ox, oy] = origin;
    const range = options.range || 8;

    // Clear
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    // Draw grid lines (parallelogram grid)
    if (lattice.showGrid && !options.hideGrid) {
        ctx.strokeStyle = 'rgba(59, 130, 246, 0.1)';
        ctx.lineWidth = 1;

        // Draw grid lines along b1 direction
        for (let i = -range; i <= range; i++) {
            ctx.beginPath();
            for (let j = -range; j <= range; j++) {
                const x = ox + scale * (i * basis[0][0] + j * basis[1][0]);
                const y = oy - scale * (i * basis[0][1] + j * basis[1][1]);
                if (j === -range) {
                    ctx.moveTo(x, y);
                } else {
                    ctx.lineTo(x, y);
                }
            }
            ctx.stroke();
        }

        // Draw grid lines along b2 direction
        for (let j = -range; j <= range; j++) {
            ctx.beginPath();
            for (let i = -range; i <= range; i++) {
                const x = ox + scale * (i * basis[0][0] + j * basis[1][0]);
                const y = oy - scale * (i * basis[0][1] + j * basis[1][1]);
                if (i === -range) {
                    ctx.moveTo(x, y);
                } else {
                    ctx.lineTo(x, y);
                }
            }
            ctx.stroke();
        }
    }

    // Draw axes
    ctx.strokeStyle = '#cbd5e1';
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(0, oy);
    ctx.lineTo(canvas.width, oy);
    ctx.moveTo(ox, 0);
    ctx.lineTo(ox, canvas.height);
    ctx.stroke();

    // Draw lattice points
    for (let i = -range; i <= range; i++) {
        for (let j = -range; j <= range; j++) {
            const x = ox + scale * (i * basis[0][0] + j * basis[1][0]);
            const y = oy - scale * (i * basis[0][1] + j * basis[1][1]);

            if (x >= 0 && x <= canvas.width && y >= 0 && y <= canvas.height) {
                // Highlight origin
                if (i === 0 && j === 0) {
                    ctx.fillStyle = '#ef4444';
                    ctx.beginPath();
                    ctx.arc(x, y, 6, 0, 2 * Math.PI);
                    ctx.fill();
                } else {
                    // Normal lattice points with hover effect
                    const isHovered = lattice.hoveredPoint &&
                                    lattice.hoveredPoint[0] === i &&
                                    lattice.hoveredPoint[1] === j;

                    ctx.fillStyle = isHovered ? '#f59e0b' : '#3b82f6';
                    ctx.beginPath();
                    ctx.arc(x, y, isHovered ? 6 : 4, 0, 2 * Math.PI);
                    ctx.fill();

                    // Show coordinates on hover
                    if (isHovered) {
                        ctx.fillStyle = '#1e40af';
                        ctx.font = 'bold 12px Arial';
                        ctx.fillText(`(${i},${j})`, x + 10, y - 10);
                    }
                }
            }
        }
    }

    // Draw basis vectors
    if (!options.hideBasis) {
        // Vector 1
        ctx.strokeStyle = '#ef4444';
        ctx.lineWidth = 3;
        ctx.beginPath();
        ctx.moveTo(ox, oy);
        const v1x = ox + scale * basis[0][0];
        const v1y = oy - scale * basis[0][1];
        ctx.lineTo(v1x, v1y);
        ctx.stroke();
        drawArrow(ctx, ox, oy, v1x, v1y, '#ef4444');

        // Vector 2
        ctx.strokeStyle = '#10b981';
        ctx.beginPath();
        ctx.moveTo(ox, oy);
        const v2x = ox + scale * basis[1][0];
        const v2y = oy - scale * basis[1][1];
        ctx.lineTo(v2x, v2y);
        ctx.stroke();
        drawArrow(ctx, ox, oy, v2x, v2y, '#10b981');

        // Labels
        ctx.fillStyle = '#ef4444';
        ctx.font = 'bold 16px Arial';
        ctx.fillText('b₁', v1x + 10, v1y - 10);
        ctx.fillStyle = '#10b981';
        ctx.fillText('b₂', v2x + 10, v2y - 10);
    }

    // Draw selected vector if any
    if (options.selectedVector) {
        const [i, j] = options.selectedVector;
        const x = ox + scale * (i * basis[0][0] + j * basis[1][0]);
        const y = oy - scale * (i * basis[0][1] + j * basis[1][1]);

        ctx.strokeStyle = '#f59e0b';
        ctx.lineWidth = 4;
        ctx.beginPath();
        ctx.moveTo(ox, oy);
        ctx.lineTo(x, y);
        ctx.stroke();
        drawArrow(ctx, ox, oy, x, y, '#f59e0b');

        // Highlight point
        ctx.fillStyle = '#f59e0b';
        ctx.beginPath();
        ctx.arc(x, y, 8, 0, 2 * Math.PI);
        ctx.fill();
    }
}

function drawArrow(ctx, fromX, fromY, toX, toY, color) {
    const headlen = 15;
    const angle = Math.atan2(toY - fromY, toX - fromX);

    ctx.fillStyle = color;
    ctx.beginPath();
    ctx.moveTo(toX, toY);
    ctx.lineTo(toX - headlen * Math.cos(angle - Math.PI / 6),
               toY - headlen * Math.sin(angle - Math.PI / 6));
    ctx.lineTo(toX - headlen * Math.cos(angle + Math.PI / 6),
               toY - headlen * Math.sin(angle + Math.PI / 6));
    ctx.closePath();
    ctx.fill();
}

// ==================== Introduction Section ====================

function setupIntroCanvas() {
    const canvas = document.getElementById('introCanvas');
    drawLattice(canvas, lattice.basis, lattice.scale, lattice.origin);

    // Mouse interaction
    let isDragging = false;
    let dragVector = null;
    let lastMousePos = null;

    canvas.addEventListener('mousedown', (e) => {
        const rect = canvas.getBoundingClientRect();
        const mouseX = e.clientX - rect.left;
        const mouseY = e.clientY - rect.top;

        // Check if clicking on a basis vector tip
        const [ox, oy] = lattice.origin;
        const v1x = ox + lattice.scale * lattice.basis[0][0];
        const v1y = oy - lattice.scale * lattice.basis[0][1];
        const v2x = ox + lattice.scale * lattice.basis[1][0];
        const v2y = oy - lattice.scale * lattice.basis[1][1];

        const dist1 = Math.sqrt((mouseX - v1x) ** 2 + (mouseY - v1y) ** 2);
        const dist2 = Math.sqrt((mouseX - v2x) ** 2 + (mouseY - v2y) ** 2);

        if (dist1 < 20) {
            isDragging = true;
            dragVector = 0;
            canvas.style.cursor = 'grabbing';
        } else if (dist2 < 20) {
            isDragging = true;
            dragVector = 1;
            canvas.style.cursor = 'grabbing';
        }
    });

    canvas.addEventListener('mousemove', (e) => {
        const rect = canvas.getBoundingClientRect();
        const mouseX = e.clientX - rect.left;
        const mouseY = e.clientY - rect.top;

        if (isDragging && dragVector !== null) {
            // Update basis vector based on mouse position
            const [ox, oy] = lattice.origin;
            const newX = (mouseX - ox) / lattice.scale;
            const newY = -(mouseY - oy) / lattice.scale;

            // Clamp to reasonable values
            lattice.basis[dragVector][0] = Math.round(Math.max(-6, Math.min(6, newX)));
            lattice.basis[dragVector][1] = Math.round(Math.max(-6, Math.min(6, newY)));

            updateBasisDisplay();
            drawLattice(canvas, lattice.basis, lattice.scale, lattice.origin);
        } else {
            // Hover detection
            const [ox, oy] = lattice.origin;
            const range = 8;
            let found = false;

            for (let i = -range; i <= range; i++) {
                for (let j = -range; j <= range; j++) {
                    const x = ox + lattice.scale * (i * lattice.basis[0][0] + j * lattice.basis[1][0]);
                    const y = oy - lattice.scale * (i * lattice.basis[0][1] + j * lattice.basis[1][1]);

                    const dist = Math.sqrt((mouseX - x) ** 2 + (mouseY - y) ** 2);
                    if (dist < 10) {
                        lattice.hoveredPoint = [i, j];
                        found = true;
                        canvas.style.cursor = 'pointer';
                        break;
                    }
                }
                if (found) break;
            }

            // Check if hovering over vector tips
            const v1x = ox + lattice.scale * lattice.basis[0][0];
            const v1y = oy - lattice.scale * lattice.basis[0][1];
            const v2x = ox + lattice.scale * lattice.basis[1][0];
            const v2y = oy - lattice.scale * lattice.basis[1][1];

            const dist1 = Math.sqrt((mouseX - v1x) ** 2 + (mouseY - v1y) ** 2);
            const dist2 = Math.sqrt((mouseX - v2x) ** 2 + (mouseY - v2y) ** 2);

            if (dist1 < 20 || dist2 < 20) {
                canvas.style.cursor = 'grab';
                found = true;
            }

            if (!found && !isDragging) {
                lattice.hoveredPoint = null;
                canvas.style.cursor = 'default';
            }

            drawLattice(canvas, lattice.basis, lattice.scale, lattice.origin);
        }

        lastMousePos = [mouseX, mouseY];
    });

    canvas.addEventListener('mouseup', () => {
        isDragging = false;
        dragVector = null;
        canvas.style.cursor = 'default';
    });

    canvas.addEventListener('mouseleave', () => {
        isDragging = false;
        dragVector = null;
        lattice.hoveredPoint = null;
        canvas.style.cursor = 'default';
        drawLattice(canvas, lattice.basis, lattice.scale, lattice.origin);
    });

    // Zoom with mouse wheel
    canvas.addEventListener('wheel', (e) => {
        e.preventDefault();
        const delta = e.deltaY > 0 ? 0.9 : 1.1;
        lattice.scale = Math.max(10, Math.min(60, lattice.scale * delta));
        drawLattice(canvas, lattice.basis, lattice.scale, lattice.origin);
    });

    // Button handlers with animation
    document.getElementById('randomBasis').addEventListener('click', () => {
        const targetBasis = [
            [Math.floor(Math.random() * 5) + 1, Math.floor(Math.random() * 4) - 2],
            [Math.floor(Math.random() * 4) - 2, Math.floor(Math.random() * 5) + 1]
        ];
        animateBasisChange(canvas, lattice.basis, targetBasis);
    });

    document.getElementById('resetBasis').addEventListener('click', () => {
        const targetBasis = [[4, 1], [1, 3]];
        animateBasisChange(canvas, lattice.basis, targetBasis);
    });

    document.getElementById('toggleGrid').addEventListener('click', () => {
        lattice.showGrid = !lattice.showGrid;
        drawLattice(canvas, lattice.basis, lattice.scale, lattice.origin);
    });

    // Update basis display
    updateBasisDisplay();
}

// Update basis display
function updateBasisDisplay() {
    const b1 = lattice.basis[0];
    const b2 = lattice.basis[1];
    const det = Math.abs(b1[0] * b2[1] - b1[1] * b2[0]);

    document.getElementById('b1Display').textContent = `(${b1[0]}, ${b1[1]})`;
    document.getElementById('b2Display').textContent = `(${b2[0]}, ${b2[1]})`;
    document.getElementById('detDisplay').textContent = det;
}

// Animate basis change
function animateBasisChange(canvas, fromBasis, toBasis) {
    if (lattice.animating) return;

    lattice.animating = true;
    const steps = 30;
    let step = 0;

    const dx1 = (toBasis[0][0] - fromBasis[0][0]) / steps;
    const dy1 = (toBasis[0][1] - fromBasis[0][1]) / steps;
    const dx2 = (toBasis[1][0] - fromBasis[1][0]) / steps;
    const dy2 = (toBasis[1][1] - fromBasis[1][1]) / steps;

    function animate() {
        if (step >= steps) {
            lattice.basis = toBasis;
            lattice.animating = false;
            updateBasisDisplay();
            drawLattice(canvas, lattice.basis, lattice.scale, lattice.origin);
            return;
        }

        lattice.basis = [
            [fromBasis[0][0] + dx1 * step, fromBasis[0][1] + dy1 * step],
            [fromBasis[1][0] + dx2 * step, fromBasis[1][1] + dy2 * step]
        ];

        updateBasisDisplay();
        drawLattice(canvas, lattice.basis, lattice.scale, lattice.origin);
        step++;
        requestAnimationFrame(animate);
    }

    animate();
}

// ==================== Basics Section ====================

function setupBasicsCanvas() {
    const canvas = document.getElementById('basicsCanvas');
    const basis = [[3, 1], [1, 3]];
    const scale = 40;
    const origin = [250, 250];

    drawLattice(canvas, basis, scale, origin);

    canvas.addEventListener('click', (e) => {
        const rect = canvas.getBoundingClientRect();
        const clickX = e.clientX - rect.left;
        const clickY = e.clientY - rect.top;

        // Check if click is on a lattice point
        const range = 8;
        const threshold = 10;

        for (let i = -range; i <= range; i++) {
            for (let j = -range; j <= range; j++) {
                const x = origin[0] + scale * (i * basis[0][0] + j * basis[1][0]);
                const y = origin[1] - scale * (i * basis[0][1] + j * basis[1][1]);

                const dist = Math.sqrt((clickX - x) ** 2 + (clickY - y) ** 2);

                if (dist < threshold) {
                    const feedback = document.getElementById('basicsFeedback');
                    feedback.className = 'puzzle-feedback success';
                    feedback.textContent = `✓ Correct! This is lattice point (${i},${j}) = ${i}b₁ + ${j}b₂`;
                    return;
                }
            }
        }

        const feedback = document.getElementById('basicsFeedback');
        feedback.className = 'puzzle-feedback error';
        feedback.textContent = '✗ Not a lattice point. Try clicking on a blue dot!';
    });
}

// ==================== SVP Section ====================

let svpState = {
    selectedVector: null,
    shortestLength: Infinity
};

function setupSVPCanvas() {
    const canvas = document.getElementById('svpCanvas');
    const basis = [[5, 2], [2, 5]];
    const scale = 35;
    const origin = [250, 250];

    drawLattice(canvas, basis, scale, origin);

    canvas.addEventListener('click', (e) => {
        const rect = canvas.getBoundingClientRect();
        const clickX = e.clientX - rect.left;
        const clickY = e.clientY - rect.top;

        const range = 6;
        const threshold = 12;

        for (let i = -range; i <= range; i++) {
            for (let j = -range; j <= range; j++) {
                if (i === 0 && j === 0) continue; // Skip origin

                const x = origin[0] + scale * (i * basis[0][0] + j * basis[1][0]);
                const y = origin[1] - scale * (i * basis[0][1] + j * basis[1][1]);

                const dist = Math.sqrt((clickX - x) ** 2 + (clickY - y) ** 2);

                if (dist < threshold) {
                    svpState.selectedVector = [i, j];
                    const vx = i * basis[0][0] + j * basis[1][0];
                    const vy = i * basis[0][1] + j * basis[1][1];
                    const length = Math.sqrt(vx ** 2 + vy ** 2);

                    document.getElementById('selectedVector').textContent = `(${i}, ${j})`;
                    document.getElementById('vectorLength').textContent = length.toFixed(2);

                    if (length < svpState.shortestLength) {
                        svpState.shortestLength = length;
                        document.getElementById('shortestLength').textContent = length.toFixed(2);
                    }

                    drawLattice(canvas, basis, scale, origin, {
                        selectedVector: [i, j]
                    });
                    return;
                }
            }
        }
    });

    document.getElementById('checkSVP').addEventListener('click', () => {
        if (!svpState.selectedVector) {
            alert('Please select a vector first!');
            return;
        }

        const [i, j] = svpState.selectedVector;
        const vx = i * basis[0][0] + j * basis[1][0];
        const vy = i * basis[0][1] + j * basis[1][1];
        const length = Math.sqrt(vx ** 2 + vy ** 2);

        // The shortest vector for this basis is approximately (1,-1) or similar
        if (length < 4.5) {
            const feedback = document.getElementById('svpFeedback');
            feedback.className = 'puzzle-feedback success';
            feedback.textContent = `🎉 Excellent! Vector (${i},${j}) with length ${length.toFixed(2)} is very short!`;
        } else {
            const feedback = document.getElementById('svpFeedback');
            feedback.className = 'puzzle-feedback error';
            feedback.textContent = `Keep looking! There are shorter vectors. Try vectors closer to the origin.`;
        }
    });
}

// ==================== LLL Algorithm Demo ====================

function setupLLLDemo() {
    const canvas = document.getElementById('lllCanvas');

    document.getElementById('runLLL').addEventListener('click', () => {
        runLLLAlgorithm(false);
    });

    document.getElementById('stepLLL').addEventListener('click', () => {
        runLLLAlgorithm(true);
    });

    // Initial draw
    updateLLLCanvas();
}

function updateLLLCanvas() {
    const canvas = document.getElementById('lllCanvas');
    const b1x = parseInt(document.getElementById('b1x').value) || 15;
    const b1y = parseInt(document.getElementById('b1y').value) || 4;
    const b2x = parseInt(document.getElementById('b2x').value) || 8;
    const b2y = parseInt(document.getElementById('b2y').value) || 17;

    const basis = [[b1x, b1y], [b2x, b2y]];
    const scale = 15;
    const origin = [300, 250];

    drawLattice(canvas, basis, scale, origin, { range: 6 });
}

function runLLLAlgorithm(stepMode) {
    const canvas = document.getElementById('lllCanvas');
    const b1x = parseInt(document.getElementById('b1x').value) || 15;
    const b1y = parseInt(document.getElementById('b1y').value) || 4;
    const b2x = parseInt(document.getElementById('b2x').value) || 8;
    const b2y = parseInt(document.getElementById('b2y').value) || 17;

    let basis = [[b1x, b1y], [b2x, b2y]];

    document.getElementById('lllInfo').textContent = 'Running LLL algorithm...';

    // Simplified 2D LLL algorithm
    const delta = 0.75;
    let step = 0;

    function lllStep() {
        step++;

        // Gram-Schmidt orthogonalization
        const b1 = basis[0];
        const b2 = basis[1];

        const mu = (b1[0] * b2[0] + b1[1] * b2[1]) / (b1[0] ** 2 + b1[1] ** 2);

        // Size reduction
        if (Math.abs(mu) > 0.5) {
            const q = Math.round(mu);
            basis[1] = [b2[0] - q * b1[0], b2[1] - q * b1[1]];
            document.getElementById('lllInfo').textContent = `Step ${step}: Size reduction (subtract ${q}×b₁ from b₂)`;
        }

        // Check Lovász condition
        const b1Squared = b1[0] ** 2 + b1[1] ** 2;
        const b2Squared = basis[1][0] ** 2 + basis[1][1] ** 2;
        const proj = (b1[0] * basis[1][0] + b1[1] * basis[1][1]) / b1Squared;

        if (b2Squared < (delta - proj ** 2) * b1Squared) {
            // Swap
            [basis[0], basis[1]] = [basis[1], basis[0]];
            document.getElementById('lllInfo').textContent = `Step ${step}: Swap vectors (Lovász condition)`;

            if (step < 10) {
                setTimeout(lllStep, stepMode ? 2000 : 500);
            }
        } else {
            document.getElementById('lllInfo').textContent = `✓ LLL Complete! Reduced basis found in ${step} steps`;
        }

        document.getElementById('lllStep').textContent = `Step: ${step}`;
        const scale = 15;
        const origin = [300, 250];
        drawLattice(canvas, basis, scale, origin, { range: 6 });
    }

    lllStep();
}

// ==================== Attack Demo ====================

function setupAttackDemo() {
    document.getElementById('performAttack').addEventListener('click', performLatticeAttack);
}

function performLatticeAttack() {
    const progressDiv = document.getElementById('attackProgress');
    const resultDiv = document.getElementById('attackResult');

    progressDiv.classList.add('active');
    progressDiv.innerHTML = `
        <div class="progress-step">
            <span class="step-icon">⚙️</span>
            <span class="step-text">Constructing lattice from ciphertexts...</span>
        </div>
    `;

    setTimeout(() => {
        progressDiv.innerHTML += `
            <div class="progress-step">
                <span class="step-icon">🔬</span>
                <span class="step-text">Running LLL algorithm...</span>
            </div>
        `;
    }, 1000);

    setTimeout(() => {
        progressDiv.innerHTML += `
            <div class="progress-step">
                <span class="step-icon">🔍</span>
                <span class="step-text">Finding short vectors...</span>
            </div>
        `;
    }, 2000);

    setTimeout(() => {
        progressDiv.innerHTML += `
            <div class="progress-step">
                <span class="step-icon">🎯</span>
                <span class="step-text">Recovering plaintext...</span>
            </div>
        `;
    }, 3000);

    setTimeout(() => {
        resultDiv.classList.add('active');
        resultDiv.innerHTML = `
            <h3>🎉 Attack Successful!</h3>
            <p><strong>Recovered Message:</strong></p>
            <div style="background: white; padding: 15px; border-radius: 8px; margin: 10px 0; font-family: monospace;">
                "Hello from Alice! This is a secret message."
            </div>
            <p style="margin-top: 15px;">
                <strong>Explanation:</strong> Using the Chinese Remainder Theorem and lattice reduction,
                we constructed a lattice where the plaintext appears as a short vector.
                The LLL algorithm found this vector, revealing the message!
            </p>
        `;
        progressDiv.classList.remove('active');
    }, 4000);
}

// ==================== Save/Load ====================

function saveGameState() {
    localStorage.setItem('lattice_save', JSON.stringify(gameState));
}

function loadGameState() {
    const saved = localStorage.getItem('lattice_save');
    if (saved) {
        const loaded = JSON.parse(saved);
        gameState = { ...gameState, ...loaded };
        updateStats();

        // Unlock sections
        gameState.unlockedSections.forEach(section => {
            document.querySelector(`[data-section="${section}"]`)?.classList.remove('locked');
        });
    }
}

// Make unlockSection available globally
window.unlockSection = unlockSection;
