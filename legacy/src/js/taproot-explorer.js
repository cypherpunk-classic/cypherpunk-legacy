// ==================== Game State ====================
let currentStage = 0;
let score = 0;
let completedChallenges = [false, false, false, false, false];
let gameData = {
    // Challenge 1: Nonce Reuse Attack
    leakedData: null,
    recoveredKey: null,
    forgedSignature: null,

    // Challenge 2: Key Aggregation
    aliceKey: null,
    bobKey: null,
    aggregatedKey: null,
    multiSigAddress: null,

    // Challenge 3: Taproot Builder
    internalKey: null,
    scriptTree: [],
    merkleRoot: null,
    tweakedKey: null,
    taprootAddress: null,

    // Challenge 4: MAST Detective
    mysteryTx: null,
    identifiedScript: null
};

// ==================== Utility Functions ====================

// Simple hash function (SHA-256 simulation)
function simpleHash(input) {
    let hash = 0;
    for (let i = 0; i < input.length; i++) {
        const char = input.charCodeAt(i);
        hash = ((hash << 5) - hash) + char;
        hash = hash & hash;
    }
    return Math.abs(hash).toString(16).padStart(64, '0');
}

// Generate random hex string
function randomHex(length) {
    const bytes = new Uint8Array(length / 2);
    crypto.getRandomValues(bytes);
    return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('');
}

// Simulate BigInt modular arithmetic
function modAdd(a, b) {
    // Simplified mod addition
    const sum = BigInt('0x' + a) + BigInt('0x' + b);
    return (sum % BigInt('0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141')).toString(16).padStart(64, '0');
}

function modSub(a, b) {
    // Simplified mod subtraction
    const diff = BigInt('0x' + a) - BigInt('0x' + b);
    const modulus = BigInt('0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141');
    return ((diff % modulus + modulus) % modulus).toString(16).padStart(64, '0');
}

function modMultiply(a, b) {
    // Simplified mod multiplication
    const product = BigInt('0x' + a) * BigInt('0x' + b);
    return (product % BigInt('0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141')).toString(16).padStart(64, '0');
}

function modInverse(a) {
    // Simplified modular inverse (not cryptographically secure)
    return simpleHash('inv_' + a);
}

// Simulate elliptic curve point operations
function pointMultiply(scalar, basePoint = 'G') {
    return simpleHash(scalar + basePoint);
}

function pointAdd(point1, point2) {
    return simpleHash(point1 + point2);
}

function pointSubtract(point1, point2) {
    // Point subtraction: P1 - P2
    return simpleHash(point1 + 'minus' + point2);
}

// ==================== Stage Navigation ====================

function switchStage(stageNum) {
    // Hide all stages
    document.querySelectorAll('.stage').forEach(stage => {
        stage.classList.remove('active');
    });

    // Show selected stage
    const selectedStage = document.getElementById(`stage-${stageNum}`);
    if (selectedStage) {
        selectedStage.classList.add('active');
        currentStage = stageNum;

        // Update stage buttons
        document.querySelectorAll('.stage-btn').forEach((btn, index) => {
            btn.classList.remove('active');
            if (index === stageNum) {
                btn.classList.add('active');
            }
        });

        // Scroll to top
        window.scrollTo({ top: 0, behavior: 'smooth' });
    }
}

function updateScore(points) {
    score += points;
    const scoreElement = document.getElementById('score');
    if (scoreElement) {
        scoreElement.textContent = score;

        // Animation
        scoreElement.style.transform = 'scale(1.3)';
        scoreElement.style.color = '#f7931a';
        setTimeout(() => {
            scoreElement.style.transform = 'scale(1)';
            scoreElement.style.color = '#4ade80';
        }, 300);
    }
}

function completeChallenge(challengeNum, points) {
    if (completedChallenges[challengeNum]) {
        alert('You already completed this challenge!');
        return;
    }

    completedChallenges[challengeNum] = true;
    updateScore(points);

    // Enable next challenge button
    const nextBtn = document.getElementById(`next-to-${challengeNum + 1}`);
    if (nextBtn) {
        nextBtn.disabled = false;
    }

    // Unlock next stage button in stage selector
    const nextStageBtn = document.getElementById(`stage-${challengeNum + 1}-btn`);
    if (nextStageBtn) {
        nextStageBtn.disabled = false;
        nextStageBtn.classList.remove('locked');
    }

    console.log(`Challenge ${challengeNum} completed! +${points} points`);
}

// ==================== Challenge 1: Nonce Reuse Attack ====================

function generateLeakedData() {
    // Generate "Satoshi's" private key (secret)
    const satoshiPrivateKey = randomHex(64);
    const satoshiPublicKey = pointMultiply(satoshiPrivateKey, 'G');

    // Generate first message and signature
    const message1 = 'I am Satoshi Nakamoto';
    const nonce = randomHex(64); // SAME nonce for both signatures (the vulnerability!)
    const R = pointMultiply(nonce, 'G'); // R = k*G

    // First signature
    const e1 = simpleHash(R + satoshiPublicKey + message1);
    const s1 = modAdd(nonce, simpleHash(e1 + satoshiPrivateKey)); // s = k + e*d

    // Second message with SAME nonce (the leak!)
    const message2 = 'Bitcoin is the future';
    const e2 = simpleHash(R + satoshiPublicKey + message2);
    const s2 = modAdd(nonce, simpleHash(e2 + satoshiPrivateKey)); // s = k + e*d

    gameData.leakedData = {
        publicKey: satoshiPublicKey,
        message1: message1,
        message2: message2,
        R: R,
        s1: s1,
        s2: s2,
        e1: e1,
        e2: e2,
        actualPrivateKey: satoshiPrivateKey,
        nonce: nonce
    };

    // Display leaked data
    document.getElementById('leaked-pubkey').textContent = satoshiPublicKey;
    document.getElementById('leaked-msg1').textContent = message1;
    document.getElementById('leaked-sig1').textContent = R.substring(0, 32) + '...' + s1.substring(0, 16);
    document.getElementById('leaked-msg2').textContent = message2;
    document.getElementById('leaked-sig2').textContent = R.substring(0, 32) + '...' + s2.substring(0, 16);

    // Show workspace
    document.getElementById('attack-workspace').classList.remove('hidden');

    console.log('Leaked data generated:', {
        pubKey: satoshiPublicKey.substring(0, 16) + '...',
        R: R.substring(0, 16) + '...',
        s1: s1.substring(0, 16) + '...',
        s2: s2.substring(0, 16) + '...',
        actualKey: satoshiPrivateKey.substring(0, 16) + '...'
    });
}

function calculatePrivateKey() {
    if (!gameData.leakedData) {
        alert('Generate leaked data first!');
        return;
    }

    const { s1, s2, e1, e2, actualPrivateKey } = gameData.leakedData;

    // Simulate the recovery (we'll use actual key for demo)
    gameData.recoveredKey = actualPrivateKey;

    // Display result
    setTimeout(() => {
        document.getElementById('recovered-key-value').textContent = gameData.recoveredKey;
        document.getElementById('recovered-key').classList.remove('hidden');
        document.getElementById('forge-section').classList.remove('hidden');

        console.log('Private key recovered!');
    }, 1500);
}

function forgeSignature() {
    if (!gameData.recoveredKey) {
        alert('Recover the private key first!');
        return;
    }

    const forgeMessage = document.getElementById('forge-message').value || 'I, Satoshi, hereby transfer 1 million BTC';

    // Create forged signature using recovered key
    const nonce = randomHex(64);
    const R = pointMultiply(nonce, 'G');
    const pubKey = pointMultiply(gameData.recoveredKey, 'G');
    const e = simpleHash(R + pubKey + forgeMessage);
    const s = modAdd(nonce, simpleHash(e + gameData.recoveredKey));

    gameData.forgedSignature = {
        message: forgeMessage,
        R: R,
        s: s
    };

    // Display forged signature
    document.getElementById('forged-sig-value').textContent = R.substring(0, 32) + '...' + s.substring(0, 16);
    document.getElementById('forged-result').classList.remove('hidden');

    console.log('Signature forged successfully');
}

// ==================== Challenge 2: MuSig2 Protocol ====================

// MuSig2 game data
gameData.musig2 = {
    alice: null,
    bob: null,
    L: null,
    coefficients: {},
    aggregatedPubkey: null,
    nonces: {},
    commitments: {},
    revealedNonces: {},
    aggregatedNonce: null,
    challenge: null,
    partialSigs: {},
    finalSignature: null,
    message: null
};

// Phase 1: Rogue Key Attack Demo
function musigDemoAttack() {
    // Generate honest keys
    const alicePriv = randomHex(64);
    const alicePub = pointMultiply(alicePriv, 'G');

    const bobHonestPriv = randomHex(64);
    const bobHonestPub = pointMultiply(bobHonestPriv, 'G');

    // Bob creates a rogue key: P_bob_rogue = P_bob_honest - P_alice
    const bobRoguePub = pointSubtract(bobHonestPub, alicePub);

    // Naive aggregation
    const aggregatedKey = pointAdd(alicePub, bobRoguePub);

    // Store for display
    gameData.attackDemo = {
        alicePub,
        bobHonestPub,
        bobRoguePub,
        aggregatedKey
    };

    // Display results
    document.getElementById('attack-alice-pub').textContent = alicePub;
    document.getElementById('attack-bob-honest').textContent = bobHonestPub;
    document.getElementById('attack-bob-rogue').textContent = bobRoguePub;
    document.getElementById('attack-agg-key').textContent = aggregatedKey;
    document.getElementById('attack-demo-result').classList.remove('hidden');

    console.log('Rogue key attack demonstrated');
    console.log('Aggregated key equals Bob honest pub:', aggregatedKey === bobHonestPub);
}

function proceedToMusig2() {
    document.getElementById('musig2-section').classList.remove('hidden');
    document.getElementById('musig2-section').scrollIntoView({ behavior: 'smooth', block: 'start' });
}

// Phase 2: MuSig2 Key Generation and Aggregation
function musigGenerateKeys(party) {
    if (party === 'alice') {
        gameData.musig2.alice = {
            private: randomHex(64),
            public: null
        };
        gameData.musig2.alice.public = pointMultiply(gameData.musig2.alice.private, 'G');

        document.getElementById('musig-alice-priv').textContent = gameData.musig2.alice.private.substring(0, 32) + '...';
        document.getElementById('musig-alice-pub').textContent = gameData.musig2.alice.public;
        document.getElementById('musig-alice-keys').classList.remove('hidden');

        console.log('Alice MuSig2 keys generated');
    } else if (party === 'bob') {
        gameData.musig2.bob = {
            private: randomHex(64),
            public: null
        };
        gameData.musig2.bob.public = pointMultiply(gameData.musig2.bob.private, 'G');

        document.getElementById('musig-bob-priv').textContent = gameData.musig2.bob.private.substring(0, 32) + '...';
        document.getElementById('musig-bob-pub').textContent = gameData.musig2.bob.public;
        document.getElementById('musig-bob-keys').classList.remove('hidden');

        console.log('Bob MuSig2 keys generated');
    }

    // Show aggregation section when both keys exist
    if (gameData.musig2.alice && gameData.musig2.bob) {
        document.getElementById('musig2-aggregate-section').classList.remove('hidden');
    }
}

function musig2AggregateKeys() {
    if (!gameData.musig2.alice || !gameData.musig2.bob) {
        alert('Generate both keys first!');
        return;
    }

    const alicePub = gameData.musig2.alice.public;
    const bobPub = gameData.musig2.bob.public;

    // Step 1: Compute L = H(P1 || P2 || ... || Pn)
    // Sort pubkeys lexicographically for deterministic ordering
    const pubkeys = [alicePub, bobPub].sort();
    gameData.musig2.L = simpleHash(pubkeys.join(''));

    // Step 2: Compute key aggregation coefficients ai = H(L || Pi)
    gameData.musig2.coefficients.alice = simpleHash(gameData.musig2.L + alicePub);
    gameData.musig2.coefficients.bob = simpleHash(gameData.musig2.L + bobPub);

    // Step 3: Aggregate with coefficients: P_agg = a1*P1 + a2*P2
    const aliceWeighted = pointMultiply(gameData.musig2.coefficients.alice, alicePub);
    const bobWeighted = pointMultiply(gameData.musig2.coefficients.bob, bobPub);
    gameData.musig2.aggregatedPubkey = pointAdd(aliceWeighted, bobWeighted);

    // Display results
    setTimeout(() => {
        document.getElementById('musig2-L').textContent = gameData.musig2.L.substring(0, 32) + '...';
        document.getElementById('musig2-a-alice').textContent = gameData.musig2.coefficients.alice.substring(0, 32) + '...';
        document.getElementById('musig2-a-bob').textContent = gameData.musig2.coefficients.bob.substring(0, 32) + '...';
        document.getElementById('musig2-agg-pub').textContent = gameData.musig2.aggregatedPubkey;
        document.getElementById('musig2-agg-result').classList.remove('hidden');
        document.getElementById('musig2-nonce-section').classList.remove('hidden');

        console.log('MuSig2 key aggregation complete');
    }, 1500);
}

// Phase 3: Nonce Commitment and Revelation
function musig2GenerateNonces() {
    if (!gameData.musig2.aggregatedPubkey) {
        alert('Aggregate keys first!');
        return;
    }

    // Each party generates two nonces (for enhanced security in MuSig2)
    gameData.musig2.nonces.alice = {
        r1: randomHex(64),
        r2: randomHex(64)
    };
    gameData.musig2.nonces.bob = {
        r1: randomHex(64),
        r2: randomHex(64)
    };

    // Compute nonce points R1 and R2
    gameData.musig2.nonces.alice.R1 = pointMultiply(gameData.musig2.nonces.alice.r1, 'G');
    gameData.musig2.nonces.alice.R2 = pointMultiply(gameData.musig2.nonces.alice.r2, 'G');
    gameData.musig2.nonces.bob.R1 = pointMultiply(gameData.musig2.nonces.bob.r1, 'G');
    gameData.musig2.nonces.bob.R2 = pointMultiply(gameData.musig2.nonces.bob.r2, 'G');

    // Create commitments (hash of nonce points)
    gameData.musig2.commitments.alice = simpleHash(
        gameData.musig2.nonces.alice.R1 + gameData.musig2.nonces.alice.R2
    );
    gameData.musig2.commitments.bob = simpleHash(
        gameData.musig2.nonces.bob.R1 + gameData.musig2.nonces.bob.R2
    );

    // Display commitments
    setTimeout(() => {
        document.getElementById('musig2-alice-commit').textContent = gameData.musig2.commitments.alice.substring(0, 48) + '...';
        document.getElementById('musig2-bob-commit').textContent = gameData.musig2.commitments.bob.substring(0, 48) + '...';
        document.getElementById('musig2-nonce-result').classList.remove('hidden');

        console.log('Nonce commitments generated');
    }, 1000);
}

function musig2RevealNonces() {
    if (!gameData.musig2.commitments.alice || !gameData.musig2.commitments.bob) {
        alert('Generate commitments first!');
        return;
    }

    // Reveal the nonce points
    gameData.musig2.revealedNonces = {
        alice: {
            R1: gameData.musig2.nonces.alice.R1,
            R2: gameData.musig2.nonces.alice.R2
        },
        bob: {
            R1: gameData.musig2.nonces.bob.R1,
            R2: gameData.musig2.nonces.bob.R2
        }
    };

    // Aggregate nonces: R_agg = R1_alice + R2_alice + R1_bob + R2_bob
    let Ragg = gameData.musig2.nonces.alice.R1;
    Ragg = pointAdd(Ragg, gameData.musig2.nonces.alice.R2);
    Ragg = pointAdd(Ragg, gameData.musig2.nonces.bob.R1);
    Ragg = pointAdd(Ragg, gameData.musig2.nonces.bob.R2);
    gameData.musig2.aggregatedNonce = Ragg;

    // Display revealed nonces
    setTimeout(() => {
        document.getElementById('musig2-alice-R1').textContent = gameData.musig2.revealedNonces.alice.R1.substring(0, 32) + '...';
        document.getElementById('musig2-alice-R2').textContent = gameData.musig2.revealedNonces.alice.R2.substring(0, 32) + '...';
        document.getElementById('musig2-bob-R1').textContent = gameData.musig2.revealedNonces.bob.R1.substring(0, 32) + '...';
        document.getElementById('musig2-bob-R2').textContent = gameData.musig2.revealedNonces.bob.R2.substring(0, 32) + '...';
        document.getElementById('musig2-R-agg').textContent = gameData.musig2.aggregatedNonce;
        document.getElementById('musig2-reveal-result').classList.remove('hidden');
        document.getElementById('musig2-partial-section').classList.remove('hidden');

        console.log('Nonces revealed and aggregated');
    }, 1000);
}

// Phase 4: Partial Signature Generation
function musig2CreatePartialSigs() {
    if (!gameData.musig2.aggregatedNonce) {
        alert('Reveal nonces first!');
        return;
    }

    gameData.musig2.message = document.getElementById('musig2-message').value || 'Transfer 1 BTC to Alice';

    // Compute challenge: e = H(R_agg || P_agg || m)
    gameData.musig2.challenge = simpleHash(
        gameData.musig2.aggregatedNonce +
        gameData.musig2.aggregatedPubkey +
        gameData.musig2.message
    );

    // Alice partial signature: s_alice = (r1 + r2) + e * a_alice * d_alice
    const aliceNonceSum = modAdd(gameData.musig2.nonces.alice.r1, gameData.musig2.nonces.alice.r2);
    const aliceWeighted = modMultiply(
        gameData.musig2.challenge,
        modMultiply(gameData.musig2.coefficients.alice, gameData.musig2.alice.private)
    );
    gameData.musig2.partialSigs.alice = modAdd(aliceNonceSum, aliceWeighted);

    // Bob partial signature: s_bob = (r1 + r2) + e * a_bob * d_bob
    const bobNonceSum = modAdd(gameData.musig2.nonces.bob.r1, gameData.musig2.nonces.bob.r2);
    const bobWeighted = modMultiply(
        gameData.musig2.challenge,
        modMultiply(gameData.musig2.coefficients.bob, gameData.musig2.bob.private)
    );
    gameData.musig2.partialSigs.bob = modAdd(bobNonceSum, bobWeighted);

    // Display partial signatures
    setTimeout(() => {
        document.getElementById('musig2-challenge').textContent = gameData.musig2.challenge.substring(0, 48) + '...';
        document.getElementById('musig2-s-alice').textContent = gameData.musig2.partialSigs.alice.substring(0, 48) + '...';
        document.getElementById('musig2-s-bob').textContent = gameData.musig2.partialSigs.bob.substring(0, 48) + '...';
        document.getElementById('musig2-partial-result').classList.remove('hidden');
        document.getElementById('musig2-final-section').classList.remove('hidden');

        console.log('Partial signatures created');
    }, 1000);
}

// Phase 5: Signature Aggregation
function musig2AggregateSignature() {
    if (!gameData.musig2.partialSigs.alice || !gameData.musig2.partialSigs.bob) {
        alert('Create partial signatures first!');
        return;
    }

    // Aggregate partial signatures: s = s_alice + s_bob (mod n)
    const s = modAdd(gameData.musig2.partialSigs.alice, gameData.musig2.partialSigs.bob);

    // Final signature is (R, s)
    gameData.musig2.finalSignature = {
        R: gameData.musig2.aggregatedNonce,
        s: s
    };

    const sigString = gameData.musig2.finalSignature.R.substring(0, 32) + '...' + gameData.musig2.finalSignature.s.substring(0, 32);

    // Display final signature
    setTimeout(() => {
        document.getElementById('musig2-final-sig').textContent = sigString;
        document.getElementById('musig2-final-result').classList.remove('hidden');

        console.log('MuSig2 signature aggregation complete');
        console.log('Final signature:', gameData.musig2.finalSignature);
    }, 1500);
}

// ==================== Challenge 3: Taproot Builder ====================

let taprootProgress = 0;

function taprootStep1() {
    // Generate internal key
    gameData.internalKey = randomHex(64);

    document.getElementById('taproot-internal').textContent = 'Internal Key: ' + gameData.internalKey;
    document.getElementById('taproot-internal').classList.remove('hidden');
    document.getElementById('taproot-step2').classList.remove('hidden');

    taprootProgress = 1;
    updateTaprootProgress();

    console.log('Internal key generated:', gameData.internalKey.substring(0, 16) + '...');
}

function taprootStep2() {
    if (taprootProgress < 1) {
        alert('Complete step 1 first!');
        return;
    }

    // Get selected scripts
    const checks = document.querySelectorAll('.script-check:checked');
    if (checks.length === 0) {
        alert('Select at least one script condition!');
        return;
    }

    gameData.scriptTree = [];

    checks.forEach(check => {
        const scriptType = check.getAttribute('data-script');
        let scriptData;

        switch(scriptType) {
            case 'timelock':
                scriptData = {
                    name: 'Timelock (144 blocks)',
                    script: 'OP_CHECKSEQUENCEVERIFY 144',
                    hash: simpleHash('OP_CHECKSEQUENCEVERIFY 144')
                };
                break;
            case 'multisig':
                scriptData = {
                    name: '2-of-3 Multisig',
                    script: 'OP_CHECKMULTISIG 2 3',
                    hash: simpleHash('OP_CHECKMULTISIG 2 3')
                };
                break;
            case 'hashlock':
                scriptData = {
                    name: 'Hash Preimage',
                    script: 'OP_HASH160 <hash> OP_EQUAL',
                    hash: simpleHash('OP_HASH160 OP_EQUAL')
                };
                break;
            case 'oracle':
                scriptData = {
                    name: 'Oracle Signature',
                    script: 'OP_CHECKSIG <oracle_pubkey>',
                    hash: simpleHash('OP_CHECKSIG oracle')
                };
                break;
        }

        if (scriptData) {
            gameData.scriptTree.push(scriptData);
        }
    });

    // Display selected scripts
    const scriptsDiv = document.getElementById('scripts-selected');
    scriptsDiv.innerHTML = `
        <div style="color: #4ade80; margin-bottom: 10px;">✓ ${gameData.scriptTree.length} scripts selected:</div>
        ${gameData.scriptTree.map((script, i) => `
            <div style="color: #9ca3af; margin: 5px 0;">• ${script.name}</div>
        `).join('')}
    `;
    scriptsDiv.classList.remove('hidden');

    document.getElementById('taproot-step3').classList.remove('hidden');

    taprootProgress = 2;
    updateTaprootProgress();

    console.log('Scripts selected:', gameData.scriptTree.length);
}

function taprootStep3() {
    if (taprootProgress < 2) {
        alert('Complete step 2 first!');
        return;
    }

    // Build Merkle tree
    const leaves = gameData.scriptTree.map(s => s.hash);

    if (leaves.length === 1) {
        gameData.merkleRoot = leaves[0];
    } else if (leaves.length === 2) {
        gameData.merkleRoot = simpleHash(leaves[0] + leaves[1]);
    } else if (leaves.length >= 3) {
        const left = simpleHash(leaves[0] + leaves[1]);
        gameData.merkleRoot = simpleHash(left + leaves.slice(2).join(''));
    }

    // Visualize tree
    const treeDiv = document.getElementById('merkle-visual');
    treeDiv.innerHTML = `
        <div style="text-align: center; font-family: monospace; font-size: 0.85em; margin-top: 15px;">
            <div style="color: #f7931a; margin: 15px 0; font-weight: bold;">Merkle Root: ${gameData.merkleRoot.substring(0, 16)}...</div>
            ${leaves.length > 1 ? `<div style="color: #9ca3af;">└─┬─┘</div>` : ''}
            <div style="display: flex; gap: 10px; justify-content: center; flex-wrap: wrap; margin-top: 15px;">
                ${gameData.scriptTree.map((script, i) => `
                    <div style="background: rgba(0,0,0,0.3); padding: 10px; border-radius: 5px; border: 1px solid #3a3a3a;">
                        <div style="color: #4ade80; font-size: 0.9em; margin-bottom: 5px;">${script.name}</div>
                        <div style="color: #6b7280; font-size: 0.75em;">${script.hash.substring(0, 12)}...</div>
                    </div>
                `).join('')}
            </div>
        </div>
    `;
    treeDiv.classList.remove('hidden');

    document.getElementById('taproot-step4').classList.remove('hidden');

    taprootProgress = 3;
    updateTaprootProgress();

    console.log('Merkle root:', gameData.merkleRoot.substring(0, 16) + '...');
}

function taprootStep4() {
    if (taprootProgress < 3) {
        alert('Complete step 3 first!');
        return;
    }

    // Tweak the key
    const tweak = simpleHash(gameData.internalKey + gameData.merkleRoot);
    const tweakPoint = pointMultiply(tweak, 'G');
    gameData.tweakedKey = pointAdd(gameData.internalKey, tweakPoint);

    document.getElementById('tweaked-key').textContent = 'Tweaked Output Key: ' + gameData.tweakedKey;
    document.getElementById('tweaked-key').classList.remove('hidden');

    document.getElementById('taproot-step5').classList.remove('hidden');

    taprootProgress = 4;
    updateTaprootProgress();

    console.log('Key tweaked:', gameData.tweakedKey.substring(0, 16) + '...');
}

function taprootStep5() {
    if (taprootProgress < 4) {
        alert('Complete step 4 first!');
        return;
    }

    // Generate Taproot address
    const addressData = simpleHash(gameData.tweakedKey);
    gameData.taprootAddress = 'bc1p' + addressData.substring(0, 52);

    document.getElementById('taproot-address-value').textContent = gameData.taprootAddress;
    document.getElementById('final-address').classList.remove('hidden');

    taprootProgress = 5;
    updateTaprootProgress();

    console.log('Taproot address created:', gameData.taprootAddress);
}

function updateTaprootProgress() {
    const progressBar = document.getElementById('taproot-progress');
    const stepSpan = document.getElementById('taproot-step');
    const percentSpan = document.getElementById('taproot-percent');

    if (progressBar) {
        const percentage = (taprootProgress / 5) * 100;
        progressBar.style.width = percentage + '%';
    }

    if (stepSpan) {
        stepSpan.textContent = taprootProgress;
    }

    if (percentSpan) {
        percentSpan.textContent = Math.round((taprootProgress / 5) * 100);
    }
}

// ==================== Challenge 4: MAST Detective ====================

function generateMysteryTx() {
    // Create a mystery transaction with hidden script
    const scripts = [
        { type: 'timelock', name: 'Timelock Script (OP_CHECKSEQUENCEVERIFY)' },
        { type: 'multisig', name: '2-of-3 Multisig Script' },
        { type: 'hashlock', name: 'Hash Lock Script (OP_HASH160)' },
        { type: 'oracle', name: 'Oracle Signature Script' }
    ];

    const selectedScript = scripts[Math.floor(Math.random() * scripts.length)];

    const witnessData = {
        signature: randomHex(64),
        scriptData: randomHex(128),
        controlBlock: randomHex(64)
    };

    gameData.mysteryTx = {
        type: selectedScript.type,
        name: selectedScript.name,
        witness: witnessData
    };

    // Display witness data
    document.getElementById('witness-0').textContent = witnessData.signature.substring(0, 48) + '...';
    document.getElementById('witness-1').textContent = witnessData.scriptData.substring(0, 48) + '...';
    document.getElementById('witness-2').textContent = witnessData.controlBlock.substring(0, 48) + '...';

    document.getElementById('detective-workspace').classList.remove('hidden');

    console.log('Mystery transaction generated:', selectedScript.name);
}

function checkScript(scriptType) {
    if (!gameData.mysteryTx) {
        alert('Generate mystery transaction first!');
        return;
    }

    const resultDiv = document.getElementById('detective-result');

    if (scriptType === gameData.mysteryTx.type) {
        resultDiv.className = 'result-box success';
        resultDiv.innerHTML = `
            <div class="result-header">
                <span class="result-icon">✓</span>
                CORRECT IDENTIFICATION!
            </div>
            <div class="result-content">
                <p>You correctly identified the spending script: <strong>${gameData.mysteryTx.name}</strong></p>
                <p style="margin-top: 15px; color: #7a9a7a;">
                    This demonstrates the power of MAST - only the used script was revealed,
                    while all other scripts in the Merkle tree remain completely hidden!
                </p>
            </div>
        `;
        resultDiv.classList.remove('hidden');

        document.getElementById('merkle-explanation').classList.remove('hidden');

        console.log('Correct script identified!');
    } else {
        resultDiv.className = 'result-box';
        resultDiv.style.background = 'rgba(239,68,68,0.2)';
        resultDiv.style.border = '2px solid #ef4444';
        resultDiv.innerHTML = `
            <div class="result-header">
                <span class="result-icon">✗</span>
                INCORRECT
            </div>
            <div class="result-content">
                <p style="color: #f87171;">That's not the script revealed in this transaction. Analyze the witness data carefully and try again!</p>
            </div>
        `;
        resultDiv.classList.remove('hidden');
    }
}

function downloadCertificate() {
    const summary = `
═══════════════════════════════════════════════════════════
    TAPROOT EXPLORER - COMPLETION CERTIFICATE
═══════════════════════════════════════════════════════════

This certifies that the holder has successfully completed
the Taproot Explorer challenge series and demonstrated
understanding of:

✓ BIP 340: Schnorr Signatures
  - Nonce reuse vulnerabilities and exploitation
  - Key aggregation with MuSig protocol
  - Signature linearity properties

✓ BIP 341: Taproot Addresses
  - Internal key and script tree construction
  - Merkle tree building and root calculation
  - Key tweaking: Q = P + H(P||m)·G
  - Taproot address generation (bc1p...)

✓ MAST (Merkelized Alternative Script Trees)
  - Selective script disclosure
  - Merkle proof verification
  - Privacy through hiding unused conditions

✓ Security Analysis
  - Understanding cryptographic vulnerabilities
  - Privacy benefits of Schnorr and Taproot
  - Best practices for signature generation

═══════════════════════════════════════════════════════════
FINAL SCORE: ${score} / 500 points
DATE: ${new Date().toLocaleDateString('en-US', { year: 'numeric', month: 'long', day: 'numeric' })}
═══════════════════════════════════════════════════════════

Challenges Completed:
1. Nonce Reuse Attack - Exploited vulnerability to recover private key
2. Key Aggregation - Created 2-of-2 Schnorr multisig
3. Taproot Builder - Built complete Taproot address with scripts
4. MAST Detective - Analyzed transaction witness data
5. Security Quiz - Demonstrated comprehensive understanding

"Don't trust, verify." - Bitcoin Core

Generated by Taproot Explorer v1.0
https://github.com/cypherpunk-legacy
`;

    const blob = new Blob([summary], { type: 'text/plain;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `taproot-explorer-certificate-${Date.now()}.txt`;
    a.click();
    URL.revokeObjectURL(url);

    console.log('Certificate downloaded');
}

// ==================== Challenge 5: Security Quiz ====================

function submitQuiz() {
    const correctAnswers = {
        q1: 'b',  // Linear aggregation
        q2: 'c',  // Private key can be calculated
        q3: 'b',  // All outputs look identical
        q4: 'a',  // Only used script + proof
        q5: 'b'   // Q = P + H(P || merkle_root) · G
    };

    const explanations = {
        q1: {
            correct: "✓ Correct! Schnorr's linearity enables signature aggregation through MuSig, allowing multiple parties to create a single signature.",
            incorrect: "✗ Incorrect. While size and speed are benefits, the key advantage is the linear aggregation property that enables MuSig protocols."
        },
        q2: {
            correct: "✓ Correct! If you reuse a nonce, anyone can calculate your private key using: d = (s₁ - s₂) / (e₁ - e₂). This is catastrophic!",
            incorrect: "✗ Incorrect. Nonce reuse allows attackers to calculate the private key through simple algebra. This is the worst-case scenario."
        },
        q3: {
            correct: "✓ Correct! Taproot makes all outputs (single-sig, multisig, complex scripts) look identical on-chain, massively improving privacy.",
            incorrect: "✗ Incorrect. Taproot's main privacy benefit is that all output types (single-sig, multisig, scripts) are indistinguishable on the blockchain."
        },
        q4: {
            correct: "✓ Correct! MAST (Merkelized Alternative Script Trees) only reveals the script you're using plus the Merkle proof. Other scripts stay hidden forever.",
            incorrect: "✗ Incorrect. Only the executed script and its Merkle proof are revealed. All other scripts in the tree remain private - that's the power of MAST!"
        },
        q5: {
            correct: "✓ Correct! The tweak commits to the script tree: tweak = H(P || merkle_root), then Q = P + tweak·G. This is the core of Taproot!",
            incorrect: "✗ Incorrect. The correct formula is Q = P + H(P || merkle_root) · G, where the hash commits to both the internal key and script tree."
        }
    };

    let correctCount = 0;
    let allAnswered = true;

    // Check each question
    for (let i = 1; i <= 5; i++) {
        const questionName = `q${i}`;
        const selectedAnswer = document.querySelector(`input[name="${questionName}"]:checked`);
        const resultDiv = document.getElementById(`result-${questionName}`);

        if (!selectedAnswer) {
            allAnswered = false;
            if (resultDiv) {
                resultDiv.className = 'quiz-result incorrect';
                resultDiv.textContent = '⚠️ Please select an answer';
            }
            continue;
        }

        const isCorrect = selectedAnswer.value === correctAnswers[questionName];

        if (isCorrect) {
            correctCount++;
            resultDiv.className = 'quiz-result correct';
            resultDiv.textContent = explanations[questionName].correct;
        } else {
            resultDiv.className = 'quiz-result incorrect';
            resultDiv.textContent = explanations[questionName].incorrect;
        }
    }

    if (!allAnswered) {
        alert('Please answer all questions before submitting!');
        return;
    }

    // Show final score after brief delay
    setTimeout(() => {
        if (correctCount === 5) {
            const finalPoints = 50;
            completeChallenge(4, finalPoints);
            showFinalScore(correctCount);
        } else {
            alert(`You got ${correctCount}/5 correct. Try again! You need all 5 correct to complete the challenge.`);
        }
    }, 1000);
}

function showFinalScore(quizScore) {
    const finalScoreDiv = document.getElementById('final-score');
    const totalScoreSpan = document.getElementById('total-score');

    if (totalScoreSpan) {
        totalScoreSpan.textContent = score;
    }

    if (finalScoreDiv) {
        finalScoreDiv.classList.remove('hidden');
        window.scrollTo({ top: finalScoreDiv.offsetTop - 100, behavior: 'smooth' });
    }

    console.log(`All challenges completed! Final score: ${score}`);
}

// ==================== Initialize ====================

document.addEventListener('DOMContentLoaded', () => {
    console.log('🌳 Taproot Explorer initialized');
    console.log('Interactive challenges for BIP 340 & BIP 341');

    // Initialize first stage
    switchStage(0);

    // Disable locked stages
    const stageButtons = document.querySelectorAll('.stage-btn');
    stageButtons.forEach((btn, index) => {
        if (index > 0) {
            btn.disabled = true;
            btn.classList.add('locked');
        }
    });
});
