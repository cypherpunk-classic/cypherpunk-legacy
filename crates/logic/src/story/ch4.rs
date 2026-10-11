//! Chapter 4 · Signatures. Ibrahim, Warden's nonce, rogue keys, Taproot.

use crate::puzzles::{NonceReuse, RogueKey, TaprootReveal};
use crate::say;
use crate::script::Ctx;
use crate::speakers::Sp;

async fn run_nonce(c: &Ctx) {
    if !c.puzzle(Box::new(NonceReuse::new(c.world()))).await {
        c.objective("Study Warden's signing log at the Mint.");
        return;
    }
    c.set("ch4.nonce");
    c.give("warden-key");
    c.codex(&["schnorr", "nonce-reuse"]);
    say!(
        c,
        (Ibrahim, "There it is. One reused nonce and his whole key falls out. You could sign the Directorate's surrender if you wanted."),
        (Ibrahim, "This is not hypothetical. Sony signed every PlayStation 3 with a constant nonce and lost the console in 2010. Android wallets repeated nonces in 2013 and people lost real money. Warden should have used deterministic nonces, RFC 6979, BIP-340. He bought the cheap signer."),
        (Ibrahim, "Use his key. Sign an order that unseals the Archive. The door at the back of the Mint is waiting for Warden's signature; it does not care that it is you holding the pen."),
    )
    .await;
    c.set("ch4.archive");
    c.objective("Unseal the Archive door at the Mint with the forged order.");
}

async fn run_archive(c: &Ctx) {
    if !c.flag("ch4.nonce") {
        say!(
            c,
            (Narrator, "A vault door, sealed with a Directorate signature lock: ORDER 0417. It wants a newer, signed order to open."),
            (Nonce, "You need Warden's key. Ibrahim has the log."),
        )
        .await;
        return;
    }
    if !c.flag("ch4.opened") {
        c.cutscene(async {
            let (ax, ay) = c.object_pos("archive").unwrap_or((0.0, 0.0));
            c.sfx("door");
            c.set_object_frame("archive", 1);
            c.burst(ax, ay - 20.0, 0x73eff7, 30);
            say!(
                c,
                (Narrator, "You feed the signed order into the lock. It checks the signature against Warden's public key, finds it valid, and has no way to know it did not come from him."),
                (Narrator, "The Archive door rolls open. Inside: shelf after shelf of it. Thirty years of letters, source code, arguments, mistakes."),
                (Ibrahim, "The things we wrote so the next people would not start from zero. Welcome to the actual legacy."),
            )
            .await;
            c.give("archive");
            c.set("ch4.opened");
        })
        .await;
    } else {
        c.say1(Sp::Narrator, "The Archive stands open. Someone has already started cataloguing it by hand.").await;
    }
    if !c.flag("ch4.rogue") {
        c.say1(Sp::Ibrahim, "Before you go. The Directorate is pushing \"joint custody\" on everyone now. Two-of-two, they say, so no one acts alone. It is a lie, and I can show you exactly how.").await;
        run_rogue(c).await;
    }
}

async fn run_rogue(c: &Ctx) {
    if !c.puzzle(Box::new(RogueKey::new(c.world()))).await {
        c.objective("See through the Directorate's joint-custody offer.");
        return;
    }
    c.set("ch4.rogue");
    c.codex(&["musig", "rogue-key"]);
    say!(
        c,
        (Ibrahim, "Naive key aggregation let them announce a key that cancels yours, and take sole control of a \"shared\" address. MuSig2 stops it by weighting every key with a hash of the whole set."),
        (Ibrahim, "One more thing, and this is the good one. Your grandfather used all of this himself. The Vault you opened was a Taproot address. Let me show you what he hid in it."),
    )
    .await;
    c.objective("Examine Grandpa's Taproot address (Ibrahim's terminal).");
}

async fn run_taproot(c: &Ctx) {
    if !c.puzzle(Box::new(TaprootReveal::new(c.world()))).await {
        c.objective("Find the scripts hidden in Grandpa's Taproot address.");
        return;
    }
    c.set("ch4.taproot");
    c.codex(&["taproot"]);
}

pub async fn ibrahim(c: Ctx) {
    if !c.flag("ch4.met") {
        c.set("ch4.met");
        say!(
            c,
            (Ibrahim, "You opened the Vault. Half the city felt that. I'm Ibrahim. I used to audit the Directorate's cryptography, before I started auditing it *against* them."),
            (Ibrahim, "They sealed the Archive, your grandfather's Archive, behind one of Warden's signatures. Order 0417. Everything he and his friends ever wrote, locked up for \"curation\"."),
            (Ibrahim, "Here is the thing about signatures. They prove a message came from the holder of a key. They are only as safe as that key, and Warden has been careless. Look at his signing log."),
        )
        .await;
        run_nonce(&c).await;
        return;
    }
    if !c.flag("ch4.nonce") {
        return run_nonce(&c).await;
    }
    if !c.flag("ch4.opened") {
        c.say1(Sp::Ibrahim, "The Archive door is at the back of the Mint. Feed it the order you signed.").await;
        return;
    }
    if !c.flag("ch4.rogue") {
        return run_rogue(&c).await;
    }
    if !c.flag("ch4.taproot") {
        return run_taproot(&c).await;
    }
    if !c.flag("ch4.done") {
        c.set("ch4.done");
        c.chapter(5);
        c.sfx("unlock");
        say!(
            c,
            (Ibrahim, "A recovery path and an inheritance path. He set it up years ago so that if anything happened to him, it would come to you, cooperatively if possible, and alone if it had to be. The Directorate's escrow leaf? Never committed. He drafted it and threw it out."),
            (Ibrahim, "He trusted the math to carry his intent across his own death. That is the whole cypherpunk bet, really."),
            (Ibrahim, "You have the Archive. Now you need to move through the city without the Directorate cataloguing your every step. Lin, at the Masquerade, proves things without revealing them. Go learn how. The gate is open."),
        )
        .await;
        c.objective("Find Lin at the Masquerade (north-west).");
        return;
    }
    c.say1(Sp::Ibrahim, "Tell Lin I said the Fiat–Shamir heuristic is not a suggestion. She'll know.").await;
}

pub async fn archive(c: Ctx) {
    run_archive(&c).await;
}

pub async fn kiosk(c: Ctx) {
    if !c.flag("ch4.met") {
        c.say1(Sp::System, "MINT TERMINAL\nAuditor on site. Speak with Ibrahim.").await;
        return;
    }
    if !c.flag("ch4.nonce") {
        return run_nonce(&c).await;
    }
    if c.flag("ch4.opened") && !c.flag("ch4.rogue") {
        return run_rogue(&c).await;
    }
    if c.flag("ch4.rogue") && !c.flag("ch4.taproot") {
        return run_taproot(&c).await;
    }
    c.say1(Sp::System, "MINT TERMINAL\nArchive: OPEN. Audit complete.").await;
}

pub async fn vending(c: Ctx) {
    c.say1(Sp::Narrator, "The machine sells snacks for satoshis. Someone scratched into the panel: \"NEVER REUSE A NONCE.\" Below it, smaller: \"ask Warden.\"").await;
}
