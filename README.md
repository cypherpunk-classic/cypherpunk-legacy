# 🔐 Cypherpunk Legacy

> A pixel-art adventure through real cryptography and the crypto wars.

Your grandfather left you a cabin, a cat, and twelve words. Behind them waits
Cryptopolis — a city he helped build in the 1990s so people could talk without
being overheard — and an argument about privacy that never ended. Every key,
signature and attack in this game is **real cryptography, computed in your
browser**: no server, no tracking, no accounts.

This is a from-scratch rewrite (v2) of the original prototype. The old
single-file experiments are preserved under [`legacy/`](legacy/).

---

## Play

```bash
npm install
npm run dev          # http://localhost:5173
```

Or build a self-contained file you can open directly or host anywhere:

```bash
npm run build:single   # dist-single/index.html — one file, works from file://
npm run build          # dist/ — static site for GitHub Pages / any static host
```

**Controls:** WASD / arrow keys to move · Shift to walk faster · E / Space /
Enter to interact · C for the codex · Esc for the menu. Touch controls appear
on phones.

---

## What you actually do

Seven acts, each built on a working implementation you drive yourself:

| Act | Place | Cryptography |
|----|-------|--------------|
| **Prologue** | The Cabin | BIP-39 seed recovery (checksum + master fingerprint), BIP-340 Schnorr challenge–response login |
| **1 · One time only** | The Wharf | One-time pad reuse (crib dragging), AES-CTR malleability vs AES-GCM authentication |
| **2 · Strangers agree** | The Exchange | Diffie–Hellman, man-in-the-middle on an unauthenticated exchange, baby-step giant-step on a weak group |
| **3 · Threshold** | The Commons | Shamir secret sharing over GF(31) and secp256k1, Feldman verifiable secret sharing (catch a forged share) |
| **4 · Signatures** | The Mint | Nonce-reuse key recovery, rogue-key attack vs BIP-327 MuSig2, BIP-341 Taproot script trees |
| **5 · Knowledge** | The Masquerade | Schnorr Σ-protocol, the real-or-simulated game, the rewinding extractor, Fiat–Shamir (and its weak form) |
| **6 · After the quantum** | The Observatory | Lagrange–Gauss & LLL lattice reduction, breaking a Merkle–Hellman knapsack, LWE, and a real ML-KEM-768 encapsulation |
| **Epilogue** | Legacy Plaza | Sign and broadcast the manifesto; the Archive goes public for good |

A **Codex** of 40+ entries unlocks as you play, covering both the mathematics
and the history: the Crypto Wars, PGP, Clipper, Bernstein v. US, Venona,
Dual_EC_DRBG, the NIST PQC standards, and more.

---

## The cryptography is real

The whole point of the rewrite: nothing is faked. Each puzzle calls the same
code the unit tests verify against official vectors.

- **Primitives** come from audited libraries — [`@noble/curves`](https://github.com/paulmillr/noble-curves),
  `@noble/hashes`, `@noble/ciphers`, `@scure/bip39`, `@scure/bip32`,
  `@noble/post-quantum` (Paul Miller).
- **Protocol code** (`src/crypto/`) is original, pure TypeScript with no DOM
  or engine dependency, so it reads as reference material and tests in Node:
  BIP-340 signing & nonce-reuse recovery, BIP-327 KeyAgg, BIP-341 tweaking &
  addresses, Shamir + Feldman over the curve order, the Schnorr Σ-protocol
  (simulator, extractor, Fiat–Shamir), **integral LLL** with exact bigint
  arithmetic, the CJLOSS knapsack attack, and a toy LWE.
- **Verified** against the official BIP-340, BIP-341, BIP-327, BIP-86, Trezor
  BIP-39 and FIPS test vectors (`tests/unit`, `tests/vectors`).

Your save carries a random 32-byte seed; every key, pad, share and address in
your playthrough is derived from it, so no two players see the same puzzles —
and the solutions are always the honest cryptographic ones.

---

## How it's built

- **Phaser 3** for the world (tiles, movement, lighting, cameras), **TypeScript**
  in strict mode, **Vite** for dev/build.
- **All art is drawn in code** (`src/art/`) — characters, tilesets, props,
  buildings, a 3×5 font — so the repo ships with zero image assets. Textures
  are rasterised to canvases at boot.
- **All text lives in the DOM overlay** (`src/ui/`), crisp at any zoom and
  fully testable; the canvas only ever shows pixels.
- **All sound is synthesised** with the Web Audio API (`src/core/audio.ts`) —
  no audio files.
- Every puzzle implements an `autoplay()` that drives its own controls to the
  solution; the end-to-end test plays the entire game through it.

```
src/
  crypto/     pure, tested cryptography (no DOM/engine)
  art/        code-defined pixel art → canvas textures
  core/       state, save, input, audio, events
  world/      Phaser WorldScene + the two maps
  ui/         DOM overlay: dialogue, HUD, terminals, menus, widgets
  puzzles/    one folder per act; each puzzle is a self-contained terminal
  story/      narrative wiring, the per-save crypto world, the Codex
tests/
  unit/       Vitest, against official vectors
  e2e/        Playwright: a full new-game-to-credits playthrough
```

See [`docs/DESIGN.md`](docs/DESIGN.md) and [`docs/CRYPTO.md`](docs/CRYPTO.md).

---

## Develop

```bash
npm run typecheck   # tsc, strict, app + tests
npm test            # Vitest unit tests (37, against official vectors)
npm run e2e         # Playwright: play the whole game through the UI
npm run build       # typecheck + static build
```

The e2e test is the real acceptance check: it starts a new game and completes
every chapter by working each puzzle's own controls, asserting zero page errors
from title to credits.

---

## License

MIT — see [LICENSE](LICENSE). Characters and organisations in Cryptopolis are
fiction. The history in the Codex is not.

> *"Privacy is necessary for an open society in the electronic age."*
> — Eric Hughes, *A Cypherpunk's Manifesto*, 1993

> *"Cypherpunks write code."*
