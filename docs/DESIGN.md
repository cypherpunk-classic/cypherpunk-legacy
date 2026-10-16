# Design notes

## Why Bevy, and what changed from v2

v2 (TypeScript, Phaser 3, a DOM overlay) proved the game: real cryptography,
one through-line story, tests that play it to the end. v3 keeps all of that
and moves it to Rust and Bevy 0.20 so that one codebase gives a native desktop
game and a WebAssembly build, with the cryptography on audited RustCrypto
crates and the whole game checkable by `cargo test`.

The rule from v2 stands: **every "break" the player performs is computed live
from real cryptography and could be verified by anyone.** The story is the
delivery mechanism for that, not a substitute.

## Architecture

The split that matters is between what knows about the engine and what does
not:

- **`crates/crypto` knows nothing about the game.** `BigInt`s and byte arrays
  in and out, tested against published vectors.
- **`crates/art` and `crates/audio` generate content.** Pixel art is painted
  into RGBA buffers by code; sound effects and the five music loops are
  synthesised sample by sample and wrapped as WAV. No binary assets anywhere.
- **`crates/logic` is the whole game except the world renderer.** Save state,
  maps (as data), the story, the codex, and every puzzle with its egui
  interface. It depends on `egui` but not on Bevy, and it can run headless.
- **The root crate is the Bevy app.** It turns map data into entities,
  moves people around with collisions, runs the camera, lights and particles,
  plays sound, stores saves, and executes what the story asks of the world.

### The story runtime

Story scripts are ordinary `async fn`s taking a `Ctx`. They change game state
directly (`c.set("ch1.done")`, `c.give("share1")`) and ask for everything else
by sending a command and awaiting its reply:

```rust
say!(c, (Mara, "Here. My radio log. Have a look.")).await;
if !c.puzzle(Box::new(TwoTimePad::new(c.world()))).await { return; }
c.goto(MapId::City, Some("gateway")).await;
```

A tiny executor polls the scripts with a no-op waker every frame. Commands that
only concern the interface (dialogue, choices, puzzle terminals, waits, sound)
are handled inside `Game`; world commands (change map, pan the camera, flash,
burst, set an object's frame, save the position) become `EngineReq`s for the
Bevy side. A headless director answers those instantly, which is how the tests
play the whole game without a window.

### Rendering

- One baked image per map holds the tiles (two when there is water, swapped
  every 650 ms). Props, people and the cat are sprites, y-sorted by giving each
  a z derived from its feet.
- The night is a colour multiplied into every world sprite; lights are
  additive sprites on top: `ColorMaterial` extended with a pipeline
  specialisation that sets the blend state to `src·α + dst`. Flicker and pulse
  switch between a handful of pre-made strengths.
- The camera uses an integer zoom (`floor(min(w/300, h/190))`, clamped to
  2..6), made whole in device pixels, and snaps to the device-pixel grid with a
  half-pixel correction on odd window sizes, so every texel is a crisp block.
- egui draws all text on top: crisp at any zoom, and the same code serves
  dialogue, terminals, menus and the HUD. Fades, flashes and CRT scanlines are
  painted on egui's background layer, under the windows.

## Determinism and the per-save world

`World` (in `crates/logic/src/world.rs`) derives the entire cryptographic state
of a playthrough from the save's single 32-byte `seed`: Grandpa's mnemonic, the
Vault key and its five shares (and the forged one), Warden's two nonce-reusing
signatures, the Taproot tree, the checkpoint key. Values are computed on first
use and cached. Consequences:

- No two players get the same puzzles, but every player's puzzles are solvable
  by the same honest method.
- Tests and the autopilot are reproducible: `CPL_SEED` or `?seed=` fixes it all.

## Narrative wiring

`crates/logic/src/story` maps interaction ids (`"mara.talk"`, `"plaza.vault"`)
to scripts. The world only reports *that* the player pressed E in front of
something. Chapter gating is flags: `ch1.done` hides the barrier on the west
avenue through the object's `show_if`, and the world re-evaluates those
conditions every frame.

Each of the six friends teaches one idea and hands over one piece (a Shamir
share, a key, access), so the mechanics and the plot advance together: you learn
MuSig2 *because* the Directorate offered you a rigged joint-custody contract.

## Testing strategy

Three layers, all real:

- **Unit** (`cargo test -p cpl-crypto`): every cryptographic module against
  published vectors (BIP-39/340/341/327/86) and structural properties (LLL
  output is reduced and covolume-preserving, any 3-of-5 shares reconstruct, a
  forged share fails Feldman, the cheater wins exactly when its bet matches).
- **Headless playthroughs** (`cargo test -p cpl-logic`): new game to credits
  through the real scripts, dialogue and egui terminals, every puzzle solved by
  its `autoplay` (which works the puzzle's own controls), once in autoplay mode
  and once by simulated key presses. A separate test draws every puzzle's UI
  before and after solving it.
- **Autopilot** (`CPL_AUTOPLAY=1 CPL_EXIT_AT_END=1`): the Bevy app plays itself.
  A* over an 8-pixel grid takes the player to each person or prop on the route,
  through the real collision code, gates and map changes, and screenshots are
  written at every milestone. Paths are cut down to their corners and every
  frame's step is scaled to land exactly on the next one, so the walk is the
  same at a dozen frames a second as at a hundred. CI runs it under Xvfb with
  software Vulkan; there a step that stalls for five minutes fails the run with
  a report and a screenshot instead of hanging until the job's timeout.

## Accessibility and platforms

- Keyboard and touch (an on-screen stick and action button appear after the
  first touch).
- Settings for text speed, music and effects volume, UI scale, CRT scanlines,
  and reduced motion (no camera shake, fewer particles).
- Saves are JSON: in the platform data folder natively, in `localStorage` on the
  web, and the menu can copy a save to the clipboard or import one, so progress
  is never trapped.

## Extending it

A new puzzle is one file in `crates/logic/src/puzzles` implementing `Puzzle`
(including `autoplay`), registered in `puzzles::make`, plus a few lines of
story. If it needs per-save data, derive it in `World`. A new district is a
region in the city map with a gated barrier and an NPC. Keep the cryptography
in `crates/crypto` with a test, and keep words in egui.
