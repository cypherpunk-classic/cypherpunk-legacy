# Cypherpunk Legacy

> A pixel-art adventure through real cryptography and the crypto wars.

Your grandfather left you a cabin, a cat, and twelve words. Behind them waits
Cryptopolis, a city he helped build in the 1990s so people could talk without
being overheard, and an argument about privacy that never ended. Every key,
signature and attack in this game is **real cryptography, computed on your
machine**: no server, no tracking, no accounts.

This is v3, a rewrite of the whole game in **Rust** on the **[Bevy](https://bevy.org)
0.20** engine, running natively on desktop and in the browser (WebAssembly +
WebGL2). The TypeScript/Phaser v2 lives on the `v2` branch; the original
single-file prototype is preserved under [`legacy/`](legacy/).

**v3.1, "Cypherpunks write code"**, puts the decisive step of every puzzle in
your hands (the terminal checks, it never solves), adds six missions from
cypherpunk history, and turns the movement's ideals into systems you live
with between puzzles: a surveillance Dragnet, a web of trust, a wallet of
blind-signed eCash, side quests, and choices that lead to three endings. See
[`docs/GAMEPLAY.md`](docs/GAMEPLAY.md).

**v4: the Wharf and the Exchange.** Playtesting 3.1 showed that honest
cryptography had not made a fun game: too much walking between terminals,
too much text, puzzles that read like exercises. v4 rebuilds the city one
district at a time around puzzles that stand in the street, each district
with one physical rule you learn by playing. On **the Wharf** every gate, patrol and
camera is a lamp board speaking in one-time-pad pictures, and you carry
punched cards that flip the lamps behind their holes. Up the road, at **the
Exchange**, every gate, guard, camera and the tower take their orders over
telephone lines, and you work the junction boxes: a cord joins two jacks and
whatever one end says, the other end hears. You answer a guard as the
Directorate's desk, stand in the middle of a scrambled Diffie-Hellman line
with a relay, turn its knob until a short authentication word agrees, meet a
sealed handshake you cannot turn, record the tower's export-grade key
exchange and take it apart with baby-step giant-step, and make the last call
over a Clipper phone with a forged LEAF. No quest givers, a map with one-key
travel, guards and cameras that keep watching while you work, and three
cyberspace dives. Why, and where it goes next: [`docs/V4.md`](docs/V4.md).

**v4.2: one game.** v4.0 and v4.1 shipped the two districts as a separate
game behind their own title button. v4.2 merges them into the story: one NEW
GAME, one save. Chapter 1 still opens with Mara's radio log and the boathouse
gate, then she sends you down to the harbour proper, the Wharf district, and
the chapter ends when its mast answers to you. Chapter 2 still opens with
Felix's terminals and the press, then the Exchange district is its climax, and
Felix hands over his share when its lines are free. Chapters 3 to 6, the
Dragnet, the wallet, the web of trust, the side quests and the endings are
all as they were. The districts' puzzles unlock six new codex entries (known
plaintext, MACs, replay, blue boxes, short authentication strings, Logjam),
and the end credits count your cards and dives.

**v4.3: the campaign map.** You no longer walk from one quarter of the city
to the next: M (or the ★ button) opens a map of Cryptopolis with every stage
on it, joined by roads the story opens one at a time, and picking a stage
takes you there. Each district scores up to three stars (freed; never
flagged on the way; no paid notebook pages), each dive one. A finished
district can be replayed from the map on fresh pads, in a sandbox that leaves
the story alone and brings back only better stars. Stars buy kit in a
four-line upgrade tree (opsec, the Wharf kit, the lineman's set, Grandpa's
notebook) that makes the street kinder and never the cryptography easier.

**v4.4: every chapter is a stage.** The quarters of the v3 city score stars
too, from the terminals you solved there: every terminal solved, steady
hands (a wrong move per terminal at most), and step hints off. A finished
quarter can be replayed as a **gauntlet**, its terminals back to back on fresh
numbers. Thirty stars in all, and two new lines in the tree: **Chaff** (Q:
a few seconds of noise the sensors cannot tell from you, after Rivest's
*chaffing and winnowing*, 1998) and the **Terminal kit**.

**v4.5: souls in the street.** Once the mast is yours, **the Lamplighter**
comes back for it: a boss you beat by cryptanalysis. Its floodlight leaves
one lane dark each beat, chosen by a shift register; dodge to live, but only
a call made *before* the light shows hurts it, and a call is a prediction.
Phase I is one LFSR (fit its taps to ten bits), phase II two XORed (run
Berlekamp–Massey on at least 2L bits), phase III reboots with phase II's
seed (line the new beats up against your log). Trace out and the phase starts
over, but what you worked out stays: knowledge is the one thing you keep.
Around it, the souls loop: **lanterns** at every safe house and district start
are bonfires (rest to clear your trace, recharge chaff, bank your cards, set
where you wake up, and spend stars, which you can do nowhere else); a fall in
the Wharf **drops** the rubbings and cuts you made since your last rest where
they caught you, gone for good if you fall again first; and **F** is a dodge
the sensors cannot fix on.

---

## Play

Native (macOS, Linux, Windows), with a recent stable Rust:

```bash
cargo run --release
```

In the browser (builds a static site into `dist/`):

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129   # must match Cargo.lock
./scripts/build-web.sh
python3 -m http.server -d dist 8080                # then open http://localhost:8080
```

**Controls:** WASD / arrow keys to move · Shift to walk faster · E / Space /
Enter to interact · C for the codex · Esc for the menu · M (or the ★ button)
for the campaign map · Q to throw chaff, once you have it · F to dodge while
moving. On a touch screen, drag on the left half to walk and tap
the E button to interact.

**In the two districts:** click a card to put it in a board's reader,
right-click (or drag) it onto the light table, M for the district map, `?` on
a board for Grandpa's notes; at a junction box, click a jack to take up a
cord and another jack to plug it in, and click a plugged jack to pull its
cord. To skip ahead for development, `CPL_WHARF=1` (`?wharf`) starts a game at
chapter 1's harbour and `CPL_EXCHANGE=1` (`?exchange`) at chapter 2's street;
add `CPL_AUTOPLAY=1 CPL_PACE=0.6` (`&autoplay=demo`) to watch it play on from
there.

**Watch it play itself:** `CPL_AUTOPLAY=1 cargo run --release` natively, or
`?autoplay=demo` on the web build. The autopilot walks the city, talks to
everyone and solves every puzzle with the honest method. Add `CPL_SIDE=1` (or
use `?autoplay=full`) and it takes every side quest on the way. It plays a
fresh game of its own and never overwrites your save.

---

## What you actually do

Seven acts, each built on a working implementation you drive yourself:

| Act | Place | Cryptography |
|----|-------|--------------|
| **Prologue** | The Cabin | BIP-39 seed recovery (checksum + master fingerprint), a BIP-340 Schnorr login whose response you build yourself |
| **1 · One time only** | The Wharf, then the harbour district | One-time pad reuse (crib dragging), AES-CTR malleability vs AES-GCM authentication; then the harbour's lamp boards: malleability without the key, known plaintext, the two-time pad, HMAC seals and replay, and two dives (Shamir as a shooting gallery, crib dragging live) |
| **2 · Strangers agree** | The Exchange, then its street | Diffie–Hellman, man-in-the-middle on an unauthenticated exchange, baby-step giant-step on an export-grade group, and Grandpa's code smuggled out as a printed book (OCR repair against CRC-16); then the street's lines: in-band trust, a relay in a scrambled line, short authentication strings ground and sealed, LOGJAM, and a Clipper LEAF forged |
| **3 · Threshold** | The Commons | Shamir secret sharing over GF(31) and secp256k1, Feldman verifiable secret sharing, Chaumian eCash with blind Diffie–Hellman at the Commons Mint |
| *Interlude* | The Exchange | A key-signing party: fingerprints, IDs and a colliding short key ID (PGP's web of trust) |
| **4 · Signatures** | The Mint | Nonce-reuse key recovery, rogue-key attack vs BIP-327 MuSig2, BIP-341 Taproot script trees, Matt Blaze's attack on the Clipper chip's LEAF |
| **5 · Knowledge** | The Masquerade | Onion-routed remailers and mixing, Hashcash postage, the Schnorr Σ-protocol, the simulator, the rewinding extractor, Fiat–Shamir done right and wrong |
| **6 · After the quantum** | The Observatory | Lagrange–Gauss & LLL lattice reduction, breaking a Merkle–Hellman knapsack, LWE, and a real ML-KEM-768 encapsulation |
| **Epilogue** | Legacy Plaza | Sign, verify and broadcast the manifesto through the remailers; the Archive goes public for good |

Between the puzzles:

- **The Dragnet.** Masts and sweeping cameras log you; stand in their fields and
  your trace climbs, faster or slower depending on the tools you have earned. At
  100 an agent fines you and walks you to a safe house.
- **The web of trust.** People who check your fingerprint sign your key with a
  real BIP-340 certification; validity follows GnuPG's rules, and your allies
  fill the plaza at the end.
- **A wallet of blind-signed eCash.** Every purchase is a real redemption at the
  Commons Mint, which never learns who paid. Hashcash stamps are the second
  currency.
- **Side quests and floppies.** A reporter's forged drops, a clerk's printer
  dots, a grocer's hash-chained ledger, and six disks from Grandpa's
  mailing-list years.
- **Choices with consequences**, and three endings tallied in the credits.

Every terminal lists its steps (with hints in *Guided* mode), and most offer a
**deep dive**: the same idea worked by hand in a toy group, against a clock.

A **Codex** of 54 entries unlocks as you play, covering both the mathematics
and the history: the Crypto Wars, PGP, Clipper, Bernstein v. US, Venona,
DigiCash, the remailers, Hashcash, Dual_EC_DRBG, the NIST post-quantum
standards, and more.

---

## The cryptography is real

Nothing is faked. Each puzzle calls the same code the tests pin to official
vectors.

- **Primitives** come from the audited [RustCrypto](https://github.com/RustCrypto)
  crates: `k256` (secp256k1, BIP-340 Schnorr), `sha2`, `hmac`, `pbkdf2`,
  `ripemd`, `hkdf`, `aes`, `ctr`, `aes-gcm`, `ml-kem` (FIPS 203) and `ml-dsa`
  (FIPS 204).
- **Protocol code** (`crates/crypto`) is original and engine-free: BIP-39
  checksum funnels, BIP-32/86 derivation, BIP-340 nonce-reuse recovery, BIP-327
  KeyAgg, BIP-341 tweaking and bech32m addresses, Shamir + Feldman over the
  curve order, the Schnorr Σ-protocol (simulator, extractor, Fiat–Shamir),
  **integral LLL** with exact big-integer arithmetic, the CJLOSS knapsack
  attack, a toy LWE, Chaumian eCash with blind DH and DLEQ proofs (the Cashu
  construction), ECIES onion layers with a mixing simulation, Hashcash, a model
  of the Clipper chip's LEAF and Blaze's attack, an OpenPGP-style web of trust
  with GnuPG validity, CRC-16 OCR repair, printer tracking dots and hash
  chains.
- **Verified** against the official BIP-39 (Trezor), BIP-340, BIP-341, BIP-327
  and BIP-86 test vectors (`crates/crypto/tests`).

Your save carries a random 32-byte seed; every key, pad, share and address in
your playthrough is derived from it, so no two players see the same puzzles,
and the solutions are always the honest cryptographic ones.

---

## How it's built

```
crates/
  crypto/   pure, tested cryptography (no engine, no UI)
  art/      code-defined pixel art: tiles, people, props, buildings, signs, lights
  audio/    synthesised sound effects and music loops (written out as WAV)
  logic/    game state, story scripts, maps, codex, puzzles and their egui UI,
            plus a headless driver that plays the whole game in tests
src/        the Bevy app: world, actors, camera, lights, particles, sound,
            saving, autopilot, web/native platform glue
web/        the HTML shell for the browser build
```

- **Bevy 0.20** renders the world: baked tile layers, y-sorted sprites, a
  multiplied night tint, additive light sprites (a two-line material extension),
  particles, an integer-zoom pixel-perfect camera.
- **egui** (via `bevy_egui`) draws every word: dialogue with a typewriter,
  puzzle terminals, HUD, menus, codex, title and credits. Fonts are embedded
  (VT323 and IBM Plex Mono, both OFL).
- **Story scripts are plain `async fn`s** over a small context (`say`,
  `choose`, `puzzle`, `goto`, `pan_to`…), polled each frame by a tiny executor,
  so the same story runs in the game, in the headless tests, natively and on
  the web.
- **All art is drawn in code and all sound is synthesised**: the repository
  ships no image or audio files.
- Every puzzle implements `autoplay`, which works its own controls to the
  solution. The tests play the entire game with it; the autopilot uses it too.

See [`docs/DESIGN.md`](docs/DESIGN.md) and [`docs/CRYPTO.md`](docs/CRYPTO.md).

---

## Develop

```bash
cargo test -p cpl-crypto -p cpl-art -p cpl-audio -p cpl-logic   # unit tests + headless playthroughs
cargo run                                                        # dev build (dependencies optimised)
CPL_AUTOPLAY=1 CPL_SPEED=2 CPL_SHOTS=shots CPL_EXIT_AT_END=1 cargo run --release   # end-to-end, with screenshots
cargo run -p cpl-art --example preview                           # write every sprite sheet as PNG to scratch/art
```

The headless playthroughs (`crates/logic/tests/playthrough.rs`) start a new
game and play to the credits through the real scripts, dialogue and egui
terminals: the main story in autoplay mode and by simulated key presses, a
completionist run through every side quest, two runs with different choices
that must reach the other endings, and a run that gets caught by the Dragnet.
The autopilot run does the same through the real Bevy world: walking,
collisions, sensor fields, interactions, map changes, camera pans and all.

Environment variables for native runs: `CPL_AUTOPLAY`, `CPL_SIDE` (the
autopilot takes the side quests too), `CPL_SPEED` (walking speed multiplier),
`CPL_PACE` (seconds per line while autoplaying), `CPL_SEED` (reproducible new
game), `CPL_SHOTS` (screenshot folder), `CPL_SHOT_EVERY` (also a screenshot
every so many seconds), `CPL_EXIT_AT_END` (quit after the credits with status
0, or with status 1 and a report if any step of the route stalls for five
minutes), `CPL_DATA_DIR` (where saves go), and `CPL_PUZZLE` (open one terminal,
a comma-separated list, or `all`, and screenshot each fresh, solved and in its
deep dive; without `CPL_SHOTS` the terminal simply opens for you to play,
and the next one in the list opens when you close it). On the web:
`?autoplay`, `?autoplay=demo`, `?autoplay=full`, `?side`, `?puzzle=…` (jump
straight into a terminal, e.g. `?puzzle=ch4.nonce`), `?seed=…`, `?speed=…`.

```bash
CPL_PUZZLE=all CPL_SHOTS=shots CPL_EXIT_AT_END=1 cargo run   # every terminal, three screenshots each
```

---

## License

The game is dedicated to the public domain under CC0 1.0 (see [LICENSE](LICENSE)).
The bundled fonts keep their own licences: VT323 and IBM Plex Mono under the SIL
Open Font License, and a subset of DejaVu Sans Mono under the Bitstream Vera
licence (see `crates/logic/assets/fonts`). Characters and organisations in
Cryptopolis are fiction. The history in the Codex is not.

> *"Privacy is necessary for an open society in the electronic age."*
> Eric Hughes, *A Cypherpunk's Manifesto*, 1993

> *"Cypherpunks write code."*
