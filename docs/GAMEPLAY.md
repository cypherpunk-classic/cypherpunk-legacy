# Gameplay: v3.1, "Cypherpunks write code"

v3 put real cryptography under every puzzle. Playtesting showed the weak spot:
too many terminals had one big button that did the cryptography *for* you. This
revision is about the player doing the work, about the cypherpunks' ideas
showing up as mechanics rather than as quotes, and about a city you can live
in between puzzles.

Three rules guide every change:

1. **You do the decisive step.** The computer verifies; it does not solve. Pick
   the two signatures that share a nonce, slide the share onto the curve, type
   the crib, wrap the onion, choose the blinding factor. A wrong move gets a
   specific, truthful reason, computed from the same cryptography.
2. **Ideals become systems.** Privacy is a trace meter you can actually keep
   low. Trust is a web of signed keys that grows across the map. Anonymous
   money is a wallet of blind-signed tokens that the mint cannot link to you.
   Code is speech is a book you have to get past a checkpoint.
3. **Choices cost something.** Logging, escrow, publishing a stolen key: each
   is offered at the moment it is tempting, and the city remembers.

## Layered difficulty

Every terminal shows a **step list** on the left (what you have done, what is
next, an optional hint per step). The *Hints* setting chooses between
**Guided** (steps and hints visible) and **Minimal** (steps only, hints hidden).
Most terminals also offer a **Deep dive** once solved: a harder variant (do the
arithmetic yourself in a toy group, break it against a clock, no hints) that
pays out eCash and a badge in the save. The main story never requires a deep
dive.

## Systems

### Surveillance: the Dragnet

The Directorate watches the streets with two kinds of sensor, placed in the map
data: **masts** (a pulsing radius that logs every device nearby) and **cameras**
(a cone that sweeps back and forth). While you stand in an active field your
**trace** rises; outside it decays. What a sensor records is metadata, so how
fast trace rises depends on your **opsec**, which grows with the tools you earn:

| tool (item) | earned | effect |
|---|---|---|
| sealed messenger | Chapter 1 | trace gain ×0.8 |
| signed keyring | key-signing party | ×0.8 |
| eCash wallet | the Commons Mint | ×0.85 (no card payments to log) |
| Faraday pouch | bought at the stall | ×0.6 |
| onion route | the remailers | ×0.7 |

At 100 you are **flagged**: an agent stops you, fines you a few credits and
walks you back to the nearest safe house. Escrow taps (one kind of mast) go dark
for good once you have broken the Clipper LEAF. After the broadcast, the
Dragnet shuts down. The autopilot plans around sensor fields with a cost
penalty, so it rarely gets flagged.

### Allies and the web of trust

Citizens, and the people you meet in the story, can become **allies**. Recruiting
someone means verifying their key fingerprint and signing it with your identity
key: a real BIP-340 signature over the fingerprint, stored in the save. Allies
sign each other too, and validity is computed the way GnuPG does it (signed by
you, or by one fully trusted or three marginally trusted valid keys, paths up to
five long). The **Network** tab draws the graph. Allies run relays (lower
trace), open safe houses, and in the finale each of them rebroadcasts the
manifesto: the ending depends on how many voices you have.

### Anonymous economy

The **Commons Mint** issues Chaumian eCash with blind Diffie–Hellman key exchange
on secp256k1 (the scheme Cashu and Fedimint use today; David Wagner's 1996
variant of Chaum's blind signatures). Withdrawing: you pick a secret, blind it
with a factor only you know, the mint signs the blinded point, you unblind. When
a shop deposits the token, the mint sees a secret and a signature it has never
seen before and cannot connect to any withdrawal. Double spends are rejected by
the mint's spent list.

Credits come from quest rewards and deep dives; they are spent at the stall
(the Faraday pouch, a mesh radio), at the print shop (the book), at vending
machines, and on fines. **Hashcash stamps** are the second currency: real SHA-256
proof-of-work, minted in a terminal, needed to post through the remailers and
for the final broadcast.

### Side quests

| quest | where | idea |
|---|---|---|
| The Source | Wharf, Rosa the reporter | authenticate before you trust; never reuse a nonce |
| Yellow Dots | Exchange, Theo the clerk | printers leak a serial number and a timestamp in tracking dots |
| The Ledger | Commons, Wen the grocer | a hash chain shows exactly where history was rewritten |
| Grandpa's Floppies | all over | six disks with lore from the mailing-list years |

Each pays credits, recruits an ally and unlocks codex entries. The quest log
lives in the menu.

## New missions from cypherpunk history

| mission | chapter | history | you do |
|---|---|---|---|
| Key-signing party | 2, the Exchange | PGP (1991), the web of trust; Evil32 short-ID collisions (2014) | compare fingerprints, catch the impostor whose short ID collides, sign the rest |
| The Book | 2, the Exchange | PGP source printed as a book (1995); *Bernstein v. US* | fix OCR errors line by line against printed checksums |
| The Commons Mint | 3, the Commons | Chaum (1982), DigiCash, blind DH (1996) | blind, get signed, unblind, spend; watch the mint fail to link you |
| The Clipper | 4, the Mint | the Clipper chip (1993), Matt Blaze's LEAF attack (1994) | brute-force a fake LEAF past its 16-bit checksum |
| The Remailers | 5, the Masquerade | anon.penet.fi (1993–96), Cypherpunk and Mixmaster remailers | wrap a message in onion layers, then beat a traffic analyst by mixing |
| Stamps | 5, the Masquerade | Hashcash (Adam Back, 1997) | mint proof-of-work stamps, price out a spam flood |

## Choices

| when | choice | consequence |
|---|---|---|
| Chapter 1 | read only the Directorate's traffic, or everyone's | Mara's trust; one ally fewer if you snoop |
| Chapter 3 | the mint keeps a "compliance log", or stays blind | a later subpoena exposes allies if it logged |
| Chapter 4 | Warden's key: publish it, use it once and disclose, or keep it | the Archive's fate, Warden's last scene, the ending |
| Chapter 5 | your remailer keeps logs, or none | the anon.penet.fi moment: give up a user, or shut down |

## Endings

The finale tallies allies, choices, trace history and whether the book got out:

- **An Open Society**: many allies, no logs, key disclosed responsibly.
- **A Quiet Victory**: the Archive is public, but the network is thin or
  compromised.
- **The Long Winter**: the Directorate keeps its grip for now; the code is out
  there, and that is how it starts again.

Every ending reaches the credits; the credits show your numbers.
