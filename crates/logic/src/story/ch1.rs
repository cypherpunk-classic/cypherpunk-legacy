//! Chapter 1 · One time only. Mara, the radio log, the boathouse gate.

use crate::puzzles::{BitFlip, TwoTimePad};
use crate::say;
use crate::script::Ctx;
use crate::speakers::Sp;
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
    if !c.flag("ch1.done") {
        c.set("ch1.done");
        c.chapter(2);
        say!(
            c,
            (Mara, "You got in. Good. Did you notice what happened when I switched the gate to GCM?"),
            (You, "It didn't even read my forgery. The tag didn't match, so it refused."),
            (Mara, "That's the second lesson. Encryption hides things. It does not stop people from changing them. For that you need authentication, and these days you get both at once, AEAD, or you get nothing."),
            (Mara, "Keep that share safe. It's number one of five; three open the Vault."),
            (Mara, "Next you need Felix, at the Exchange. He thinks every problem is a protocol problem. He's usually right. I've opened the gate for you: west avenue."),
        )
        .await;
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
