# Design notes

## Goals of the rewrite

The original prototype had the right *story* — a grandfather's cabin, a seed
phrase, a portal to a crypto city — but three problems made it worth starting
over rather than patching:

1. **The cryptography was fake.** Signatures were `SHA256(key‖msg)`; "elliptic
   curve points" were string hashes; `modInverse` returned a hash; the knapsack
   attack was a `setTimeout` that printed a hard-coded answer.
2. **Seven disconnected pages.** Each level was its own HTML file with its own
   state, style and save, no shared engine, no through-line.
3. **No build, tests or types.** Dependencies loaded from CDNs at runtime.

v2 fixes all three. The guiding rule: **every "break" the player performs is
computed live in the browser from real cryptography, and could be verified by
anyone.** The story is the delivery mechanism for that, not a substitute.

## Architecture

The clean split is the thing to preserve:

- **`src/crypto/` knows nothing about the game.** Pure functions, `bigint` and
  `Uint8Array` in and out. This is what the unit tests import. If you can't test
  a cryptographic claim in Node without a browser, it's in the wrong place.
- **The canvas only ever shows pixels; every word is DOM.** Phaser renders the
  world (`src/world/WorldScene.ts` drives all maps). Text — dialogue, HUD,
  puzzle terminals, menus — lives in an overlay (`src/ui/`) that sits above the
  canvas. This keeps text crisp at every integer zoom, selectable, accessible,
  and reachable by Playwright with ordinary selectors.
- **Content is generated, not stored.** All art is drawn in code
  (`src/art/`) and rasterised to textures at boot (`src/world/textures.ts`);
  all audio is synthesised (`src/core/audio.ts`). The repo has no binary assets,
  which is why the whole game fits in one 1.7 MB HTML file.
- **Puzzles are self-contained terminals.** Each implements the `Puzzle`
  interface (`src/ui/terminal.ts`): a `mount(host)` that builds its DOM and an
  `autoplay()` that drives its own controls to the solution. Story code just
  calls `runPuzzle()` and reacts to success.

## Determinism and the per-save world

`src/story/world.ts` derives the entire cryptographic state of a playthrough —
Grandpa's mnemonic, the Vault key and its five shares, Warden's two
nonce-reusing signatures, the Taproot tree, the checkpoint key — from the save's
single 32-byte `seed`, memoised per session. Two consequences:

- No two players get the same puzzles, but every player's puzzles are solvable
  by the same honest method.
- Tests and the autoplay path are reproducible: `?seed=...` fixes everything.

## Narrative wiring

`src/story/` maps world events to scripts. `WorldScene` reports *that* an
interaction or zone happened; `story/index.ts` routes it to a handler. Handlers
are plain async functions using a tiny API (`say`, `choose`, `puzzle`, `give`,
`codex`, `objective`, `set`/`flag`). Chapter gating is just flags
(`ch1.done` opens the next district's barrier via a `showIf` on the map object).

Each of the six friends teaches one idea and hands over one piece (a Shamir
share, a key, access), so the mechanics and the plot advance together: you learn
MuSig2 *because* the Directorate offered you a rigged joint-custody contract.

## Testing strategy

Two layers, both real:

- **Unit (`tests/unit`, Vitest).** Every cryptographic module against published
  vectors (BIP-340/341/327/86, Trezor BIP-39, FIPS) and structural properties
  (LLL output is reduced and covolume-preserving; any 3-of-5 shares reconstruct;
  a forged share fails Feldman; the cheater wins exactly when its bet matches).
- **End-to-end (`tests/e2e`, Playwright).** One test starts a new game and plays
  to the credits, solving every puzzle through its `autoplay()` — i.e. through
  the same DOM controls a player uses — and asserts zero page errors. A puzzle
  that isn't actually solvable fails the build. The `?debug` bridge only
  triggers interactions and advances dialogue; it never sets a flag to skip a
  puzzle.

## Accessibility & platforms

- Keyboard and touch (an on-screen stick + button appear on coarse pointers).
- Respects `prefers-reduced-motion`; a settings toggle also forces it.
- Responsive down to phone width; the DOM overlay reflows, the camera zoom
  adapts to the viewport.
- Storage is best-effort: the save falls back to in-memory if `localStorage`
  throws (private windows, blocked site data), and the menu offers copy/paste
  export so progress is never trapped.

## Extending it

A new puzzle is one file implementing `Puzzle` plus a few lines of story wiring
and (if it needs per-save data) an entry in `world.ts`. A new district is a
region in `city.ts` with a gated barrier and an NPC. Keep the cryptography in
`src/crypto/` with a test, and keep the text in the DOM.
