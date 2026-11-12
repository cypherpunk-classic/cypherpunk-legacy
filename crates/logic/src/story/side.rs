//! Side quests, citizens who can join you, shops, and Grandpa's floppies.

use crate::puzzles::{LedgerChain, Shop, SourceCheck, TrackingDots};
use crate::quests::FLOPPIES;
use crate::say;
use crate::script::Ctx;
use crate::speakers::Sp;

use super::missions::recruit;

/// A quest-giver who has been helped asks to sign keys once you have one.
async fn offer_signing(c: &Ctx, sp: Sp, id: &str) -> bool {
    if !c.flag("vault.opened") || c.state_ref().is_ally(id) {
        return false;
    }
    c.say1(sp, "You have a key of your own now, I hear. Will you sign mine? I've checked your fingerprint against Felix's printout; you check mine.").await;
    let fp = c.state_ref().fingerprint_of(id);
    say!(c, (Narrator, format!("Forty hex digits, read out loud, four at a time: {} …  They match. You sign, and so do they.", &fp[..20]))).await;
    recruit(c, id);
    true
}

// --- the Source (Rosa, the Wharf) ---------------------------------------------------

pub async fn rosa(c: Ctx) {
    if !c.flag("ch1.met") {
        c.say1(Sp::Rosa, "Rosa, Harbour Light. Off the record: the Directorate reads everything. On the record: no comment.").await;
        return;
    }
    if !c.flag("q.source") {
        c.set("q.source");
        say!(
            c,
            (Rosa, "You're the one Mara's been talking about. Rosa, the *Harbour Light*. I have a source inside the Directorate. They leave me encrypted drops."),
            (Rosa, "Tonight's drops don't feel right. One wants me alone at the Plaza fountain; another says the documents are fake, which contradicts everything else. My reader shows them all just fine."),
            (Nonce, "\"Shows them just fine\" is the problem. Ask her if it checks the tags."),
        )
        .await;
    }
    if !c.flag("q.source.done") {
        if c.puzzle(Box::new(SourceCheck::new(c.world()))).await {
            c.set("q.source.done");
            c.codex(&["aead", "gcm-nonce"]);
            c.state().earn(4, "Rosa's thanks");
            say!(
                c,
                (Rosa, "Two forgeries, and I'd have walked into one of them. The tag. Always the tag. I'll print a sticker for my monitor."),
            )
            .await;
        }
        return;
    }
    if offer_signing(&c, Sp::Rosa, "rosa").await {
        return;
    }
    c.say1(Sp::Rosa, "My source is quiet this week. Quiet is good. Quiet means nobody's been caught.").await;
}

// --- Yellow Dots (Theo, the Exchange) --------------------------------------------------

pub async fn theo(c: Ctx) {
    if !c.flag("ch1.done") {
        c.say1(Sp::Theo, "Records office. Closed. Everything is closed when you're on the wrong side of the gate.").await;
        return;
    }
    if !c.flag("q.dots") {
        c.set("q.dots");
        say!(
            c,
            (Theo, "Theo, records clerk. I've got a memo here that proves the Directorate's Harvest program. Printed it on the office colour laser this afternoon."),
            (Theo, "I'm going to give it to Rosa at the Harbour Light. Paper can't be hacked, right? Nobody can trace paper."),
            (Nonce, "Hold it under the blue lamp first."),
        )
        .await;
    }
    if !c.flag("q.dots.done") {
        if c.puzzle(Box::new(TrackingDots::new(c.world()))).await {
            c.set("q.dots.done");
            c.codex(&["printer-dots"]);
            c.state().earn(4, "Theo's thanks");
            c.say1(Sp::Theo, "My printer's serial number and the minute I printed it. On every page. I feel slightly sick. Thank you.").await;
        }
        return;
    }
    if offer_signing(&c, Sp::Theo, "theo").await {
        return;
    }
    c.say1(Sp::Theo, "I retype everything now. My typing has never been better.").await;
}

// --- the Ledger (Wen, the Commons) -------------------------------------------------------

pub async fn wen(c: Ctx) {
    if !c.flag("ch2.done") {
        c.say1(Sp::Wen, "The co-op's closed to outsiders this week. Directorate inspections.").await;
        return;
    }
    if !c.flag("q.ledger") {
        c.set("q.ledger");
        say!(
            c,
            (Wen, "Wen. I keep the co-op's ledger. Every delivery is a line, and every line's hash covers the one before it, so you can't change the past without it showing."),
            (Wen, "Somebody changed it anyway. Twice, I think. The Directorate wants to know where we buy from, and I think someone is putting them on the books."),
        )
        .await;
    }
    if !c.flag("q.ledger.done") {
        if c.puzzle(Box::new(LedgerChain::new(c.world()))).await {
            c.set("q.ledger.done");
            c.codex(&["timestamping", "hash"]);
            c.state().earn(4, "Wen's thanks");
            c.say1(Sp::Wen, "The radio hashes. I nearly stopped reading them out; it felt silly. I'll read them every night now, and twice on Sundays.").await;
        }
        return;
    }
    if offer_signing(&c, Sp::Wen, "wen").await {
        return;
    }
    c.say1(Sp::Wen, "Tonight's hash: four-four-one-seven… no, that was yesterday's. You see? It matters that everyone hears it.").await;
}

// --- citizens who can join you ---------------------------------------------------------------

/// A citizen's request after the Vault opens. Returns true if it handled the conversation.
pub async fn citizen(c: &Ctx, n: u32) -> bool {
    let id = format!("c{n}");
    if !c.flag("vault.opened") || c.state_ref().is_ally(&id) {
        return false;
    }
    match n {
        1 => {
            let pick = c
                .choose(
                    Sp::Narrator,
                    "Bao, the dock worker: \"Everyone's signing keys. Someone sent me a key by email saying it's yours. How would I know?\"",
                    &["Compare the fingerprint with me, face to face, or over a channel the email didn't come through.", "If it came from my address, it's mine.", "Check that the short ID matches."],
                )
                .await;
            if pick == 0 {
                c.say1(Sp::Narrator, "Bao reads you the fingerprint off his phone. You read it back. Forty characters later, he signs your key and you sign his.").await;
                recruit(c, &id);
            } else {
                c.say1(Sp::Narrator, "Bao frowns. \"Anyone can send an email from anywhere. And I heard short IDs can be faked.\" He'll think about it.").await;
            }
            true
        }
        2 => {
            if c.flag("choice.snoop") {
                c.say1(Sp::Narrator, "Ines turns away before you can say anything.").await;
                return true;
            }
            false
        }
        3 => {
            if !c.state_ref().wallet_open() {
                return false;
            }
            say!(c, (Narrator, "Kai, a student, with a stack of books: \"You're the one with the Commons coins? Buy me a coffee, the untraceable kind, and I'll sign your key. I'm testing something.\"")).await;
            let r = c.state().pay(1, "Kai's coffee");
            match r {
                Ok(_) => {
                    c.say1(Sp::Narrator, "The café takes your coin; the mint checks it; nobody learns who paid. Kai grins, writes \"1 coffee, payer unknown\" in a notebook, and signs your key.").await;
                    recruit(c, &id);
                }
                Err(_) => c.say1(Sp::Narrator, "You don't have a coin to spare. Kai shrugs: \"Next time.\"").await,
            }
            true
        }
        4 => {
            if !c.flag("ch5.stamps") {
                return false;
            }
            say!(c, (Narrator, "Odile, the gardener: \"The remailers want postage now. I've got an old laptop and no patience. Spare a stamp?\"")).await;
            let st = c.state().take_stamp();
            match st {
                Some(_) => {
                    c.say1(Sp::Narrator, "You hand over one stamp. Odile posts her note about the Directorate's pesticide contracts, and signs your key on the spot.").await;
                    recruit(c, &id);
                }
                None => c.say1(Sp::Narrator, "You have no stamps left. Odile sighs and starts her laptop hashing.").await,
            }
            true
        }
        5 => {
            if !c.flag("party.done") {
                return false;
            }
            say!(
                c,
                (Narrator, "Ravi the courier waves an ID card at you. \"Brought it this time. You were right not to sign without it.\""),
                (Narrator, "The card matches his face; his slip matches the keyserver, all forty digits. You sign."),
            )
            .await;
            recruit(c, &id);
            true
        }
        _ => false,
    }
}

// --- shops -----------------------------------------------------------------------------------

pub async fn stall(c: Ctx) {
    if !c.flag("ch3.cash") {
        say!(
            c,
            (Narrator, "A hardware stall: copper mesh, radios, old phones in bins. A hand-lettered sign: \"COMMONS COINS ONLY. NO CARDS. NO NAMES.\""),
            (Nonce, "No wallet, no shopping. The Commons Mint first."),
        )
        .await;
        return;
    }
    c.puzzle(Box::new(Shop::new(c.clone(), "stall", "HARDWARE STALL"))).await;
}

pub async fn vending(c: Ctx) {
    if !c.flag("ch3.cash") {
        c.say1(Sp::Narrator, "The machine sells snacks for Commons coins. Someone scratched into the panel: \"NEVER REUSE A NONCE.\" Below it, smaller: \"ask Warden.\"").await;
        return;
    }
    c.puzzle(Box::new(Shop::new(c.clone(), "vending", "VENDING MACHINE"))).await;
}

/// Nonce and the dried fish: once, only once.
pub async fn feed_nonce(c: &Ctx) -> bool {
    if !c.flag("bought.fish") {
        return false;
    }
    if !c.flag("nonce.fed") {
        c.set("nonce.fed");
        say!(
            c,
            (Narrator, "You offer Nonce the dried fish. It disappears with a speed that suggests practice."),
            (Nonce, "…Thank you. Once. Your grandfather fed me exactly once a day, and he never, ever fed me the same fish twice."),
            (Nonce, "It's a nonce joke. I'm named after it. Don't make that face."),
        )
        .await;
        return true;
    }
    c.say1(Sp::Nonce, "Once, I said. A number used once. A fish fed once. Some rules are load-bearing.").await;
    true
}

// --- Grandpa's floppies --------------------------------------------------------------------------

const FLOPPY_TEXTS: [(&str, &str); FLOPPIES] = [
    ("Re: chaining remailers", "One remailer is a friend who promises not to tell. Three remailers, each knowing one hop, is a friend who cannot tell. Promises are policy. Ignorance is architecture. Build ignorance."),
    ("Re: export controls (again)", "They say the code is a munition. Fine. Then the printed page is a munition, and the photocopier is an arms factory, and every student who learned modular arithmetic is a gunsmith. A law that says that is a law that is going to lose."),
    ("Re: what does the bank know", "Chaum's trick in one line: the bank signs your coin through a blindfold. It knows it signed something, once, for one unit. It never learns what. Cash had that property for three thousand years; I don't see why we should give it up for computers."),
    ("Re: the LEAF", "A backdoor has to be perfect, forever, against everyone, including the people who hold the key. Today someone showed the escrow chip fails on a 16-bit field. Next year it will be something else. The math does not care about the intentions of the key holder."),
    ("Re: postage", "Make every message cost a second of somebody's CPU. You won't notice. A man sending ten million will. Stamps that cost work instead of money; even the anonymous can afford them."),
    ("Re: why write code", "Arguments get forgotten. Laws get repealed. Code that people use gets copied, and copied, and copied. If you want something to outlive you, write it down where a compiler can read it. For E.: the cabin key is under the third stone."),
];

pub async fn floppy(c: Ctx, n: usize) {
    let flag = format!("floppy.{n}");
    if c.flag(&flag) {
        c.say1(Sp::Narrator, "Just the dust outline of where a floppy disk was.").await;
        return;
    }
    c.set(&flag);
    if !c.flag("q.floppies") {
        c.set("q.floppies");
        c.say1(Sp::Nonce, "A floppy disk. One of his. He hid six around the city, \"for whoever needs them\". I never knew where.").await;
    }
    let (subject, body) = FLOPPY_TEXTS[n - 1];
    let found = crate::quests::floppies_found(&c.state_ref());
    say!(
        c,
        (Narrator, format!("A 3½-inch floppy, label in Grandpa's handwriting: \"list archive, {n}/6\". You read it on your phone.")),
        (Grandpa, format!("*{subject}*\n\n{body}")),
    )
    .await;
    if found == FLOPPIES && !c.flag("floppy.all") {
        c.set("floppy.all");
        c.codex(&["cypherpunks-list"]);
        c.state().earn(6, "Grandpa's floppies");
        c.say1(Sp::Nonce, "All six. He'd have liked that you looked. The last line of the last one is about the cabin. I'm not telling you what's under the stone. Go and see some day.").await;
    } else {
        c.say1(Sp::Nonce, &format!("{found} of six.")).await;
    }
}
