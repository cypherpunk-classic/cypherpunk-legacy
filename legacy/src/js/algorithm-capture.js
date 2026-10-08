// ==================== Algorithm Capture - Cryptomon Game ====================

// Algorithm Database (Pokemon-inspired crypto algorithms)
const ALGORITHMS = {
    // Basic Tier
    'aes-basic': {
        id: 'aes-basic',
        name: 'AES-Cipher',
        sprite: '🔐',
        types: ['symmetric', 'block'],
        baseHP: 100,
        attack: 45,
        defense: 60,
        rarity: 'common',
        evolution: { level: 20, target: 'aes-advanced' },
        description: 'A fundamental block cipher algorithm. Uses symmetric key encryption for secure data protection.'
    },
    'rsa-basic': {
        id: 'rsa-basic',
        name: 'RSA-Key',
        sprite: '🔑',
        types: ['asymmetric'],
        baseHP: 80,
        attack: 70,
        defense: 40,
        rarity: 'common',
        evolution: { level: 20, target: 'rsa-advanced' },
        description: 'Uses public-key cryptography. Strong attack but vulnerable defense due to computational complexity.'
    },
    'sha-basic': {
        id: 'sha-basic',
        name: 'SHA-Hash',
        sprite: '🏷️',
        types: ['hash'],
        baseHP: 90,
        attack: 55,
        defense: 50,
        rarity: 'common',
        evolution: { level: 20, target: 'sha-advanced' },
        description: 'Creates fixed-size digest from input. One-way function that ensures data integrity.'
    },
    'rc4-basic': {
        id: 'rc4-basic',
        name: 'RC4-Stream',
        sprite: '🌊',
        types: ['stream', 'symmetric'],
        baseHP: 70,
        attack: 65,
        defense: 35,
        rarity: 'common',
        evolution: { level: 20, target: 'chacha' },
        description: 'Fast stream cipher. Quick attacks but lower defense. Generates keystream for encryption.'
    },

    // Advanced Tier
    'aes-advanced': {
        id: 'aes-advanced',
        name: 'AES-256',
        sprite: '🛡️',
        types: ['symmetric', 'block'],
        baseHP: 150,
        attack: 75,
        defense: 95,
        rarity: 'uncommon',
        evolution: { level: 40, target: 'aes-gcm' },
        description: 'Evolved AES with 256-bit key. Industry standard with exceptional defense capabilities.'
    },
    'rsa-advanced': {
        id: 'rsa-advanced',
        name: 'RSA-2048',
        sprite: '🗝️',
        types: ['asymmetric'],
        baseHP: 120,
        attack: 110,
        defense: 70,
        rarity: 'uncommon',
        evolution: { level: 40, target: 'rsa-pss' },
        description: 'Enhanced RSA with 2048-bit modulus. Powerful asymmetric encryption for secure communication.'
    },
    'sha-advanced': {
        id: 'sha-advanced',
        name: 'SHA-256',
        sprite: '🎯',
        types: ['hash'],
        baseHP: 140,
        attack: 85,
        defense: 80,
        rarity: 'uncommon',
        evolution: { level: 40, target: 'sha3' },
        description: 'Cryptographic hash function producing 256-bit digest. Widely used in blockchain technology.'
    },
    'chacha': {
        id: 'chacha',
        name: 'ChaCha20',
        sprite: '💨',
        types: ['stream', 'symmetric'],
        baseHP: 110,
        attack: 95,
        defense: 65,
        rarity: 'uncommon',
        description: 'Modern stream cipher with enhanced security. Fast and secure alternative to RC4.'
    },

    // Master Tier
    'aes-gcm': {
        id: 'aes-gcm',
        name: 'AES-GCM',
        sprite: '💎',
        types: ['symmetric', 'block'],
        baseHP: 200,
        attack: 105,
        defense: 130,
        rarity: 'rare',
        description: 'AES in Galois/Counter Mode. Provides both encryption and authentication. Ultimate defense!'
    },
    'rsa-pss': {
        id: 'rsa-pss',
        name: 'RSA-PSS',
        sprite: '👑',
        types: ['asymmetric'],
        baseHP: 180,
        attack: 150,
        defense: 100,
        rarity: 'rare',
        description: 'RSA with Probabilistic Signature Scheme. Provably secure signature algorithm with maximum attack power.'
    },
    'sha3': {
        id: 'sha3',
        name: 'SHA-3',
        sprite: '⭐',
        types: ['hash'],
        baseHP: 190,
        attack: 120,
        defense: 110,
        rarity: 'rare',
        description: 'Latest SHA standard using sponge construction. Resistant to length extension attacks.'
    },

    // Special/Fusion Only
    'hybrid-crypto': {
        id: 'hybrid-crypto',
        name: 'Hybrid-System',
        sprite: '🌟',
        types: ['symmetric', 'asymmetric'],
        baseHP: 220,
        attack: 140,
        defense: 140,
        rarity: 'legendary',
        description: 'Fusion of symmetric and asymmetric encryption. Combines speed of AES with security of RSA!'
    },
    'authenticated-encryption': {
        id: 'authenticated-encryption',
        name: 'AuthEncrypt',
        sprite: '✨',
        types: ['symmetric', 'hash'],
        baseHP: 210,
        attack: 130,
        defense: 150,
        rarity: 'legendary',
        description: 'Combines encryption with authentication. Provides confidentiality and integrity simultaneously!'
    },
    'quantum-resistant': {
        id: 'quantum-resistant',
        name: 'Lattice-Crypto',
        sprite: '🔮',
        types: ['quantum', 'asymmetric'],
        baseHP: 250,
        attack: 160,
        defense: 160,
        rarity: 'legendary',
        description: 'Post-quantum cryptography using lattice problems. Resistant to quantum computer attacks!'
    }
};

// Fusion Recipes
const FUSION_RECIPES = [
    { alg1: 'aes-gcm', alg2: 'rsa-pss', result: 'hybrid-crypto' },
    { alg1: 'aes-advanced', alg2: 'sha-advanced', result: 'authenticated-encryption' },
    { alg1: 'rsa-pss', alg2: 'sha3', result: 'quantum-resistant' }
];

// Game State
let gameState = {
    playerName: 'Crypto Master',
    collection: {},
    currentWild: null,
    currentPlayer: null,
    battleState: 'idle'
};

// Initialize game
function initGame() {
    loadGameState();
    setupEventListeners();
    updateUI();
    spawnWildAlgorithm();
    renderCollection();
    renderFusionRecipes();
}

// Setup Event Listeners
function setupEventListeners() {
    // Tab switching
    document.querySelectorAll('.tab-btn').forEach(btn => {
        btn.addEventListener('click', (e) => {
            const view = e.currentTarget.dataset.view;
            switchView(view);
        });
    });

    // Battle actions
    document.getElementById('attackBtn').addEventListener('click', handleAttack);
    document.getElementById('captureBtn').addEventListener('click', handleCapture);
    document.getElementById('analyzeBtn').addEventListener('click', handleAnalyze);
    document.getElementById('fleeBtn').addEventListener('click', handleFlee);

    // Fusion
    document.getElementById('fuseBtn').addEventListener('click', handleFusion);

    // Modal close
    document.getElementById('closeModal').addEventListener('click', closeSelectionModal);

    // Click on player algorithm area to change
    document.getElementById('playerAlgorithm').addEventListener('click', openSelectionModal);
}

// View Switching
function switchView(viewName) {
    document.querySelectorAll('.tab-btn').forEach(btn => btn.classList.remove('active'));
    document.querySelectorAll('.view-content').forEach(view => view.classList.remove('active'));

    document.querySelector(`[data-view="${viewName}"]`).classList.add('active');
    document.getElementById(`${viewName}View`).classList.add('active');

    if (viewName === 'evolution') {
        renderEvolutionView();
    } else if (viewName === 'fusion') {
        renderFusionView();
    }
}

// Spawn Wild Algorithm
function spawnWildAlgorithm() {
    const basicAlgos = ['aes-basic', 'rsa-basic', 'sha-basic', 'rc4-basic'];
    const randomId = basicAlgos[Math.floor(Math.random() * basicAlgos.length)];
    const algo = { ...ALGORITHMS[randomId] };

    algo.level = Math.floor(Math.random() * 10) + 1;
    algo.currentHP = algo.baseHP + (algo.level * 5);
    algo.maxHP = algo.currentHP;

    gameState.currentWild = algo;
    gameState.battleState = 'encounter';

    renderWildAlgorithm();
    addBattleLog(`A wild ${algo.name} Lv.${algo.level} appeared!`);
}

// Render Wild Algorithm
function renderWildAlgorithm() {
    const wild = gameState.currentWild;
    if (!wild) return;

    document.getElementById('wildName').textContent = `${wild.name} Lv.${wild.level}`;
    document.getElementById('wildSprite').textContent = wild.sprite;

    const typesContainer = document.getElementById('wildTypes');
    typesContainer.innerHTML = wild.types.map(t =>
        `<span class="type-badge ${t}">${t}</span>`
    ).join('');

    updateHP('wild', wild.currentHP, wild.maxHP);
}

// Render Player Algorithm
function renderPlayerAlgorithm() {
    const player = gameState.currentPlayer;
    if (!player) {
        document.getElementById('playerAlgoName').textContent = 'Select Algorithm';
        document.getElementById('playerSprite').textContent = '❓';
        document.getElementById('playerTypes').innerHTML = '';
        updateHP('player', 0, 100);
        return;
    }

    document.getElementById('playerAlgoName').textContent = `${player.name} Lv.${player.level}`;
    document.getElementById('playerSprite').textContent = player.sprite;

    const typesContainer = document.getElementById('playerTypes');
    typesContainer.innerHTML = player.types.map(t =>
        `<span class="type-badge ${t}">${t}</span>`
    ).join('');

    updateHP('player', player.currentHP, player.maxHP);
}

// Update HP Bar
function updateHP(target, current, max) {
    const percent = (current / max) * 100;
    const fill = document.getElementById(`${target}HP`);
    const text = document.getElementById(`${target}HPText`);

    fill.style.width = `${percent}%`;
    text.textContent = `${current}/${max}`;

    if (percent < 30) {
        fill.classList.add('low');
    } else {
        fill.classList.remove('low');
    }
}

// Battle Actions
function handleAttack() {
    if (!gameState.currentPlayer) {
        selectPlayerAlgorithm();
        return;
    }

    const player = gameState.currentPlayer;
    const wild = gameState.currentWild;

    // Calculate damage
    const effectiveness = calculateEffectiveness(player.types, wild.types);
    const damage = Math.floor((player.attack + player.level * 2) * effectiveness);

    wild.currentHP = Math.max(0, wild.currentHP - damage);
    renderWildAlgorithm();

    addBattleLog(`${player.name} attacks! Deals ${damage} damage!`);

    if (wild.currentHP === 0) {
        handleWildDefeated();
        return;
    }

    // Wild counter-attack
    setTimeout(() => {
        const counterDamage = Math.floor((wild.attack + wild.level * 2) * 0.8);
        player.currentHP = Math.max(0, player.currentHP - counterDamage);
        renderPlayerAlgorithm();
        addBattleLog(`${wild.name} counter-attacks! Deals ${counterDamage} damage!`);

        if (player.currentHP === 0) {
            addBattleLog(`${player.name} fainted! Selecting new algorithm...`);
            gameState.currentPlayer = null;
            renderPlayerAlgorithm();
        }
    }, 1000);
}

function handleCapture() {
    const wild = gameState.currentWild;
    const captureRate = calculateCaptureRate(wild);

    showCaptureAnimation();

    setTimeout(() => {
        if (Math.random() < captureRate) {
            // Success!
            captureAlgorithm(wild);
            addBattleLog(`Success! ${wild.name} was captured!`);
            hideCaptureAnimation('success');

            setTimeout(() => {
                spawnWildAlgorithm();
                gameState.currentPlayer = null;
                renderPlayerAlgorithm();
            }, 2000);
        } else {
            // Failed
            addBattleLog(`Oh no! ${wild.name} broke free!`);
            hideCaptureAnimation('failed');
        }
    }, 2000);
}

function handleAnalyze() {
    const wild = gameState.currentWild;
    showAlgorithmDetail(wild);
    addBattleLog(`Analyzing ${wild.name}...`);
}

function handleFlee() {
    addBattleLog('You fled from the battle!');
    spawnWildAlgorithm();
    gameState.currentPlayer = null;
    renderPlayerAlgorithm();
}

// Calculate Effectiveness (Type advantages)
function calculateEffectiveness(attackTypes, defenseTypes) {
    const advantages = {
        'symmetric': ['stream'],
        'asymmetric': ['symmetric'],
        'hash': ['asymmetric'],
        'stream': ['hash']
    };

    let multiplier = 1.0;
    attackTypes.forEach(atkType => {
        defenseTypes.forEach(defType => {
            if (advantages[atkType] && advantages[atkType].includes(defType)) {
                multiplier *= 1.5;
            }
        });
    });

    return multiplier;
}

// Calculate Capture Rate
function calculateCaptureRate(wild) {
    const hpFactor = 1 - (wild.currentHP / wild.maxHP);
    const rarityFactor = {
        'common': 0.5,
        'uncommon': 0.3,
        'rare': 0.15,
        'legendary': 0.05
    }[wild.rarity] || 0.3;

    return Math.min(0.95, rarityFactor + hpFactor * 0.4);
}

// Capture Algorithm
function captureAlgorithm(algo) {
    if (!gameState.collection[algo.id]) {
        gameState.collection[algo.id] = {
            ...algo,
            caught: true,
            level: algo.level,
            exp: 0
        };
    } else {
        // Already have it, just add exp
        gameState.collection[algo.id].exp += 50;
        checkLevelUp(gameState.collection[algo.id]);
    }

    saveGameState();
    renderCollection();
    updateUI();
}

// Level Up
function checkLevelUp(algo) {
    const expNeeded = algo.level * 100;
    if (algo.exp >= expNeeded) {
        algo.level++;
        algo.exp = 0;
        algo.maxHP = algo.baseHP + (algo.level * 5);
        algo.currentHP = algo.maxHP;

        addBattleLog(`${algo.name} leveled up to Lv.${algo.level}!`);

        // Check evolution
        const template = ALGORITHMS[algo.id];
        if (template.evolution && algo.level >= template.evolution.level) {
            addBattleLog(`${algo.name} can now evolve!`);
        }
    }
}

// Wild Defeated
function handleWildDefeated() {
    addBattleLog(`${gameState.currentWild.name} was defeated!`);

    if (gameState.currentPlayer) {
        gameState.collection[gameState.currentPlayer.id].exp += 30;
        checkLevelUp(gameState.collection[gameState.currentPlayer.id]);
    }

    setTimeout(() => {
        spawnWildAlgorithm();
    }, 2000);
}

// Select Player Algorithm
function selectPlayerAlgorithm() {
    const collection = Object.values(gameState.collection).filter(a => a.caught);
    if (collection.length === 0) {
        addBattleLog('You have no algorithms! Try capturing one first.');
        return;
    }

    // Auto-select first algorithm for now
    const selected = collection[0];
    gameState.currentPlayer = {
        ...selected,
        currentHP: selected.maxHP,
        attack: ALGORITHMS[selected.id].attack + selected.level * 2,
        defense: ALGORITHMS[selected.id].defense + selected.level * 2
    };

    renderPlayerAlgorithm();
    addBattleLog(`Go! ${selected.name}!`);
}

// Open Selection Modal
function openSelectionModal() {
    const collection = Object.values(gameState.collection).filter(a => a.caught);
    if (collection.length === 0) {
        addBattleLog('You have no algorithms! Try capturing one first.');
        return;
    }

    const modal = document.getElementById('selectionModal');
    const grid = document.getElementById('selectionGrid');
    grid.innerHTML = '';

    collection.forEach(algo => {
        const template = ALGORITHMS[algo.id];
        const card = document.createElement('div');
        card.className = 'selection-card';
        card.innerHTML = `
            <div class="selection-card-sprite">${template.sprite}</div>
            <div class="selection-card-name">${template.name}</div>
            <div class="selection-card-level">Lv. ${algo.level}</div>
            <div class="selection-card-hp">HP: ${algo.maxHP}</div>
            <div class="type-badges">
                ${template.types.map(t => `<span class="type-badge ${t}">${t.slice(0,3)}</span>`).join('')}
            </div>
        `;
        card.addEventListener('click', () => {
            selectAlgorithmForBattle(algo);
            closeSelectionModal();
        });
        grid.appendChild(card);
    });

    modal.classList.add('active');
}

// Close Selection Modal
function closeSelectionModal() {
    document.getElementById('selectionModal').classList.remove('active');
}

// Select Algorithm for Battle
function selectAlgorithmForBattle(algo) {
    const template = ALGORITHMS[algo.id];
    gameState.currentPlayer = {
        ...algo,
        name: template.name,
        sprite: template.sprite,
        types: template.types,
        currentHP: algo.maxHP,
        attack: template.attack + algo.level * 2,
        defense: template.defense + algo.level * 2
    };

    renderPlayerAlgorithm();
    addBattleLog(`Go! ${template.name}!`);
}

// Render Collection
function renderCollection() {
    const grid = document.getElementById('collectionGrid');
    grid.innerHTML = '';

    Object.values(ALGORITHMS).forEach(template => {
        const caught = gameState.collection[template.id];
        const card = document.createElement('div');
        card.className = `collection-card ${!caught ? 'locked' : ''}`;

        if (caught) {
            card.innerHTML = `
                <div class="card-sprite">${template.sprite}</div>
                <div class="card-name">${template.name}</div>
                <div class="card-level">Lv. ${caught.level}</div>
                <div class="type-badges">
                    ${template.types.map(t => `<span class="type-badge ${t}">${t}</span>`).join('')}
                </div>
            `;
            card.addEventListener('click', () => showAlgorithmDetail(caught));
        } else {
            card.innerHTML = `
                <div class="card-sprite">❓</div>
                <div class="card-name">???</div>
                <div class="card-level">Not captured</div>
            `;
        }

        grid.appendChild(card);
    });

    updateUI();
}

// Show Algorithm Detail
function showAlgorithmDetail(algo) {
    const template = ALGORITHMS[algo.id];
    const detail = document.getElementById('dexDetail');

    detail.innerHTML = `
        <div class="dex-sprite">${template.sprite}</div>
        <div class="dex-name">${template.name} ${algo.level ? `Lv.${algo.level}` : ''}</div>
        <div class="type-badges">
            ${template.types.map(t => `<span class="type-badge ${t}">${t}</span>`).join('')}
        </div>
        <div class="dex-description">${template.description}</div>
        <div class="dex-stats-detail">
            <div class="stat-row">
                <span>HP:</span>
                <span>${template.baseHP}${algo.level ? ` + ${algo.level * 5}` : ''}</span>
            </div>
            <div class="stat-row">
                <span>Attack:</span>
                <span>${template.attack}</span>
            </div>
            <div class="stat-row">
                <span>Defense:</span>
                <span>${template.defense}</span>
            </div>
            <div class="stat-row">
                <span>Rarity:</span>
                <span>${template.rarity}</span>
            </div>
        </div>
    `;
}

// Evolution System
function renderEvolutionView() {
    const selectContainer = document.getElementById('evolutionSelect');
    selectContainer.innerHTML = '';

    const evolvable = Object.values(gameState.collection).filter(algo => {
        const template = ALGORITHMS[algo.id];
        return algo.caught && template.evolution && algo.level >= template.evolution.level;
    });

    if (evolvable.length === 0) {
        selectContainer.innerHTML = '<p class="hint-text">No algorithms ready to evolve. Keep leveling up!</p>';
        return;
    }

    evolvable.forEach(algo => {
        const template = ALGORITHMS[algo.id];
        const card = document.createElement('div');
        card.className = 'evolution-card';
        card.innerHTML = `
            <div style="font-size: 32px;">${template.sprite}</div>
            <div>
                <div style="font-weight: bold;">${template.name}</div>
                <div style="font-size: 12px; color: #7f8c8d;">Lv. ${algo.level} - Ready!</div>
            </div>
        `;
        card.addEventListener('click', () => showEvolutionPreview(algo));
        selectContainer.appendChild(card);
    });
}

function showEvolutionPreview(algo) {
    const template = ALGORITHMS[algo.id];
    const evolution = ALGORITHMS[template.evolution.target];
    const preview = document.getElementById('evolutionPreview');

    preview.innerHTML = `
        <div style="font-size: 64px;">${template.sprite}</div>
        <div style="font-size: 20px; font-weight: bold;">${template.name}</div>
        <div class="evolution-arrow">⬇️</div>
        <div style="font-size: 64px;">${evolution.sprite}</div>
        <div style="font-size: 24px; font-weight: bold; color: #27ae60;">${evolution.name}</div>
        <button class="evolution-btn" onclick="evolveAlgorithm('${algo.id}')">Evolve Now!</button>
    `;
}

function evolveAlgorithm(algoId) {
    const algo = gameState.collection[algoId];
    const template = ALGORITHMS[algoId];
    const evolution = ALGORITHMS[template.evolution.target];

    // Create new evolved algorithm
    gameState.collection[evolution.id] = {
        id: evolution.id,
        caught: true,
        level: algo.level,
        exp: 0,
        maxHP: evolution.baseHP + (algo.level * 5),
        currentHP: evolution.baseHP + (algo.level * 5)
    };

    // Remove old one
    delete gameState.collection[algoId];

    saveGameState();
    renderCollection();
    renderEvolutionView();

    addBattleLog(`${template.name} evolved into ${evolution.name}!`);
    alert(`Congratulations! Your ${template.name} evolved into ${evolution.name}!`);
}

// Fusion System
function renderFusionView() {
    const slot1 = document.querySelector('[data-slot="1"]');
    const slot2 = document.querySelector('[data-slot="2"]');

    slot1.onclick = () => selectForFusion(1);
    slot2.onclick = () => selectForFusion(2);
}

let fusionSlots = { slot1: null, slot2: null };

function selectForFusion(slotNum) {
    const available = Object.values(gameState.collection).filter(a => a.caught);
    if (available.length === 0) {
        alert('You need to capture algorithms first!');
        return;
    }

    // Simple selection (could be enhanced with a modal)
    const selected = available[Math.min(slotNum - 1, available.length - 1)];
    fusionSlots[`slot${slotNum}`] = selected;

    const slotContent = document.querySelector(`[data-slot="${slotNum}"]`);
    slotContent.textContent = ALGORITHMS[selected.id].sprite;
    slotContent.style.fontSize = '48px';

    checkFusionRecipe();
}

function checkFusionRecipe() {
    if (!fusionSlots.slot1 || !fusionSlots.slot2) return;

    const recipe = FUSION_RECIPES.find(r =>
        (r.alg1 === fusionSlots.slot1.id && r.alg2 === fusionSlots.slot2.id) ||
        (r.alg2 === fusionSlots.slot1.id && r.alg1 === fusionSlots.slot2.id)
    );

    const resultContent = document.querySelector('.result-content');
    const fuseBtn = document.getElementById('fuseBtn');

    if (recipe) {
        const result = ALGORITHMS[recipe.result];
        resultContent.textContent = result.sprite;
        resultContent.style.fontSize = '48px';
        fuseBtn.disabled = false;
    } else {
        resultContent.textContent = '❌';
        resultContent.style.fontSize = '48px';
        fuseBtn.disabled = true;
    }
}

function handleFusion() {
    const recipe = FUSION_RECIPES.find(r =>
        (r.alg1 === fusionSlots.slot1.id && r.alg2 === fusionSlots.slot2.id) ||
        (r.alg2 === fusionSlots.slot1.id && r.alg1 === fusionSlots.slot2.id)
    );

    if (!recipe) return;

    const result = ALGORITHMS[recipe.result];

    // Create fused algorithm
    const avgLevel = Math.floor((fusionSlots.slot1.level + fusionSlots.slot2.level) / 2);
    gameState.collection[result.id] = {
        id: result.id,
        caught: true,
        level: avgLevel,
        exp: 0,
        maxHP: result.baseHP + (avgLevel * 5),
        currentHP: result.baseHP + (avgLevel * 5)
    };

    saveGameState();
    renderCollection();

    alert(`Fusion successful! Created ${result.name}!`);
    addBattleLog(`${fusionSlots.slot1.id} + ${fusionSlots.slot2.id} = ${result.name}!`);

    // Reset fusion
    fusionSlots = { slot1: null, slot2: null };
    document.querySelector('[data-slot="1"]').textContent = '+';
    document.querySelector('[data-slot="2"]').textContent = '+';
    document.querySelector('.result-content').textContent = '?';
    document.getElementById('fuseBtn').disabled = true;
}

function renderFusionRecipes() {
    const list = document.getElementById('recipeList');
    list.innerHTML = FUSION_RECIPES.map(r => {
        const alg1 = ALGORITHMS[r.alg1];
        const alg2 = ALGORITHMS[r.alg2];
        const result = ALGORITHMS[r.result];
        return `
            <div class="recipe-item">
                ${alg1.sprite} ${alg1.name} + ${alg2.sprite} ${alg2.name} = ${result.sprite} ${result.name}
            </div>
        `;
    }).join('');
}

// Battle Log
function addBattleLog(message) {
    const log = document.getElementById('battleLog');
    const entry = document.createElement('div');
    entry.className = 'log-entry';
    entry.textContent = message;
    log.appendChild(entry);
    log.scrollTop = log.scrollHeight;
}

// Capture Animation
function showCaptureAnimation() {
    const overlay = document.getElementById('captureOverlay');
    const text = document.getElementById('captureText');
    text.textContent = 'Attempting capture...';
    overlay.classList.add('active');
}

function hideCaptureAnimation(result) {
    const text = document.getElementById('captureText');
    if (result === 'success') {
        text.textContent = '✓ Captured!';
        text.style.color = '#2ecc71';
    } else {
        text.textContent = '✗ Failed!';
        text.style.color = '#e74c3c';
    }

    setTimeout(() => {
        document.getElementById('captureOverlay').classList.remove('active');
    }, 1500);
}

// Update UI
function updateUI() {
    const caughtCount = Object.values(gameState.collection).filter(a => a.caught).length;
    const totalCount = Object.keys(ALGORITHMS).length;

    document.getElementById('caughtCount').textContent = caughtCount;
    document.getElementById('totalCount').textContent = totalCount;
}

// Save/Load Game State
function saveGameState() {
    localStorage.setItem('cryptomon_save', JSON.stringify(gameState));
}

function loadGameState() {
    const saved = localStorage.getItem('cryptomon_save');
    if (saved) {
        const loaded = JSON.parse(saved);
        gameState.collection = loaded.collection || {};
    }
}

// Initialize on page load
window.addEventListener('DOMContentLoaded', initGame);
