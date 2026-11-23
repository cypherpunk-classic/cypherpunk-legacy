//! Chapter 1 · One time only. Mara, the radio log, the boathouse gate, and
//! then the harbour itself: the Wharf district, where every lock is a lamp
//! board (`story::wharf`). Freeing its mast ends the chapter.

use crate::puzzles::{BitFlip, TwoTimePad};
use crate::say;
use crate::script::Ctx;
use crate::signals::wharf::END as HARBOUR_DONE;
use crate::speakers::Sp;
use crate::state::MapId;
use crate::world::{CH1_P1, CH1_P2};

async fn run_pads(c: &Ctx) {
    if !c.puzzle(Box::new(TwoTimePad::new(c.world()))).await {
        c.objective("Decode the two messages in Mara's radio log (use her radio or the kiosk).");
        return;
    }
    c.set("ch1.otp");
    c.codex(&["otp", "two-time-pad", "venona"]);
    say!(
        c,
        (Mara, format!("\"{CH1_P1}\"")),
        (Mara, format!("\"{CH1_P2}\"")),
        (Mara, "Hah. We sent those the week the Directorate moved in. Same pad page, both ways. Tired eyes, cold hands, one mistake."),
        (Mara, "The Soviets made that exact mistake in the forties, and it took the Americans forty years of patience to read what they had. You did it in ten minutes. Patience is cheaper now."),
    )
    .await;
    let pick = c
        .choose(
            Sp::Mara,
            "The Directorate's intercept log has the whole week. The fishing co-op's traffic went through the same lazy relay, on the same reused pages. Want to read theirs too, while we're at it?",
            &["Only ours, and the Directorate's. The rest is nobody's business.", "Read everything. You never know what's useful."],
        )
        .await;
    if pick == 1 {
        c.set("choice.snoop");
        say!(
            c,
            (Mara, "…That's how it starts, you know. Just this once. Just the useful parts. Warden says the same thing on every screen in the city."),
            (Mara, "Read them if you like. I won't help, and the co-op will hear about it."),
        )
        .await;
    } else {
        c.say1(Sp::Mara, "Good answer. Privacy for the weak, transparency for the powerful. That one went round the list in the nineties, and it's still the whole job.").await;
    }
    say!(
        c,
        (Mara, "Split five ways. Yes. My share of the Vault key is in the lockbox at the boathouse. East end of the Wharf. The Directorate bolted one of their ticket gates onto the door."),
        (Mara, "Your ticket says \"guest\". See what you can do about that."),
    )
    .await;
    c.objective("Get into the boathouse lockbox (ticket gate at the east end of the Wharf).");
}

async fn run_gate(c: &Ctx) {
    if !c.puzzle(Box::new(BitFlip::new(c.world()))).await {
        c.objective("Get past the boathouse ticket gate.");
        return;
    }
    c.set("ch1.ctr");
    c.codex(&["malleability", "aead"]);
    c.sfx("door");
    say!(
        c,
        (Narrator, "The lockbox clicks open. Inside, sealed in plastic: a card with two numbers on it, x = 1 and a long hex string."),
        (Nonce, "A Shamir share. One of five. Bring it to Mara."),
    )
    .await;
    c.give("share1");
    c.objective("Bring the share back to Mara.");
}

/// Down to the harbour: the Wharf district, where the Directorate's lamp boards are.
async fn to_harbour(c: &Ctx) {
    if !c.flag(HARBOUR_SENT) {
        c.set(HARBOUR_SENT);
        say!(
            c,
            (Mara, "You got in. Good. Did you notice what happened when I switched the gate to GCM?"),
            (You, "It didn't even read my forgery. The tag didn't match, so it refused."),
            (Mara, "That's the second lesson. Encryption hides things. It does not stop people from changing them."),
            (Mara, "Now the hard part. That was one gate. Down on the harbour proper, every lock, every patrol, the camera and the big mast all take their orders from lamp boards: pictures under a one-time pad, sixty-four lamps each."),
            (Mara, "Your grandfather left a card in the pier locker for whoever came after him. A card flips the lamps behind its holes. That is all it does, and it is enough."),
            (Mara, "Free the mast and the harbour is ours. I'll walk you down as far as the piers. When it's done, the market's north gate brings you back up to the radio shack."),
        )
        .await;
    } else {
        c.say1(Sp::Mara, "Back down to the harbour, then. The mast is still theirs.").await;
    }
    c.objective("Down on the harbour every lock is a lamp board. Make one say something it was never told. (M: map)");
    c.goto(MapId::Wharf, Some("start")).await;
}

pub async fn mara(c: Ctx) {
    if !c.flag("ch1.met") {
        c.set("ch1.met");
        say!(
            c,
            (Mara, "Well, well. You have his chin. And his cat, I see."),
            (Nonce, "He has *my* human, technically."),
            (Mara, "I'm Mara. Your grandfather and I ran radio together for thirty years: him in the cabin, me on the boats. One-time pads, numbers stations, the whole romantic business."),
            (Mara, "A pad is perfect. Truly random key, as long as the message, used exactly once, burned after. Shannon proved nothing beats it. The catch is in the word *once*."),
            (Mara, "Last spring the Directorate pulled two of our messages out of the air. I think we used the same page twice. If so, they can read them, given time. I would rather we read them first."),
            (Mara, "Here. My radio log. Have a look."),
        )
        .await;
        run_pads(&c).await;
        return;
    }
    if !c.flag("ch1.otp") {
        return run_pads(&c).await;
    }
    if !c.flag("ch1.ctr") {
        say!(
            c,
            (Mara, "The boathouse is at the east end. The gate takes encrypted tickets. Old firmware: counter mode, no tag."),
            (Mara, "You know your own ticket says \"role=guest\". You know where those letters sit. Think about what XOR does to a known plaintext."),
        )
        .await;
        return;
    }
    if !c.flag(HARBOUR_DONE) {
        return to_harbour(&c).await;
    }
    if !c.flag("ch1.done") {
        c.set("ch1.done");
        c.chapter(2);
        say!(
            c,
            (Mara, "Every board on the harbour saying his sign. I heard it from up here. Their pads were perfect, and you never broke one. You broke how they used them."),
            (Mara, "That's the whole of chapter one, really. Encryption hides things. It does not stop people from changing them, replaying them, or using the same page twice. For that you need authentication, and these days you get both at once, AEAD, or you get nothing."),
            (Mara, "Keep that share safe. It's number one of five; three open the Vault."),
            (Mara, "And take this. Your grandfather's messenger: AES-GCM, a fresh nonce for every message, and it checks the tag before it shows you a word. The Directorate's masts log less of anyone who uses it."),
            (Mara, "Next you need Felix, at the Exchange. He thinks every problem is a protocol problem. He's usually right. I've opened the gate for you: west avenue."),
        )
        .await;
        c.give("sealed-messenger");
        c.codex(&["metadata"]);
        c.sfx("unlock");
        c.objective("Find Felix at the Exchange (west avenue, through the gate).");
        return;
    }
    c.say1(Sp::Mara, "Go on, Felix is waiting. And tell him he still owes me a radio.").await;
}

pub async fn radio(c: Ctx) {
    if !c.flag("ch1.met") {
        c.say1(Sp::Narrator, "An old shortwave radio. A handwritten label: \"MARA'S. DO NOT TOUCH. (YES, YOU.)\"").await;
        return;
    }
    if !c.flag("ch1.otp") {
        return run_pads(&c).await;
    }
    c.say1(Sp::Narrator, "The radio crackles: numbers in a flat voice, the same five digits over and over. Somebody somewhere is still using a pad.").await;
}

pub async fn kiosk(c: Ctx) {
    if !c.flag("ch1.met") {
        c.say1(Sp::System, "PUBLIC TERMINAL · THE WHARF\nNo session. Talk to the harbourmaster.").await;
        return;
    }
    if !c.flag("ch1.otp") {
        return run_pads(&c).await;
    }
    if !c.flag("ch1.ctr") {
        return run_gate(&c).await;
    }
    c.say1(Sp::System, "PUBLIC TERMINAL · THE WHARF\nAll tasks complete. Have a nice night.").await;
}

pub async fn boathouse(c: Ctx) {
    if !c.flag("ch1.otp") {
        say!(
            c,
            (Narrator, "A Directorate ticket gate is bolted to the boathouse door. It wants a ticket."),
            (Nonce, "Mara will explain. She explains everything, eventually."),
        )
        .await;
        return;
    }
    if !c.flag("ch1.ctr") {
        return run_gate(&c).await;
    }
    c.say1(Sp::Narrator, "The lockbox stands open and empty. The gate now runs GCM. Mara looks smug about it.").await;
}

/// Set when Mara has sent the player down to the harbour.
pub const HARBOUR_SENT: &str = "ch1.harbour";

