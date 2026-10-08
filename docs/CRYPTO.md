# The cryptography, module by module

Everything in `src/crypto/` is pure TypeScript: no DOM, no Phaser, no randomness
except where a function explicitly takes a PRNG or asks for OS entropy. That
keeps it testable in Node and readable as reference material. Primitives
(hashes, curve arithmetic, AEAD ciphers, BIP-39/32, ML-KEM) come from the
audited `@noble/*` and `@scure/*` libraries; the protocol logic around them is
original.

Each module below names the puzzle(s) that use it and the test file that pins it
to published vectors.

## `bytes.ts` — foundations

Hex/UTF-8/bit conversions, `bigint ↔ bytes`, and modular arithmetic on `bigint`:
`mod`, `floorDiv`, `roundDiv` (exact nearest-integer division, which LLL needs),
`egcd`, `modInv`, `modPow`, `gcd`, Miller–Rabin `isProbablePrime`, and
`log2Big` for basis profiles. Used everywhere.

## `drbg.ts` — deterministic content

`Drbg(seed, label)` is a SHA-256-based stream used to generate *puzzle content*
(keys, pads, shares) deterministically from the save's 32-byte seed, with a
per-puzzle label for domain separation. It is **not** a key generator — the only
real secret the game creates (your epilogue signature key) uses
`crypto.getRandomValues`. Rejection-samples uniform `bigint`s below a bound.

## `secp.ts` — secp256k1

A readable layer over `@noble/curves`: `G`, the group order `N`, field prime,
`mul`/`add`/`sub`, `xOnly`/`compressed` encodings, BIP-340 `liftX`, and
`taggedHash`. Everything the signature and ZK chapters stand on.

## `bip39.ts` — seeds *(Prologue · `bip39.test.ts`)*

- `checksumReport` exposes the 132 bits of a 12-word mnemonic: 128 entropy + the
  4-bit SHA-256 checksum, so the puzzle can show exactly which bits a word sets.
- `checksumCandidates` / `recoveryFunnel`: the ~1/16 of the wordlist that makes a
  hole's checksum valid, then narrowed by a visible first letter.
- `masterFingerprint` (HASH160 of the BIP-32 master pubkey), `deriveKey`
  (BIP-86 path `m/86'/0'/0'/0/0`), `descriptor`.
- `generateGrandpaSeed` builds a puzzle whose funnel needs the fingerprint to
  resolve to a unique word.

Verified against the Trezor BIP-39 vectors and the BIP-86 test vector
(`73c5da0a`, `bc1p5cyxnux…`).

## `bip340.ts` — Schnorr signatures *(Prologue, Ch. 4 · `bip340.test.ts`)*

BIP-340 sign/verify (through noble) plus two teaching extras:
`signWithNonce` (sign with a chosen `k`, for the broken-device museum) and
`recoverFromNonceReuse` → `d = (s₁−s₂)/(e₁−e₂) mod n`. `explainVerify` returns
both sides of `s·G = R + e·P` for display. Passes the full BIP-340 CSV vectors.

## `musig.ts` — key aggregation *(Ch. 4 · `musig.test.ts`)*

`naiveAggregate` (the broken `P₁+P₂`), `rogueKey` (`T − ΣPᵢ`), and BIP-327
`keyAgg` with the real coefficient `aᵢ = H(L‖Pᵢ)` and the second-key
optimisation. `rogueAttemptAgainstKeyAgg` shows the rogue trick failing to
converge under KeyAgg. Matches the official BIP-327 KeyAgg vectors.

## `taproot.ts` — Taproot *(Ch. 4 · `taproot.test.ts`)*

TapLeaf/TapBranch hashing, `tweakKey` (`Q = P + H_TapTweak(P‖root)·G`),
bech32m `taprootAddress`, script builders (single-sig, 2-of-2, timelocked), and
`pairAddresses` to enumerate leaf pairs. Matches every BIP-341 `scriptPubKey`
test vector, including addresses.

## `otp.ts` / `aead.ts` — pads and AEAD *(Ch. 1 · `symmetric.test.ts`)*

`makeTwoTimePad` and `cribDrag` (the `C₁⊕C₂ = P₁⊕P₂` attack with a plausibility
score); `ctr*`/`gcm*` wrappers, `flipBit`, and `forgeCtr` (`C' = C ⊕ (P⊕P')`) to
show CTR malleability and GCM rejecting the same edit. `kdf` is HKDF-SHA256.

## `dh.ts` — Diffie–Hellman *(Ch. 2 · `dh.test.ts`)*

Safe-prime groups `p = 2q+1`, generator `g = 4` of order `q`, at three sizes
(10-bit toy, 31-bit "export grade", 64-bit). `shared`, `sessionKey`, and
baby-step giant-step (`bsgs`, plus an `async` variant that yields to the event
loop for a progress bar). The MITM is pure group arithmetic in the puzzle. Tests
confirm the groups are genuine safe primes and BSGS recovers a 31-bit exponent.

## `shamir.ts` — secret sharing *(Ch. 3 · `shamir.test.ts`)*

`deal`/`reconstruct` over any prime field (GF(31) for the visual puzzle, the
secp256k1 order for the real key), `lagrangeAtZero` exposing the fraction and its
reduction mod *p*, Feldman `feldmanCheck` (`y·G = Σ xʲ·Cⱼ`), and `quadraticThrough`
/ `curveForGuess` for the "two shares reveal nothing" animation. Tests confirm
any 3-of-5 rebuild the secret, a forged share is caught, and two shares are
consistent with *every* secret.

## `sigma.ts` — zero knowledge *(Ch. 5 · `sigma.test.ts`)*

The Schnorr identification Σ-protocol: `commit`/`respond`/`verifyTranscript`,
`simulate` (the HVZK simulator, `R = s·G − c·X`), `extract` (special soundness,
= nonce reuse), one-bit `cheaterCommit`, and Fiat–Shamir (`fsChallenge`,
`fsProve`, `fsVerify`) with an option to omit the context so the replay bug is
demonstrable. Tests cover completeness, simulation, extraction, the cheater's
exact win condition, and the weak-FS replay.

## `lattice.ts` — reduction *(Ch. 6 · `lattice.test.ts`)*

**Integral LLL** (Cohen Alg. 2.6.7 / de Weger): instead of rational Gram–Schmidt
coefficients it keeps the integer determinants `dᵢ` and `λ_{i,j}`, so every
division is exact — no floating point anywhere in the reduction. `isLllReduced`
checks the two conditions exactly; `gramDeterminants`/`profile` give the GSO
profile the puzzle animates. Plus 2-D Lagrange–Gauss (`gaussReduce`) for the
hand-played version. Tests confirm the output is LLL-reduced and spans the same
lattice (same covolume), and that Gauss reduction finds the true shortest vector
(checked by brute force).

## `knapsack.ts` — Merkle–Hellman *(Ch. 6 · `lattice.test.ts`)*

Superincreasing keygen, encrypt/decrypt, `density`, the CJLOSS lattice
(`cjBasis`), and `attackBlock` (LLL + read off the `±1` short vector), with a
retry that reorders the basis on an unlucky block. `makeKnapsackPuzzle`
guarantees an instance every block of which LLL breaks. Tested end-to-end:
private-key decryption and lattice recovery agree on the plaintext.

## `lwe.ts` / `pq.ts` — post-quantum *(Ch. 6 · `lwe_pq.test.ts`)*

A toy LWE (`n=4, q=97`) with Regev bit encryption, the phase computation the
puzzle plots, and `gaussianSolve` (recovers `s` with no noise, garbage with
noise). `pq.ts` wraps `@noble/post-quantum` ML-KEM-768 (FIPS 203) and ML-DSA-65
(FIPS 204). Tests confirm LWE decrypts under the noise budget, Gaussian
elimination only works noise-free, and ML-KEM/ML-DSA round-trip.

---

### A note on honesty

Where the game says "this verifies", the on-screen value is the actual output of
the verification equation, not a scripted success. Where it says an attack
"works", the recovered key or plaintext is recomputed and checked against the
real one before the puzzle unlocks. The toy field sizes (GF(31), 31/64-bit DH
groups, `n=4` LWE) are chosen so the mathematics is *visible*, and the text is
explicit each time about how the real-world parameter sizes differ.
