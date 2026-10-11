# The cryptography, module by module

Everything in `crates/crypto` is plain Rust with no engine or UI dependency
and no randomness except where a function takes a `Drbg` or asks the operating
system for entropy. Primitives (hashes, curve arithmetic, Schnorr, AES, GCM,
ML-KEM, ML-DSA) come from the audited RustCrypto crates; the protocol logic
around them is original, written to be read as reference material.

Each module below names the puzzle(s) that use it. `crates/crypto/tests/vectors.rs`
pins the protocol code to published test vectors (shipped in
`crates/crypto/tests/vectors`), and every module has unit tests of its own.

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

BIP-340 sign and verify (through `k256`), plus two teaching extras:
`sign_with_nonce` (sign with a chosen `k`, which is how Warden's broken signer
behaves) and `recover_from_nonce_reuse`, which computes
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
bar. The man-in-the-middle is plain group arithmetic in the puzzle. Tests
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

---

### A note on honesty

Where the game says "this verifies", the value on screen is the output of the
verification equation, not a scripted success. Where it says an attack
"works", the recovered key or plaintext is recomputed and checked against the
real one before the puzzle unlocks. The toy sizes (GF(31), 10- and 31-bit DH
groups, `n = 4` LWE) are chosen so the mathematics is *visible*, and the text
says each time how real-world parameters differ.
