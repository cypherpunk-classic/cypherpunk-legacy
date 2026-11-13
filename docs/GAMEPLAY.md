# Gameplay: v3.1, "Cypherpunks write code"

v3 put real cryptography under every puzzle. Playtesting showed the weak spot:
too many terminals had one big button that did the cryptography *for* you. This
revision is about the player doing the work, about the cypherpunks' ideas
showing up as mechanics rather than as quotes, and about a city you can live
in between puzzles.

Three rules guide every change:

1. **You do the decisive step.** The computer verifies; it does not solve. Pick
   the two signatures that share a nonce, bend the curve through the third
   share, build the response from tokens, wrap the onion, choose the blinding
   factor. A wrong move gets a specific, truthful reason, computed from the same
   cryptography.
2. **Ideals become systems.** Privacy is a trace meter you can actually keep
   low. Trust is a web of signed keys that grows across the map. Anonymous
   money is a wallet of blind-signed tokens that the mint cannot link to you.
   Code is speech is a book you have to get past a scanner.
3. **Choices cost something.** Snooping, logging, a licence, a stolen key: each
   is offered at the moment it is tempting, and the city remembers.

## Layered difficulty

Every terminal shows a **step list** beside it (what you have done, what is
next, an optional hint for the current step). The *Puzzle hints* setting
chooses between **Guided** (steps and hints) and **Minimal** (steps only;
some terminals, like lattice reduction, also stop showing intermediate values
such as μ). Most terminals offer a **Deep dive** once solved: the same idea
redone by hand, usually in the toy group p = 1019, q = 509, g = 4, against a
clock, with no buttons to help. A finished deep dive pays 4 cr and a badge in
the save. The main story never requires one.

The *Surveillance* setting turns the Dragnet's flagging off (**Story only**):
sensors still light up and the codex still explains them, but the trace only
ever falls.

## The original nineteen terminals, reworked

| terminal | before | now you |
|---|---|---|
| Seed restore | a sweep found the word | let the sweep do the cheap filters, then derive each survivor's seed and compare fingerprints digit by digit |
| Gateway login | "Sign" | roll a nonce, then build `s = k + e·d` from tokens; the gateway shows both sides of `s·G = R + e·P` |
| Radio log (two-time pad) | already hands-on | the same, with steps and an XOR-by-hand deep dive |
| Ticket gate (CTR vs GCM) | already hands-on | the same, with steps and the five masks by hand |
| Key exchange | "Compute", "Send" | build your key `K = □^□` from what you hold; using b, or A^a, or g^a gets the reason it fails |
| Line 7 (MITM) | press both swaps | swap, read, rewrite, and choose which of your two keys control holds (the keys are now attached to the side that really got your M) |
| Logjam | m prefilled with √q | size the table yourself under a memory limit and before the link re-keys |
| Shamir | interpolation filled in the answer | bend the curve through Juno's point by moving the secret |
| Feldman | "Verify" said VALID or FORGED | compare `y·G` with `C₀ + x·C₁ + x²·C₂` yourself, choose three |
| Nonce reuse | "Recover key" | find the pair that shares R among three orders, then build `d = (s₁ − s₂)/(e₁ − e₂)` |
| Rogue key | watch | play the Directorate: choose `P_DLA` so Q = T, then chase MuSig2's coefficient and watch it move |
| Taproot | already a choice | the same, with steps and a control-block deep dive |
| Checkpoint (Σ-protocol) | "Respond" | build `s = r + c·x`, then pass three rounds |
| Real or simulated | guess | first forge a transcript yourself (`R = s·G − c·X`), then guess |
| Extractor and Fiat–Shamir | "Extract", one checkbox | rewind a prover VM to get two answers for one R, solve for x, then choose what the hash covers while an auditor tries replay, forgery without x and related keys |
| Lattice reduction | "Auto-reduce" | choose every multiple k yourself |
| QSAFE knapsack | "Break all" | run LLL, then find the ±1 row, pick its sign and check the subset sum; the rest follow the same rule |
| After the quantum (LWE) | slider and a button | read each bit from its phase, push the noise until decryption breaks, then run ML-KEM-768 |
| Broadcast | sign, broadcast | sign, verify it yourself, see who carries it, broadcast |

## Systems

### Surveillance: the Dragnet

The Directorate watches the streets with two kinds of sensor, placed in the map
data: **masts** (a pulsing ring that logs every device inside it) and
**cameras** (a cone that sweeps back and forth). While a sensor sees you, your
**trace** rises by its gain (22 per second for a mast, 34 for a camera)
multiplied by your **opsec**; outside every field it falls by 14 per second.
The first time a sensor logs you, the codex entry on metadata unlocks. Opsec
grows with the tools you earn:

| tool | earned | trace gain |
|---|---|---|
| sealed messenger | Chapter 1, from Mara | ×0.8 |
| signed keyring | the key-signing party | ×0.8 |
| eCash wallet | the Commons Mint | ×0.85 (no card payments to log) |
| Faraday pouch | the hardware stall, 6 cr | ×0.6 |
| onion route | the remailers | ×0.7 |
| mesh radio | the hardware stall, 8 cr | ×0.9 |
| allies | anyone who signs your key | ×(1 − 0.02 per ally), down to ×0.75 |

A bare phone under a mast is flagged in about four and a half seconds; with
everything above it takes several times longer. At 100 you are **flagged**: a
drone and an agent, a fine of 2 cr (taken from IOUs first), and a walk to the
nearest open safe house (the Wharf from the start, then the Exchange, the
Commons, the Mint, the Masquerade and the Observatory as their chapters
close). The two escrow taps on the north avenue go dark for good once you have
broken the Clipper LEAF, and after the broadcast the whole Dragnet shuts down.
The autopilot plans around fields with a path cost, so it is almost never
flagged.

### Allies and the web of trust

Citizens and the people you meet in the story can become **allies**.
Recruiting someone means comparing their key fingerprint and signing it with
your identity key: a real BIP-340 certification over the fingerprints and user
id, stored in the save and re-verified whenever the Network tab draws the
graph. Allies sign each other too, and validity is computed the way GnuPG does
it (signed by you, or by one fully trusted or three marginally trusted valid
keys, paths up to five long). You set owner trust per ally in the Network tab;
the remailer mission depends on it, because two keys both claim to be
Kestrel's. Allies lower your trace, and at the finale each of them rebroadcasts
the manifesto: the crowd on the plaza is your web.

Who can join: Mara, Felix and Juno at the key-signing party (and Ines, unless
you read the co-op's mail), Ibrahim, Lin and Ada as their chapters close, Rosa,
Theo and Wen after their side quests, and three citizens with conditions of
their own: Bao wants you to compare fingerprints properly, Kai wants to be paid
for a coffee in untraceable cash, Odile needs a Hashcash stamp, and Ravi comes
back to the party with his ID.

### Anonymous economy

The **Commons Mint** issues Chaumian eCash with blind Diffie–Hellman on
secp256k1 (the scheme Cashu and Fedimint use today; David Wagner's 1996 variant
of Chaum's blind signatures). Withdrawing: you pick a secret, blind it with a
factor only you know, the mint signs the blinded point and proves with a DLEQ
proof that it used its public key, and you unblind. When a shop deposits the
token, the mint sees a secret and a signature it has never seen before and
cannot connect to any withdrawal. Double spends hit the mint's spent list.

Before you have a wallet, rewards arrive as **IOUs** (credit notes with your
name on them); the mint swaps them for tokens when you open your wallet. Income:
Sol's welcome (10 cr), side quests (4 each), deep dives (4 each), all six
floppies (6). Spending: the hardware stall (Faraday pouch, mesh radio), the
vending machines (coffee, and one dried fish for Nonce, once), Kai's coffee,
and fines. Every purchase is a real redemption at the mint, and your change
comes back as freshly blinded tokens.

**Hashcash stamps** are the second currency: real SHA-256 proof of work,
minted in a terminal that measures your machine's hash rate, needed to post
through the remailers, for Odile, and for the final broadcast (the board will
sell you more postage for work whenever you run out).

### Side quests

| quest | where | idea |
|---|---|---|
| The Source | the Wharf, Rosa the reporter | check the GCM tag before you trust a drop; spot the forgery made from a reused nonce |
| Yellow Dots | the Exchange, Theo the clerk | colour printers leak a serial number and a timestamp in tracking dots |
| The Ledger | the Commons, Wen the grocer | a hash chain shows exactly where history was edited; a full rewrite needs anchors published elsewhere |
| Grandpa's Floppies | all over the city | six disks with lore from the mailing-list years |

Each pays credits and unlocks codex entries; Rosa, Theo and Wen sign your key
once you have one. The quest log lives in the menu.

## New missions from cypherpunk history

| mission | chapter | history | you do |
|---|---|---|---|
| The Book | 2, the Exchange | PGP source printed as a book (1995); *Bernstein v. US* | fix OCR errors line by line against printed CRC-16 checksums, then compile |
| The Commons Mint | 3, the Commons | Chaum (1982), DigiCash, blind DH (1996) | blind, get signed, check the DLEQ proof, unblind, spend twice, then fail to link a withdrawal to a deposit as the analyst |
| Key-signing party | after the Vault, the Exchange | PGP (1991), the web of trust; Evil32 short-ID collisions (2014) | look up each slip on the keyserver, catch the key whose short ID collides, check IDs, sign or refuse |
| The Clipper | 4, the Mint | the Clipper chip (1993), Matt Blaze's LEAF attack (1994) | estimate the work, then search for a fake LEAF that passes the 16-bit checksum |
| The Remailers | 5, the Masquerade | anon.penet.fi (1993–96), Cypherpunk and Mixmaster remailers | pick the valid Kestrel key with your trust settings, wrap three layers from the inside out, then beat a traffic analyst with padding and batching |
| Stamps | 5, the Masquerade | Hashcash (Adam Back, 1997) | price postage from your own hash rate so a flood is ruinous, mint three stamps, post one, try to replay it |

## Choices

| when | choice | consequence |
|---|---|---|
| Chapter 1 | read only the Directorate's traffic, or everyone's | Mara's opinion; Ines will not sign your key |
| Chapter 3 | Sol's mint stays blind, or takes the Directorate's licence and logs names | before the broadcast the list is seized; Kai and Odile leave the city |
| Chapter 4 | Warden's key: use it once and disclose it, publish it, or keep it | Ibrahim's or Warden's last words, and the ending |
| Chapter 5 | your remailer keeps a table of pseudonyms, or none | before the broadcast, a court order: hand over Kestrel's address, or wipe the node and lose the onion route |

## Endings

The finale adds up a tally, and the credits show every line of it: one point
per ally on your web, +2 if the book got out, +3 for disclosing Warden's key
(−1 for publishing it, −3 for keeping it), −2 for snooping, −2 for the mint's
licence, −5 for giving up Kestrel (or −1 for having kept the table at all),
and −1 for being flagged more than five times.

- **An Open Society**: 12 or more, and nobody given up. The main story played
  on principle reaches exactly 12; side quests make room for mistakes.
- **A Quiet Victory**: 6 or more. The Archive is public, but the network is
  thin or compromised.
- **The Long Winter**: anything less. The Directorate keeps its grip for now;
  the code is out there, and that is how it starts again.

The headless tests play all three: the main story (An Open Society), a
completionist run with every side quest, and two runs with different answers
(The Long Winter and A Quiet Victory).
