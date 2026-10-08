/**
 * The Codex: short, accurate reference entries unlocked as you play.
 * Markup: **bold**, *em*, `code`, {term}. Paragraphs separated by blank lines.
 */

export type CodexGroup = 'History' | 'Building blocks' | 'Protocols' | 'Attacks' | 'Post-quantum';

export interface CodexEntry {
  id: string;
  title: string;
  group: CodexGroup;
  era: string;
  body: string;
  ingame?: string;
}

export const CODEX: CodexEntry[] = [
  // ------------------------------------------------------------------ History
  {
    id: 'crypto-wars',
    title: 'The Crypto Wars',
    group: 'History',
    era: '1970s – today',
    body: `The recurring fight over who may use strong cryptography. In the 1970s and 80s encryption was a military technology: the US listed it as a *munition* and restricted its export. As personal computers and networks spread, academics, companies and activists pushed back.

The 1990s saw the sharpest battles: export controls on software like PGP, the Clipper Chip key-escrow proposal, and lawsuits arguing that source code is speech. Export rules were largely relaxed in 2000, but the argument never ended. It returns whenever a government asks for "exceptional access" to encrypted data, under names like *lawful access* or *going dark*.

The technical core of the debate is simple to state and hard to escape: a backdoor that only the good guys can use is a key, and keys get stolen, leaked, abused or compelled.`,
    ingame: 'The Directorate of Lawful Access in Cryptopolis is fictional. The arguments it makes are not.',
  },
  {
    id: 'manifesto',
    title: "A Cypherpunk's Manifesto",
    group: 'History',
    era: 'March 9, 1993',
    body: `A short essay by Eric Hughes that became the movement's statement of purpose. It argues that privacy is the power to selectively reveal oneself to the world, that it cannot be granted by governments or corporations, and so must be defended with technology anyone can use.

Two lines from it travel furthest. "Privacy is necessary for an open society in the electronic age." And: "Cypherpunks write code." The point of the second is that arguments alone do not protect anyone; working, published software does.`,
  },
  {
    id: 'cypherpunks-list',
    title: 'The Cypherpunks mailing list',
    group: 'History',
    era: '1992 – early 2000s',
    body: `A mailing list started in 1992 by Eric Hughes, Timothy C. May and John Gilmore, after a series of meetings in the San Francisco Bay Area. At its height it had hundreds of subscribers and an enormous volume of posts on cryptography, anonymity, digital cash and politics.

Ideas that circulated there include anonymous remailers, digital cash systems and proof-of-work schemes, all of which fed later work on Tor, Bitcoin and modern privacy tools.`,
    ingame: "Grandpa's photo on the mantel is from one of these meetups. His handle was *nightjar*.",
  },
  {
    id: 'pgp',
    title: 'PGP and the munitions export fight',
    group: 'History',
    era: '1991 – 1996',
    body: `Phil Zimmermann released Pretty Good Privacy in June 1991: public-key email encryption for ordinary people. It spread worldwide over the internet, and in 1993 the US government opened a criminal investigation into whether this violated export controls on cryptography.

Supporters responded creatively. The PGP source code was published as a printed book by MIT Press, since books were protected speech and could legally be exported, then scanned back in abroad. The investigation was dropped in January 1996 without charges.`,
  },
  {
    id: 'clipper',
    title: 'The Clipper Chip',
    group: 'History',
    era: '1993 – 1996',
    body: `In April 1993 the US government proposed the Clipper Chip, a hardware encryptor for phones whose keys would be held in escrow by government agencies. Each chip attached a Law Enforcement Access Field (LEAF) to its traffic so that authorised wiretaps could recover the session key.

In 1994 Matt Blaze showed that the LEAF was protected by only a 16-bit checksum, so a user could forge a valid-looking LEAF and defeat the escrow while still using the chip. Between technical flaws, industry resistance and public opposition, Clipper was effectively dead by 1996.`,
    ingame: 'Escrow schemes appear in Cryptopolis as the Directorate\'s "joint custody" contracts.',
  },
  {
    id: 'bernstein',
    title: 'Code is speech',
    group: 'History',
    era: '1995 – 2000',
    body: `Graduate student Daniel J. Bernstein wanted to publish his encryption algorithm, Snuffle, and the source code implementing it. Export rules treated the code as a munition. With the Electronic Frontier Foundation he sued in 1995.

In 1999 a federal appeals court held that source code is a form of expression protected by the First Amendment. That ruling was later withdrawn for rehearing, but by 2000 the government had relaxed the export regulations, and the case had already changed how courts and the public thought about software.`,
  },
  {
    id: 'venona',
    title: 'Venona',
    group: 'History',
    era: '1943 – 1980',
    body: `A US counter-intelligence programme that decrypted parts of Soviet intelligence traffic. The Soviets used one-time pads, which are unbreakable when used correctly. But under wartime pressure in 1942, some pad pages were produced in duplicate, so a small fraction of messages were encrypted with key material that had been used before.

Analysts found those reuses and, over decades, recovered portions of thousands of messages. The programme was kept secret until 1995. Its lesson is the one in the name: a one-time pad is only secure if it is used one time.`,
    ingame: 'Mara\'s intercepted messages reproduce the Venona mistake in miniature.',
  },
  {
    id: 'gchq',
    title: 'The secret discovery of public keys',
    group: 'History',
    era: '1970 – 1997',
    body: `In 1997 the British signals agency GCHQ revealed that its researchers had found public-key ideas years before they were published in the open. James Ellis described "non-secret encryption" in 1970; Clifford Cocks found an RSA-like scheme in 1973; Malcolm Williamson found a Diffie–Hellman-like key exchange around 1974.

None of it was used or published at the time. The open discoveries by Diffie, Hellman, Merkle, Rivest, Shamir and Adleman are what actually reached the world, which is a quiet argument for doing cryptography in public.`,
  },
  {
    id: 'bitcoin',
    title: 'Bitcoin',
    group: 'History',
    era: '2008 – today',
    body: `A peer-to-peer electronic cash system described in a whitepaper by the pseudonymous Satoshi Nakamoto on October 31, 2008, and launched on January 3, 2009. It combines ideas that had been discussed on cypherpunk lists for years: digital signatures for ownership, hash-based proof-of-work for ordering, and a public append-only ledger.

Ownership in Bitcoin is nothing more than the ability to produce a valid signature. That makes key management, key derivation and signature schemes the heart of the system, and the reason so many chapters of this game speak its dialect: BIP-32, BIP-39, BIP-340, BIP-341.`,
  },
  {
    id: 'dual-ec',
    title: 'Dual_EC_DRBG',
    group: 'History',
    era: '2006 – 2014',
    body: `A random number generator standardised by NIST in 2006. In 2007 Dan Shumow and Niels Ferguson pointed out that whoever chose its two elliptic-curve constants could have kept a secret relationship between them, letting them predict the generator's output from a small sample.

In 2013, documents leaked by Edward Snowden were reported to show that the NSA had promoted the standard. NIST withdrew its recommendation in 2014. The episode is the textbook example of a *kleptographic* backdoor: a weakness that is invisible to everyone except the party holding a secret.`,
  },
  {
    id: 'going-dark',
    title: 'Going dark and exceptional access',
    group: 'History',
    era: '2014 – today',
    body: `Law enforcement agencies argue that end-to-end encryption and encrypted devices increasingly prevent them from accessing evidence even with a warrant, a problem they call "going dark". They have asked for mechanisms that would let authorised parties decrypt data in exceptional cases.

Cryptographers have largely argued that such mechanisms cannot be built without weakening security for everyone. The 2015 report *Keys Under Doormats*, by fifteen prominent security researchers, made this case in detail: exceptional access adds complexity, concentrates valuable keys, and conflicts with forward secrecy. The 2016 Apple–FBI dispute over a locked iPhone brought the debate to the public. It continues in many countries today.`,
  },
  {
    id: 'nist-pqc',
    title: 'NIST post-quantum standards',
    group: 'History',
    era: '2016 – 2024',
    body: `In 2016 NIST opened a public competition for cryptography that resists quantum computers. Dozens of schemes were submitted and attacked in the open. Some fell spectacularly: in 2022 the isogeny scheme SIKE was broken with a laptop in about an hour, and the multivariate signature Rainbow was broken too.

In August 2024 NIST published the first three standards: FIPS 203 (ML-KEM, from Kyber), FIPS 204 (ML-DSA, from Dilithium) and FIPS 205 (SLH-DSA, from SPHINCS+). The first two are built on lattices.`,
  },

  // --------------------------------------------------------- Building blocks
  {
    id: 'bip39',
    title: 'BIP-39 mnemonic codes',
    group: 'Building blocks',
    era: '2013',
    body: `A way to write a random secret as words. 128 bits of entropy get a checksum of 4 bits (the first bits of SHA-256 of the entropy), and the resulting 132 bits are cut into twelve 11-bit numbers, each an index into a list of 2048 words.

The words are turned into a 64-byte seed with PBKDF2-HMAC-SHA512, 2048 iterations, salt \`"mnemonic"\` plus an optional passphrase. The checksum catches most typos but is only 4 bits: a random wrong word passes it one time in sixteen.`,
    ingame: 'The prologue makes you use exactly that 1-in-16: the checksum narrows the missing word, and a fingerprint finishes the job.',
  },
  {
    id: 'bip32',
    title: 'HD wallets, fingerprints and BIP-86',
    group: 'Building blocks',
    era: '2012 / 2021',
    body: `BIP-32 derives a whole tree of keys from one seed. A path like \`m/86'/0'/0'/0/0\` walks the tree; an apostrophe marks *hardened* steps, which cannot be computed from public information.

A key's **fingerprint** is the first 4 bytes of HASH160 (RIPEMD-160 of SHA-256) of its public key. Wallet descriptors start with the master fingerprint, like \`[73c5da0a/86h/0h/0h]\`, so a device can tell whether a seed is the right one without exposing any key. BIP-86 fixes the path for single-key Taproot accounts.`,
    ingame: "The sticky note on Grandpa's monitor is a descriptor. Its fingerprint is how you know you recovered the right words.",
  },
  {
    id: 'hash',
    title: 'Hash functions and tagged hashes',
    group: 'Building blocks',
    era: '1990s – today',
    body: `A cryptographic hash such as SHA-256 maps any input to a fixed-size digest. We want it to be one-way, collision resistant and to behave, as far as anyone can tell, like a random function.

BIP-340 introduced *tagged hashes*: \`H_tag(x) = SHA256(SHA256(tag) || SHA256(tag) || x)\`. Prefixing every use with a different tag means a hash computed for one purpose can never be mistaken for one computed for another, a cheap and effective form of domain separation.`,
  },
  {
    id: 'otp',
    title: 'The one-time pad',
    group: 'Building blocks',
    era: '1917 / 1949',
    body: `Gilbert Vernam patented XOR-based teleprinter encryption in 1917; Joseph Mauborgne realised the key must be random and never reused. In 1949 Claude Shannon proved such a *one-time pad* achieves perfect secrecy: the ciphertext reveals nothing about the message except its length.

The catch is the key: it must be truly random, as long as all the traffic, delivered in advance, and destroyed after use. Every practical cipher since is an attempt to get most of the pad's strength from a short key.`,
  },
  {
    id: 'aead',
    title: 'Authenticated encryption (AES-GCM)',
    group: 'Building blocks',
    era: '2001 / 2007',
    body: `AES became the US standard block cipher in 2001. In counter (CTR) mode it produces a keystream that is XORed with the message, like a pad. That gives confidentiality but not integrity: flip a ciphertext bit and the same plaintext bit flips.

Galois/Counter Mode (GCM, standardised by NIST in 2007) adds a 128-bit authentication tag computed over the ciphertext and any associated data. Decryption checks the tag first and refuses to output anything if it does not match. Modern protocols use authenticated encryption (AEAD) almost exclusively. One rule: never reuse a nonce with the same GCM key.`,
  },
  {
    id: 'dh',
    title: 'Diffie–Hellman key exchange',
    group: 'Protocols',
    era: '1976',
    body: `Whitfield Diffie and Martin Hellman's *New Directions in Cryptography* (1976) showed two strangers how to agree on a secret over a public channel. Each picks a secret exponent, publishes g to that power, and raises the other's value to their own secret: both get g^(ab).

An eavesdropper sees g^a and g^b but computing g^(ab) from them is believed to require solving a discrete logarithm. The exchange says nothing about *who* is on the other end, so on its own it is defenceless against an active attacker in the middle. A sting from history: in 2015 the Logjam attack exploited servers that still accepted deliberately weakened 512-bit "export-grade" Diffie–Hellman, a leftover of 1990s export controls.`,
  },
  {
    id: 'dlog',
    title: 'Discrete logarithms and baby-step giant-step',
    group: 'Building blocks',
    era: '1971',
    body: `Given g and y = g^x in a group of order q, find x. In well-chosen groups no efficient classical algorithm is known, and generic algorithms need about the square root of q steps.

Daniel Shanks's baby-step giant-step (1971) achieves that bound with a time-memory trade-off: write x = i·m + j with m ≈ √q, tabulate g^j, then step y·g^(−m·i) until you land in the table. A 31-bit group falls in milliseconds. A 256-bit elliptic-curve group would need about 2^128 steps.`,
  },
  {
    id: 'shamir',
    title: 'Shamir secret sharing',
    group: 'Protocols',
    era: '1979',
    body: `Adi Shamir's *How to Share a Secret* (1979): to split a secret s so that any t of n people can recover it, choose a random polynomial f of degree t−1 with f(0) = s, and give person i the share f(i). Any t points determine the polynomial by Lagrange interpolation; any t−1 points are consistent with every possible secret, so they reveal nothing.

The arithmetic happens in a finite field, which is what makes the "reveal nothing" claim exact. George Blakley published a geometric scheme the same year. Threshold signatures such as FROST (RFC 9591, 2024) build on the same idea so the full key never exists in one place.`,
  },
  {
    id: 'feldman',
    title: 'Verifiable secret sharing',
    group: 'Protocols',
    era: '1987',
    body: `In plain Shamir sharing a dishonest dealer or a tampered share goes unnoticed until reconstruction fails. Paul Feldman's 1987 scheme fixes this: the dealer publishes commitments C_j = a_j·G to each coefficient of the polynomial.

Anyone holding share (i, y) can check y·G = C_0 + i·C_1 + i²·C_2 + … without learning the secret, because the commitments hide the coefficients behind discrete logarithms. C_0 is simply the public key of the shared secret.`,
  },
  {
    id: 'schnorr',
    title: 'Schnorr signatures (BIP-340)',
    group: 'Protocols',
    era: '1989 / 2020',
    body: `Claus-Peter Schnorr's signature: pick a nonce k, publish R = k·G, compute the challenge e = H(R || P || m) and the response s = k + e·d. Verify by checking s·G = R + e·P. It is short, simple, provably secure in the random-oracle model, and *linear*, which makes multi-signatures and adaptor tricks natural.

A patent kept it out of widespread use until 2008; ECDSA filled the gap. Bitcoin adopted Schnorr signatures as BIP-340, activated with Taproot in November 2021, using x-only public keys and tagged hashes.`,
  },
  {
    id: 'musig',
    title: 'MuSig2 key aggregation (BIP-327)',
    group: 'Protocols',
    era: '2018 / 2023',
    body: `Several signers can produce one ordinary-looking Schnorr signature under one aggregate key. The naive aggregate P1 + P2 is unsafe (see *Rogue-key attacks*). MuSig weights each key by a coefficient a_i = H(L || P_i), where L hashes the full list of keys, so no signer can choose a key that cancels the others.

MuSig2 (Nick, Ruffing and Seurin, 2021) needs only two rounds of communication; BIP-327 standardises it for Bitcoin.`,
  },
  {
    id: 'taproot',
    title: 'Taproot (BIP-341)',
    group: 'Protocols',
    era: '2021',
    body: `A Taproot output is a single public key Q that can be spent two ways. The *key path*: a signature for Q. The *script path*: reveal one script from a Merkle tree of alternatives and satisfy it. The tree is hidden inside the key by a tweak: Q = P + H(P || root)·G.

If everyone cooperates, a complex contract looks exactly like a single-signature payment. If not, only the branch that is used is ever revealed.`,
    ingame: "Grandpa's address hides two leaves. One of them is a promise to you.",
  },
  {
    id: 'sigma',
    title: 'Sigma protocols and zero knowledge',
    group: 'Protocols',
    era: '1985 – 1996',
    body: `Goldwasser, Micali and Rackoff defined zero-knowledge proofs in 1985: convincing someone a statement is true while revealing nothing else. Schnorr's identification protocol is the classic example. The prover sends R = r·G, the verifier a random challenge c, the prover s = r + c·x; the verifier checks s·G = R + c·X.

Three properties make it work. *Completeness*: an honest prover always passes. *Special soundness*: two answers to different challenges for the same R reveal x, so anyone who can answer reliably must know x. *Zero knowledge*: choosing c and s first and setting R = s·G − c·X produces transcripts with exactly the same distribution, so a transcript proves nothing to a third party. Ronald Cramer named this commit-challenge-response shape a Σ-protocol.`,
  },
  {
    id: 'fiat-shamir',
    title: 'The Fiat–Shamir transform',
    group: 'Protocols',
    era: '1986',
    body: `Amos Fiat and Adi Shamir observed that the verifier's random challenge can be replaced by a hash of everything said so far: c = H(R, X, context). The interactive proof becomes a non-interactive one that anyone can check, and a Schnorr identification proof becomes a Schnorr signature.

The hash must include *everything* the challenge should depend on: the statement, the commitment and the context. Leaving things out has caused real vulnerabilities.`,
  },
  {
    id: 'lattice',
    title: 'Lattices and LLL',
    group: 'Post-quantum',
    era: '1982',
    body: `A lattice is every integer combination of some basis vectors. The same lattice has many bases; a good one is short and nearly orthogonal, a bad one long and skewed. Finding a shortest nonzero vector (SVP) is believed hard in high dimension.

Lenstra, Lenstra and Lovász published their reduction algorithm in 1982. LLL runs in polynomial time and finds a vector within a factor about 2^(n/2) of the shortest, often far better in practice. Watching the Gram–Schmidt lengths of a basis flatten out as LLL works is the clearest picture of what reduction does.`,
  },
  {
    id: 'knapsack',
    title: 'The knapsack cryptosystem and its fall',
    group: 'Post-quantum',
    era: '1978 – 1992',
    body: `Ralph Merkle and Martin Hellman's 1978 knapsack system hid an easy subset-sum problem (a superincreasing sequence) inside a hard-looking one by modular multiplication. It was one of the first public-key schemes and was faster than RSA.

In 1982 Adi Shamir broke the basic version. Lagarias and Odlyzko (1985) and then Coster, Joux, LaMacchia, Odlyzko, Schnorr and Stern (1992) showed that lattice reduction solves any subset-sum instance of low enough density. The lesson is not "lattices are dangerous". It is "structure you hid can be found", and lattices are the tool that finds it.`,
    ingame: "The Directorate's QSAFE channel is a Merkle–Hellman knapsack with a new logo.",
  },
  {
    id: 'lwe',
    title: 'Learning With Errors',
    group: 'Post-quantum',
    era: '2005',
    body: `Oded Regev's problem: given many random vectors a_i and values b_i = ⟨a_i, s⟩ + e_i with small errors e_i, find s. Without the errors, Gaussian elimination solves it instantly. With them, the best known attacks, classical and quantum, are lattice reduction, and they take exponential time for good parameters.

Regev also showed LWE is as hard as worst-case lattice problems, building on Ajtai's 1996 work. Encryption from LWE carries a small amount of noise that must stay below a threshold for decryption to work: correctness and security pull on the same dial.`,
  },
  {
    id: 'mlkem',
    title: 'ML-KEM and ML-DSA',
    group: 'Post-quantum',
    era: '2024',
    body: `ML-KEM (FIPS 203) is a key-encapsulation mechanism based on Module-LWE, standardised from CRYSTALS-Kyber. At the ML-KEM-768 level a public key is 1184 bytes and a ciphertext 1088 bytes; both sides end up with the same 32-byte secret. ML-DSA (FIPS 204), from CRYSTALS-Dilithium, is the matching signature scheme.

Deployment started before the standards were final: Signal added a post-quantum handshake in 2023, and major browsers enabled hybrid X25519 + Kyber/ML-KEM key exchange in 2024. Hybrids keep a classical key exchange in the mix, so a flaw in either half alone does not break the session.`,
  },
  {
    id: 'shor',
    title: "Shor's algorithm and harvest-now-decrypt-later",
    group: 'Post-quantum',
    era: '1994 – today',
    body: `In 1994 Peter Shor showed that a large quantum computer could factor integers and compute discrete logarithms in polynomial time. That would break RSA, Diffie–Hellman and elliptic-curve cryptography, including the Schnorr signatures and key exchanges in this game.

No quantum computer large enough exists today. But encrypted traffic recorded now can be decrypted whenever one does: *harvest now, decrypt later*. That is why key exchange is being migrated to post-quantum schemes first.`,
  },

  // ------------------------------------------------------------------ Attacks
  {
    id: 'two-time-pad',
    title: 'Pad reuse',
    group: 'Attacks',
    era: 'classic',
    body: `Encrypt two messages with the same pad and XOR the ciphertexts: the pad cancels and P1 ⊕ P2 remains. Natural language is redundant enough that guessing a likely word in one message ("crib dragging") reveals the matching letters of the other, which suggests the next guess, and so on.

The same algebra breaks any stream cipher, including AES-CTR and AES-GCM, whenever a key and nonce pair is reused.`,
  },
  {
    id: 'malleability',
    title: 'Malleability',
    group: 'Attacks',
    era: 'classic',
    body: `A cipher is malleable if an attacker can change the ciphertext in a way that changes the plaintext predictably. With any XOR-based stream cipher, C ⊕ Δ decrypts to P ⊕ Δ. If the attacker knows or guesses part of the plaintext, they can rewrite it to anything of the same length, without the key.

Integrity checks that are not cryptographic (CRC, length fields) do not help. Authentication tags (MACs, AEAD) do.`,
  },
  {
    id: 'mitm',
    title: 'Man-in-the-middle',
    group: 'Attacks',
    era: 'classic',
    body: `An active attacker relays traffic between two parties while impersonating each to the other. Against unauthenticated Diffie–Hellman it is complete: the attacker runs one exchange with each side, decrypts everything, and can rewrite messages in flight.

The fix is to authenticate the key exchange, typically by having one or both sides sign their public values (as TLS 1.3 does with a signature over the handshake transcript), with keys that are known in advance or certified by someone trusted.`,
  },
  {
    id: 'nonce-reuse',
    title: 'Nonce reuse in signatures',
    group: 'Attacks',
    era: '2010 / 2013',
    body: `In Schnorr and ECDSA, the nonce k must be secret, unpredictable and never reused. Two signatures s1 = k + e1·d and s2 = k + e2·d with the same k give d = (s1 − s2)/(e1 − e2): the private key falls out of one subtraction and one division.

This has happened. At the end of 2010 researchers showed that Sony signed PlayStation 3 software with ECDSA using a constant nonce, exposing the signing key. In 2013 a flaw in Android's random number generator led Bitcoin wallets to repeat nonces, and coins were stolen. Modern schemes derive k deterministically from the key and message (RFC 6979, BIP-340), often mixed with fresh randomness.`,
  },
  {
    id: 'rogue-key',
    title: 'Rogue-key attacks',
    group: 'Attacks',
    era: 'classic',
    body: `When an aggregate key is the plain sum of participants' keys, the last participant to announce a key can choose P_rogue = P_target − (sum of the others). The aggregate becomes P_target, a key they control alone, while everyone else believes it requires all of them.

Defences: require each participant to prove knowledge of their secret key, or, as MuSig does, weight each key by a hash of the whole key list.`,
  },
  {
    id: 'weak-fs',
    title: 'Weak Fiat–Shamir',
    group: 'Attacks',
    era: '2022',
    body: `If the Fiat–Shamir challenge leaves out part of the statement or context, a prover may be able to adapt the statement after seeing the challenge, or reuse a proof somewhere it was not meant for. In 2022 Trail of Bits disclosed a family of such bugs, nicknamed *Frozen Heart*, in several open-source proof-system implementations.

The rule of thumb: hash the full transcript, including the public statement, the parameters and a session identifier.`,
  },
];

export const codexById = (id: string): CodexEntry | undefined => CODEX.find((e) => e.id === id);

export const CODEX_GROUPS: CodexGroup[] = ['History', 'Building blocks', 'Protocols', 'Attacks', 'Post-quantum'];
