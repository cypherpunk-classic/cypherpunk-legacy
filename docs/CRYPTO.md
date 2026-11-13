# The cryptography, module by module

Everything in `crates/crypto` is plain Rust with no engine or UI dependency
and no randomness except where a function takes a `Drbg` or asks the operating
system for entropy. Primitives (hashes, curve arithmetic, Schnorr, AES, GCM,
ML-KEM, ML-DSA) come from the audited RustCrypto crates; the protocol logic
around them is original, written to be read as reference material.

Each module below names the puzzle(s) that use it. `crates/crypto/tests/vectors.rs`
pins the protocol code to published test vectors (shipped in
`crates/crypto/tests/vectors`), and every module has unit tests of its own.

Since v3.1 the terminals rarely call an "attack" function on the player's
behalf. The player assembles the decisive formula from tokens (`s = k + e·d`,
`d = (s₁ − s₂)/(e₁ − e₂)`, `R = s·G − c·X`, `K = B^a`, `C = C_ − r·K`) and the
puzzle evaluates exactly what was built with these modules' arithmetic, then
checks it with the real verification equation. A wrong arrangement produces a
real wrong number and a specific reason. The deep dives redo each idea in the
toy group `p = 1019, q = 509, g = 4` from `dh::TOY_GROUP`, by hand.

## `bytes` · foundations

Hex and bit conversions, `BigInt ↔ bytes`, and modular arithmetic on big
integers: `modulo`, `floor_div`, `round_div` (exact nearest-integer division,
which LLL needs), extended Euclid, `mod_inv`, `mod_pow`, Miller–Rabin
`is_probable_prime`, and `log2_big` for basis profiles. Small `u64` helpers
(`mul_mod_u64`, `pow_mod_u64`) keep the toy Diffie–Hellman groups fast.

## `drbg` · deterministic content

`Drbg::new(seed, label)` is a SHA-256 stream that generates *puzzle content*
(keys, pads, shares) from the save's 32-byte seed, with a label per use for
domain separation, and rejection-samples uniform integers below a bound. It is
not a key generator for anything real: the epilogue's signature and the
ML-KEM demonstration use OS entropy (`getrandom`, which is
`crypto.getRandomValues` in the browser).

## `secp` · secp256k1

A readable layer over `k256`: `G`, the group order `N`, the field prime,
scalar multiplication, x-only and compressed encodings, BIP-340 `lift_x`, and
`tagged_hash`. Everything the signature and zero-knowledge chapters stand on.

## `bip39`, `bip32` · seeds *(Prologue)*

- `checksum_report` exposes the 132 bits of a 12-word mnemonic, 128 of entropy
  and 4 of SHA-256 checksum, so the puzzle can show exactly which bits a word
  sets.
- `checksum_candidates` and `recovery_funnel`: the roughly 1/16 of the wordlist
  that makes a missing word's checksum valid, then narrowed by a visible first
  letter.
- `seed_from_mnemonic` (PBKDF2-HMAC-SHA512, 2048 rounds), BIP-32 extended keys
  and `master_fingerprint` (HASH160 of the master public key), BIP-86
  derivation `m/86'/0'/0'/0/0`, and the `tr(...)` descriptor on Grandpa's
  sticky note.
- `generate_grandpa_seed` builds a puzzle whose funnel needs the fingerprint to
  resolve to a single word.

Verified against the Trezor BIP-39 vectors (including the extended private
keys) and the BIP-86 test vector.

## `bip340` · Schnorr signatures *(Prologue, Chapter 4)*

BIP-340 sign and verify (through `k256`), plus teaching extras:
`sign_with_nonce` (sign with a chosen `k`, which is how Warden's broken signer
behaves, and how the login terminal lets the player build `s = k + e·d` by
hand after normalising `R` and `d` to even y), `challenge` (the tagged hash
`e`), and `recover_from_nonce_reuse`, which computes
`d = (s₁ − s₂)/(e₁ − e₂) mod n` and shows each step. `explain_verify` returns
both sides of `s·G = R + e·P` for display. Passes the full BIP-340 CSV vectors.

## `musig` · key aggregation *(Chapter 4)*

`naive_aggregate` (the broken `P₁ + P₂`), `rogue_key` (`T − ΣPᵢ`), and BIP-327
`key_agg` with the real coefficients `aᵢ = H(L ‖ Pᵢ)` and the second-key
optimisation. `rogue_attempt_against_key_agg` shows the rogue trick failing
under KeyAgg. Matches the official BIP-327 KeyAgg vectors.

## `taproot`, `bech32` · Taproot *(Chapter 4)*

TapLeaf and TapBranch hashing, `tweak_key` (`Q = P + H_TapTweak(P ‖ root)·G`),
bech32m addresses, script builders (single signature, 2-of-2, time-locked), and
`pair_addresses` to enumerate the leaf pairs a player might try. Matches every
BIP-341 `scriptPubKey` test vector, addresses included.

## `otp`, `aead` · pads and authenticated encryption *(Chapter 1)*

`make_two_time_pad` and `crib_drag` (the `C₁ ⊕ C₂ = P₁ ⊕ P₂` attack, with a
plausibility score for each guess); AES-256-CTR and AES-256-GCM wrappers,
`flip_bit` and `forge_ctr` (`C' = C ⊕ (P ⊕ P')`) to show CTR's malleability and
GCM rejecting the same edit. `kdf` is HKDF-SHA256.

## `dh` · Diffie–Hellman *(Chapter 2)*

Safe-prime groups `p = 2q + 1` with a generator of order `q`, at three sizes (a
10-bit toy, a 31-bit "export grade" group, and a 64-bit one), `shared` and
`session_key`, and baby-step giant-step as a resumable state machine
(`Bsgs::step(budget)`) so the attack can run a slice per frame with a progress
bar; the Logjam terminal counts its baby and giant steps against a memory
limit and a re-key deadline, so the player has to choose a table size near
√q. The man-in-the-middle is plain group arithmetic in the puzzle. Tests
confirm the groups are genuine safe primes and that BSGS recovers a 31-bit
exponent.

## `shamir` · secret sharing *(Chapter 3)*

`deal` and `reconstruct` over any prime field (GF(31) for the visible puzzle,
the secp256k1 order for the real key), `lagrange_at_zero` exposing each
coefficient as a fraction and reduced mod *p*, Feldman's `feldman_check`
(`y·G = Σ xʲ·Cⱼ`), and `quadratic_through` / `curve_for_guess` for the "two
shares reveal nothing" plot. Tests confirm any three of five shares rebuild the
secret, a forged share is caught, and two shares are consistent with *every*
secret.

## `sigma` · zero knowledge *(Chapter 5)*

The Schnorr identification Σ-protocol: `commit`, `respond`,
`verify_transcript`, `simulate` (the honest-verifier simulator,
`R = s·G − c·X`), `extract` (special soundness, which is nonce reuse again), a
one-bit `cheater_commit`, and Fiat–Shamir (`fs_challenge`, `fs_prove`,
`fs_verify`) with an option to leave the context out of the hash so the replay
bug can be demonstrated. Tests cover completeness, simulation, extraction, the
cheater's exact win condition, and the weak Fiat–Shamir replay.

## `lattice` · reduction *(Chapter 6)*

**Integral LLL** (Cohen, Algorithm 2.6.7, after de Weger): instead of rational
Gram–Schmidt coefficients it keeps the integer determinants `dᵢ` and `λᵢⱼ`, so
every division is exact and there is no floating point anywhere in the
reduction. `is_lll_reduced` checks both conditions exactly;
`gram_determinants` and `profile` give the Gram–Schmidt profile the puzzle
plots. `Gauss2` is two-dimensional Lagrange–Gauss reduction, one move at a time,
for the hand-played version. Tests confirm the output is LLL-reduced and spans
the same lattice (same covolume), and that Gauss reduction finds the true
shortest vector (checked by brute force).

## `knapsack` · Merkle–Hellman *(Chapter 6)*

Superincreasing key generation, encryption and decryption, `density`, the
CJLOSS lattice (`cj_basis`) and `attack_block` (LLL, then read the `±1` short
vector off the reduced basis, retrying with a reordered basis on an unlucky
block). `make_knapsack_puzzle` guarantees an instance that LLL breaks block by
block. Tested end to end: private-key decryption and the lattice attack agree on
the plaintext.

## `lwe`, `pq` · post-quantum *(Chapter 6)*

A toy LWE (`n = 4, q = 97`) with Regev-style bit encryption, the phase the
puzzle plots, and `gaussian_solve` (recovers `s` without noise, garbage with
it). `pq` wraps RustCrypto's ML-KEM-768 (FIPS 203) and ML-DSA-65 (FIPS 204).
Tests confirm LWE decrypts within its noise budget, Gaussian elimination only
works without noise, ML-KEM rejects a tampered ciphertext implicitly, and
ML-DSA round-trips.

## `ecash` · Chaumian eCash with blind Diffie–Hellman *(Chapter 3, the wallet)*

The scheme Cashu and Fedimint use today, after David Wagner's 1996 variant of
Chaum's blind signatures, on secp256k1. `hash_to_curve` maps a secret to a
point `Y` (domain `Secp256k1_HashToCurve_Cashu_`, counter-based try-and-
increment). The wallet blinds, `B_ = Y + r·G`; the `Mint` signs, `C_ = k·B_`,
with one key per denomination (`AMOUNTS` 1 to 32) and a Chaum–Pedersen
`Dleq` proof that it used its published key `K = k·G` rather than a key made
for one customer; the wallet unblinds, `C = C_ − r·K = k·Y`. Redeeming checks
`C = k·hash_to_curve(secret)` and a spent list. `split_amount`, `select`,
`prepare` and `finish` make change. Tests: blind–sign–unblind verifies, the
mint's own ledger cannot link a withdrawal to a deposit, a per-customer
("tagging") key is caught by the DLEQ proof, and coin selection covers any
amount. The game keeps real tokens in the save; every purchase is a redemption
at the mint plus a blind re-issue of the change.

## `onion` · remailers and mixing *(Chapter 5)*

Each `Remailer` has a secp256k1 key. `seal_layer` / `peel_layer` are
ECIES-style: an ephemeral key, ECDH, HKDF, then AES-256-GCM over the next hop's
name and the inner packet (`LAYER_OVERHEAD` = 61 bytes). `wrap` builds the
onion from the inside out; `route_packet` peels it hop by hop and returns what
each remailer saw (one name in, one name out). `pad` / `unpad` give every
packet the same length. `simulate` runs a traffic analyst against two
policies: `Immediate` (forward after a delay, so first in is first out) and
`Batch` (collect n, shuffle, flush); `fifo_attack` scores how many senders the
analyst links. Tests: three hops deliver and each hop learns one step, a wrong
key or a flipped bit fails, padding round-trips, and batching defeats FIFO
linking while immediate forwarding does not.

## `hashcash` · postage *(Chapter 5, the finale)*

Adam Back's Hashcash version 1 headers (`1:bits:date:resource::rand:counter`),
counted in leading zero bits of SHA-256. `Minter` is resumable
(`step(budget)`) and allocation-free in its inner loop, so the stamps terminal
measures the machine's real hash rate and prices a spam flood from it;
`verify` checks the bits, the resource and a replay list, and
`expected_work` is 2^bits. Tests: zero-bit counting, and mint → verify →
replay rejected.

## `clipper` · the Clipper chip's LEAF *(Chapter 4)*

A model of the 1993 Escrowed Encryption Standard: each `Chip` has a unit ID
and a unit key held in escrow; every call carries a Law Enforcement Access
Field: the session key encrypted under the unit key, the unit ID, and a 16-bit
checksum, all encrypted under a family key (AES-256 and HMAC-SHA256 stand in
for the classified Skipjack). `chip_accepts` checks only the checksum, as the
real chip did; `wiretap` opens a LEAF with the escrowed keys. `LeafSearch`
is Matt Blaze's 1994 attack: try random LEAFs until one passes the 16-bit
check (about 65,536 tries), so the phone works while the escrow agents
recover garbage. Tests: an honest LEAF is accepted and opens, a forged one
passes the chip but not the wiretap, a zero LEAF is rejected.

## `wot` · the web of trust *(the key-signing party, the remailers, allies)*

OpenPGP-style without the packet format: `fingerprint(xonly, created)` is a
20-byte v4-style hash, `short_id` and `long_id` are its last 4 and 16 hex
digits, and `collide_short_id` searches for a key whose short ID matches a
target, as the Evil32 project did in 2014. `certify` signs
`(issuer fingerprint, subject fingerprint, user id)` with BIP-340;
`verify_cert` checks it. `validity` runs GnuPG's classic rule over a set of
nodes and certifications: a key is valid if you signed it, or if it is signed
by one fully trusted or three marginally trusted valid keys, along paths up
to five long, with owner trust as an input. Tests: fingerprints and IDs,
colliding short IDs with distinct fingerprints, certifications verify, and the
GnuPG rules (full, marginal, depth, never-trusted).

## `ocr`, `dots`, `hashchain` · the paper trail *(the Book and two side quests)*

`ocr`: CRC-16/CCITT-FALSE per printed line (checked against the standard
check value `0x29B1`), the scanner's `CONFUSABLE` character classes, and
`repairs`, which finds the readings of a line that match its printed checksum,
the way the 1997 PGP source books were scanned back in. `dots`: printer
tracking dots on a 15 × 8 grid (minute, hour, day, month, year, serial
digits, odd column parity), `encode` and `decode`. `hashchain`: a ledger where
each line's hash covers the previous hash, `first_break` for a clumsy edit,
and the observation that a rewrite which recomputes every hash is only caught
by hashes published elsewhere (Haber and Stornetta, 1991). Tests for each.

---

### A note on honesty

Where the game says "this verifies", the value on screen is the output of the
verification equation, not a scripted success. Where it says an attack
"works", the recovered key or plaintext is recomputed and checked against the
real one before the puzzle unlocks. The toy sizes (GF(31), 10- and 31-bit DH
groups, `n = 4` LWE) are chosen so the mathematics is *visible*, and the text
says each time how real-world parameters differ. Since v3.1 the same holds for
the systems around the puzzles: the wallet holds real blind-signed tokens, an
ally is a real BIP-340 certification in the save, and a Hashcash stamp is a
real proof of work the finale spends.
