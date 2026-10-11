//! Chapter 2 · Strangers agree. Felix, the Exchange, Diffie–Hellman and its limits.

use crate::puzzles::{DhBasics, Logjam, MitmRelay};
use crate::say;
use crate::script::Ctx;
use crate::speakers::Sp;

async fn run_dh(c: &Ctx) {
    if !c.puzzle(Box::new(DhBasics::new(c.world()))).await {
        c.objective("Set up a shared key with Felix (the switchboard at the Exchange).");
        return;
    }
    c.set("ch2.dh");
    c.codex(&["dh", "dlog"]);
    say!(
        c,
        (Felix, "See? We never said the key out loud. Eve heard every number and still has nothing. Diffie and Hellman, 1976. Changed everything."),
        (Felix, "Now. The Directorate runs its patrols on the same trick, through my old switchboard. But they got sloppy. They never sign their public values."),
        (Felix, "Line 7 is about to carry a patrol report. Sit on the switchboard and see what \"unauthenticated\" really costs them."),
    )
    .await;
    c.objective("Work Felix's switchboard: intercept line 7.");
}

async fn run_mitm(c: &Ctx) {
    if !c.puzzle(Box::new(MitmRelay::new(c.world()))).await {
        c.objective("Intercept the key exchange on line 7.");
        return;
    }
    c.set("ch2.mitm");
    c.codex(&["mitm"]);
    say!(
        c,
        (Felix, "There it is. You were both ends of the conversation and neither side noticed. That is the man in the middle, and plain Diffie-Hellman has no answer to it."),
        (You, "So how do you stop it?"),
        (Felix, "You make each side prove who they are. Sign the exchange. We will get to that, at the Mint. For now, one more thing bothers me."),
        (Felix, "The Directorate keeps \"downgrading links for compliance\". That means forcing them onto a weak, old group. Let me show you why that is the same as handing over the keys."),
    )
    .await;
    c.objective("Break the Directorate's downgraded link (Felix's terminal).");
}

async fn run_logjam(c: &Ctx) {
    if !c.puzzle(Box::new(Logjam::new(c.world()))).await {
        c.objective("Break the export-grade group.");
        return;
    }
    c.set("ch2.logjam");
    c.codex(&["going-dark"]);
}

pub async fn felix(c: Ctx) {
    if !c.flag("ch2.met") {
        c.set("ch2.met");
        say!(
            c,
            (Felix, "You must be nightjar's kid. He said you'd turn up when the city needed untangling. He was annoying like that. Right about everything, which made it worse."),
            (Felix, "Felix. I ran the Exchange back when it was switchboards and courage. Now it mostly routes the Directorate's surveillance, but they haven't figured out I kept a key to everything."),
            (Felix, "You want my share of the Vault key. Fine. But humour an old engineer first. Do you actually know how two people who have never met agree on a secret with the whole world listening?"),
            (Felix, "Here. Pick a number nobody else knows."),
        )
        .await;
        run_dh(&c).await;
        return;
    }
    if !c.flag("ch2.dh") {
        return run_dh(&c).await;
    }
    if !c.flag("ch2.mitm") {
        return run_mitm(&c).await;
    }
    if !c.flag("ch2.logjam") {
        return run_logjam(&c).await;
    }
    if !c.flag("ch2.done") {
        c.set("ch2.done");
        c.chapter(3);
        c.give("share2");
        c.sfx("unlock");
        say!(
            c,
            (Felix, "Thirty-one bits. Seconds. \"Export-grade\" is just a polite word for \"we can read it\". In the nineties they made everyone use weak crypto on purpose, for export. People are still being bitten by that today."),
            (Felix, "You earned this. Share two of five."),
            (Felix, "Three things you now know: strangers can agree on a secret in the open; an unsigned exchange has no defence against a relay; and a weak group is no group at all."),
            (Felix, "Go see Juno in the Commons. She splits secrets for a living. Tell her the switchboard says hello. I've opened the gate, east side."),
        )
        .await;
        c.objective("Find Juno in the Commons (east of the plaza).");
        return;
    }
    c.say1(Sp::Felix, "Give Juno my regards. And tell Mara I am NOT giving her a radio.").await;
}

pub async fn kiosk(c: Ctx) {
    if !c.flag("ch2.met") {
        c.say1(Sp::System, "EXCHANGE TERMINAL\nThe switchboard operator is right there. Go say hello.").await;
        return;
    }
    if !c.flag("ch2.dh") {
        return run_dh(&c).await;
    }
    if !c.flag("ch2.mitm") {
        return run_mitm(&c).await;
    }
    if !c.flag("ch2.logjam") {
        return run_logjam(&c).await;
    }
    c.say1(Sp::System, "EXCHANGE TERMINAL\nAll lines quiet. Felix is pretending to read a newspaper.").await;
}
