//! Prologue: the cabin, the letter, the coffee ring, the gateway.

use crate::puzzles::{GatewayLogin, SeedRecovery};
use crate::say;
use crate::script::Ctx;
use crate::speakers::Sp;
use crate::state::MapId;

pub async fn cabin_ready(c: Ctx) {
    if c.flag("pro.intro") {
        return;
    }
    c.set("pro.intro");
    // set before the intro, so quitting halfway still leaves a goal
    c.objective("Look around Grandpa's cabin.");
    c.chapter(0);
    c.cutscene(async {
        c.wait(500.0).await;
        say!(
            c,
            (Narrator, "Rain on a tin roof. The smell of old paper, woodsmoke and machine oil."),
            (Narrator, "The lawyer said the cabin was yours now, \"along with the contents and the cat\". He said it like the cat was the problem."),
            (You, "Grandpa's place. He never let anyone touch the computer."),
            (You, "Well. He isn't here to stop me."),
        )
        .await;
    })
    .await;
}

fn letter_lines(c: &Ctx) -> Vec<(Sp, String)> {
    let w = c.world();
    let g = &w.grandpa().mnemonic;
    let shown: Vec<String> = g
        .words
        .iter()
        .enumerate()
        .map(|(i, word)| if i == g.missing_index { format!("**{}▒▒▒▒** *(coffee ring)*", &word[..1]) } else { format!("`{word}`") })
        .collect();
    let row = |from: usize| shown[from..from + 6].iter().enumerate().map(|(i, w)| format!("{}. {}", from + i + 1, w)).collect::<Vec<_>>().join("   ");
    vec![
        (Sp::Letter, "My dear {{handle}},\n\nIf you are reading this, the lawyer kept his promise and I have run out of time. Don't be sad for long. I had a good run and a better argument.".into()),
        (Sp::Letter, "I left you something in Cryptopolis, the city my friends and I built when the internet was young and we thought we could keep it free. To enter, you will need my key. Here it is, the way we wrote keys down: twelve words.".into()),
        (Sp::Letter, format!("{}\n{}", row(0), row(6))),
        (Sp::Letter, "Never type them into anything you don't trust, never photograph them, and never, ever tell the Directorate. Trust, but verify.\n\nYour loving Grandpa\n\nP.S. Feed Nonce once a day. Only once. She will lie about this.".into()),
    ]
}

pub async fn dresser(c: Ctx) {
    if c.flag("pro.letter") {
        c.say1(Sp::Narrator, "The drawer is empty except for a pencil stub and a receipt for floppy disks, dated 1994.").await;
        return;
    }
    c.sfx("door");
    c.say1(Sp::Narrator, "The top drawer sticks, then gives. Inside: an envelope with your name on it in blue ink. Someone set a coffee mug on it, more than once.").await;
    c.set("pro.letter");
    c.give("letter");
    c.say(letter_lines(&c)).await;
    let (n, letter) = {
        let w = c.world();
        let g = &w.grandpa().mnemonic;
        (g.missing_index + 1, g.first_letter)
    };
    c.say1(Sp::You, &format!("Twelve words, and the coffee ate one of them. Word {n} starts with \"{letter}\". That's all that's left.")).await;
    c.codex(&["bip39"]);
    c.objective("Take the letter to Grandpa's computer.");
}

pub async fn computer(c: Ctx) {
    if c.flag("pro.done") {
        let go = c.choose(Sp::System, "CRYPTOPOLIS GATEWAY v3.1 — session ready.", &["Enter Cryptopolis", "Not yet"]).await;
        if go == 0 {
            enter_city(&c).await;
        }
        return;
    }
    if !c.flag("pro.letter") {
        say!(
            c,
            (Narrator, "A beige CRT, older than you. A yellow sticky note clings to the bezel."),
            (Narrator, "You press the power button. The screen hums: ENTER RECOVERY PHRASE (12 WORDS)."),
            (You, "Twelve words. I don't have twelve words. He'd have written them down somewhere close by — the drawers, maybe. I should search the room first."),
        )
        .await;
        c.objective("Search the cabin for the twelve words (try the chest of drawers).");
        return;
    }
    if !c.flag("pro.note") {
        let descriptor = c.world().grandpa().descriptor.clone();
        say!(
            c,
            (Narrator, "The sticky note is covered in Grandpa's tiny capitals:"),
            (System, descriptor),
            (You, "tr(...)... a wallet descriptor. Taproot, path 86h/0h/0h. And that bracketed number in front is underlined twice."),
            (You, "The master fingerprint. If I get the seed right, the fingerprint will match. If not, it won't."),
        )
        .await;
        c.set("pro.note");
        c.give("descriptor");
        c.codex(&["bip32"]);
    }
    if !c.flag("pro.booted") {
        c.sfx("power");
        c.set("pro.booted");
        c.wait(400.0).await;
        c.say1(Sp::System, "CRYPTOPOLIS GATEWAY v3.1\nnode: cabin-03 · link: up · user: nightjar\nENTER RECOVERY PHRASE (12 WORDS)").await;
    }
    if !c.flag("pro.seed") {
        if !c.puzzle(Box::new(SeedRecovery::new(c.world()))).await {
            c.objective("Recover the missing word of the seed phrase.");
            return;
        }
        c.set("pro.seed");
        c.give("gateway-key");
        say!(
            c,
            (You, "Twelve words, and the fingerprint matches. That's his seed."),
            (System, "SEED ACCEPTED. Deriving identity key m/86'/0'/0'/0/0 … done.\nREQUESTING CHALLENGE FROM GATEWAY …"),
        )
        .await;
    }
    if !c.flag("pro.login") {
        if !c.puzzle(Box::new(GatewayLogin::new(c.world()))).await {
            c.objective("Sign the gateway challenge.");
            return;
        }
        c.set("pro.login");
        c.codex(&["schnorr", "hash"]);
    }
    c.set("pro.done");
    c.chapter(1);
    say!(
        c,
        (System, "WELCOME BACK, NIGHTJAR.\nLast login: 391 days ago."),
        (Narrator, "Something brushes against your leg. Nonce is sitting on the desk, staring at the screen like she has been waiting for this for a year."),
        (Nonce, "Mrrp."),
    )
    .await;
    c.set("pro.metCat");
    enter_city(&c).await;
}

pub async fn bookshelf(c: Ctx) {
    say!(
        c,
        (Narrator, "Paperbacks about codes and codebreakers, a cryptography textbook with a cracked spine, and a fat binder labelled PGP SOURCE, PRINTED — DO NOT EXPORT (HA)."),
        (You, "He printed source code so it could leave the country as a *book*. That was a thing people had to do?"),
    )
    .await;
    c.codex(&["crypto-wars", "pgp"]);
}

pub async fn photo(c: Ctx) {
    say!(
        c,
        (Narrator, "A faded photo: a dozen people around a folding table covered in laptops and pizza boxes. Grandpa is the one in the cardigan, grinning."),
        (Narrator, "On the back, in pencil: \"Bay Area, 1993. nightjar, mara, felix, juno, rook, ibrahim, lin, ada. Cypherpunks write code.\""),
    )
    .await;
    c.codex(&["cypherpunks-list", "manifesto"]);
}

pub async fn fireplace(c: Ctx) {
    c.say1(Sp::Narrator, "The fire has burned low but steady. Someone, the lawyer maybe, laid it fresh this morning. A small brass key hangs from a nail on the mantel: \"SPARE. NOT A SECRET.\"").await;
}

pub async fn armchair(c: Ctx) {
    c.say1(Sp::Narrator, "The armchair is shaped exactly like Grandpa. You decide not to sit in it yet.").await;
}

pub async fn bed(c: Ctx) {
    c.say1(Sp::Narrator, "A patchwork quilt. Under the pillow: a paperback on number theory, bookmarked at a chapter called \"Primes Are Not Rare.\"").await;
}

pub async fn plant(c: Ctx) {
    c.say1(Sp::Narrator, "A very determined fern. The soil is still damp. Somebody has been watering it.").await;
}

pub async fn coat(c: Ctx) {
    c.say1(Sp::Narrator, "A blue jacket and a red scarf. In the pocket: a ticket stub, \"CRYPTO '93\", and a mint that has seen things.").await;
}

pub async fn boxes(c: Ctx) {
    say!(
        c,
        (Narrator, "Moving boxes, half packed. One is labelled PGP in marker. Inside: floppy disks, a modem, and a stack of mailing-list printouts with a rubber band around them."),
        (Narrator, "The top page: \"Subject: Re: export controls are a munitions ban on mathematics.\""),
    )
    .await;
    c.codex(&["bernstein"]);
}

pub async fn window(c: Ctx) {
    c.say1(Sp::Narrator, "Rain, pine trees, nothing else for miles. The cabin's only line to the outside world is a cable that runs from the roof into the dark.").await;
}

pub async fn door(c: Ctx) {
    if c.flag("pro.done") {
        c.say1(Sp::You, "The rain hasn't let up. Cryptopolis is warmer anyway. The computer is right there.").await;
    } else {
        c.say1(Sp::You, "Not in this rain. And not before I find out what he left me.").await;
    }
}

pub async fn enter_city(c: &Ctx) {
    c.save_position();
    c.set("pro.toCity");
    c.goto(MapId::City, Some("gateway")).await;
}

pub async fn pet_nonce_cabin(c: Ctx) {
    if !c.flag("pro.metCat") {
        c.set("pro.metCat");
        say!(
            c,
            (Narrator, "A grey cat with a pink collar regards you from the rug. The tag on the collar says NONCE. On the back, engraved: USE ONCE."),
            (Nonce, "Mrrrow."),
            (You, "Grandpa named his cat after a number you're only supposed to use once. Of course he did."),
        )
        .await;
        c.sfx("meow");
        return;
    }
    c.sfx("meow");
    let lines = [
        "Nonce headbutts your hand, then looks pointedly at her empty bowl.",
        "Nonce purrs. She has definitely already eaten. She will tell you otherwise.",
        "Nonce stares at the computer, then at you, then at the computer.",
    ];
    let i = (crate::state::now_ms() as u64 % 3) as usize;
    c.say1(Sp::Narrator, lines[i]).await;
}
