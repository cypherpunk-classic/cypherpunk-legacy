//! Missions from cypherpunk history (v3.1), and the choices that come with
//! them: the Book (Chapter 2), the Commons Mint (Chapter 3), the key-signing
//! party (after the Vault), the escrowed phone and Warden's key (Chapter 4),
//! the remailers and Hashcash (Chapter 5).

use crate::puzzles::{BlindCash, ClipperLeaf, KeyParty, OcrBook, RemailerPuzzle, Stamps};
use crate::say;
use crate::script::Ctx;
use crate::speakers::Sp;

/// Sign someone's key and add them to the web (a toast says so).
pub fn recruit(c: &Ctx, id: &str) -> bool {
    c.state().recruit(id)
}

/// "Sign my key?" at the end of a chapter.
pub async fn sign_each_other(c: &Ctx, sp: Sp, id: &str, line: &str) {
    if !c.flag("vault.opened") || c.state_ref().is_ally(id) {
        return;
    }
    c.say1(sp, line).await;
    let fp = c.state_ref().fingerprint_of(id);
    say!(c, (Narrator, format!("You compare fingerprints, four characters at a time, out loud. {} … all forty match. You sign.", &fp[..16]))).await;
    recruit(c, id);
}

// --- Chapter 2: the Book ---------------------------------------------------------

pub async fn run_book(c: &Ctx) -> bool {
    if !c.puzzle(Box::new(OcrBook::new(c.world()))).await {
        c.objective("Fix the scanned book at Felix's press (the Exchange).");
        return false;
    }
    c.set("ch2.book");
    c.set("book.out");
    c.codex(&["pgp", "bernstein"]);
    say!(
        c,
        (Felix, "It compiles over there. A program the Directorate calls a munition, carried across the water as a book, scanned by strangers, checked line by line."),
        (Felix, "Your grandfather did that for real in ninety-five, with the PGP book. Twelve volunteers, a scanner, and a lot of coffee. The government could ban the disk. It could not ban the book."),
    )
    .await;
    c.objective("Tell Felix the book got out.");
    true
}

pub async fn press(c: Ctx) {
    if !c.flag("ch2.logjam") {
        say!(
            c,
            (Narrator, "A small offset press, still warm. A stack of blank signatures waits on the feed tray."),
            (Felix, "Not yet. Lines first, then printing."),
        )
        .await;
        return;
    }
    if !c.flag("ch2.book") {
        if !c.flag("ch2.bookIntro") {
            c.set("ch2.bookIntro");
            say!(
                c,
                (Felix, "Your grandfather's pad program. The Directorate filed it under \"munitions\": no disk with it may leave the city. So we don't send a disk."),
                (Felix, "We send a book. Every line printed in a font scanners like, with a checksum after it. The volunteers across the water have scanned it. The scanner made mistakes. Fix them."),
            )
            .await;
        }
        run_book(&c).await;
        return;
    }
    c.say1(Sp::Narrator, "The press is quiet. On the tray, a proof copy: \"otp.c\", twelve lines, a checksum after each.").await;
}

// --- Chapter 3: the Commons Mint -------------------------------------------------

pub async fn run_mint(c: &Ctx) -> bool {
    if !c.puzzle(Box::new(BlindCash::new(c.world()))).await {
        c.objective("Withdraw a coin at the Commons Mint without letting it see you.");
        return false;
    }
    c.set("ch3.cash");
    c.codex(&["ecash", "blind-signatures"]);
    let pick = c
        .choose(
            Sp::Sol,
            "One more thing. The Directorate offered us a licence. All we have to do is write the customer's name next to every coin we sign. People trust a licensed mint, they said. What would you do?",
            &["Stay blind. A list of names is a list for them.", "Take the licence. More people will use a mint they trust."],
        )
        .await;
    if pick == 1 {
        c.set("choice.mintlog");
        say!(
            c,
            (Sol, "…All right. A licence it is. The coins stay blind; it's only the withdrawals that get a name. What harm can a list do."),
            (Nonce, "Lists don't do harm. People who read them do."),
        )
        .await;
    } else {
        say!(
            c,
            (Sol, "Good. A mint that can't tell coins apart can't be made to tell on anyone. That was the whole point of Chaum's idea."),
        )
        .await;
    }
    let swapped = {
        let mut s = c.state();
        s.set("wallet.open", true);
        s.swap_ious()
    };
    c.state().earn(10, "Sol's welcome to the Commons");
    say!(
        c,
        (Sol, format!("Your wallet is open. {}Spend it anywhere that says \"eCash\": the stall on the south avenue, the vending machines. I'll never know where it went.", if swapped > 0 { format!("I swapped your {swapped} credits of IOUs for coins; IOUs carry your name, coins don't. ") } else { String::new() })),
    )
    .await;
    c.objective("Tell Juno the mint is done.");
    true
}

pub async fn sol(c: Ctx) {
    if !c.flag("ch3.feldman") {
        say!(
            c,
            (Sol, "Commons Mint. Coins nobody can trace. Juno vouches for new customers; come back when she sends you."),
        )
        .await;
        return;
    }
    if !c.flag("ch3.cash") {
        if !c.flag("ch3.solMet") {
            c.set("ch3.solMet");
            say!(
                c,
                (Sol, "Juno said you'd come. I'm Sol. I run the Commons Mint: a key per denomination, a list of spent coins, and nothing else. No accounts, no names."),
                (Sol, "Let me show you how a coin can be valid without anyone knowing whose it is. You'll do the blinding; I'll do the signing."),
            )
            .await;
        }
        run_mint(&c).await;
        return;
    }
    let ious = c.state_ref().data.ious;
    if ious > 0 {
        c.state().swap_ious();
        c.say1(Sp::Sol, &format!("Swapped {ious} credits of IOUs for blind coins. The names on the IOUs go in the fire.")).await;
        return;
    }
    if c.flag("choice.mintlog") && c.flag("ch5.done") && !c.flag("mintlog.seized") {
        mint_seizure(&c).await;
        return;
    }
    c.say1(Sp::Sol, "Coins in, coins out. I sign what I can't see and I count what I've already seen. That's all a mint should know.").await;
}

/// The cost of the licence: the Directorate takes the list.
pub async fn mint_seizure(c: &Ctx) {
    c.set("mintlog.seized");
    let lost: Vec<String> = ["c3", "c4"].iter().filter(|id| c.state_ref().is_ally(id)).map(|s| s.to_string()).collect();
    say!(
        c,
        (Sol, "They came with a warrant for the licence list. Every withdrawal with a name next to it, three months of them."),
        (Sol, if lost.is_empty() { "Nobody we know was on it, this time.".to_string() } else { "Kai and Odile were on it. They've left the city. I'm sorry.".to_string() }),
        (Nonce, "The coins were blind. The list wasn't."),
    )
    .await;
    let mut s = c.state();
    for id in &lost {
        s.lose_ally(id);
    }
}

// --- after the Vault: the key-signing party ---------------------------------------

pub async fn party(c: &Ctx) {
    if !c.flag("party.intro") {
        c.set("party.intro");
        say!(
            c,
            (Felix, "There you are. With a key of your own now, I hear. Then you're just in time: we're having a key-signing party."),
            (Felix, "After line 7 you know why. A key exchange nobody signed is an open door for a relay. Phil Zimmermann's answer in 1991 was not a company that vouches for keys. It was people who vouch for each other."),
            (Felix, "Everyone brought a slip with their fingerprint, and an ID. You check both, and you sign what you've checked. Nothing else."),
        )
        .await;
    }
    if !c.puzzle(Box::new(KeyParty::new(c.world()))).await {
        c.objective("Felix's key-signing party at the Exchange: check fingerprints and sign.");
        return;
    }
    c.codex(&["web-of-trust", "short-key-ids"]);
    for id in ["felix", "mara", "juno"] {
        recruit(c, id);
    }
    if c.flag("choice.snoop") {
        say!(
            c,
            (Narrator, "Ines, the fisher, takes back her slip before you can sign it."),
            (Narrator, "\"Mara says you read the co-op's messages that week. All of them.\" She doesn't wait for an answer."),
        )
        .await;
    } else {
        recruit(c, "c2");
    }
    c.give("keyring");
    c.set("party.done");
    c.sfx("unlock");
    say!(
        c,
        (Felix, "And that's a web. Small, but every strand of it is someone who looked another person in the eye. Open the NETWORK page in your menu: that's yours now. Decide how far you trust each of us to check others."),
        (Felix, "Ravi will be back with his ID. Ask him again; I signed him years ago, which may be enough for you."),
        (Felix, "Now go. Ibrahim's at the Mint, north-east. I've opened that gate."),
    )
    .await;
    c.objective("Find Ibrahim at the Mint (north-east). The Archive is sealed there.");
}

// --- Chapter 4: Warden's key, the escrowed phone ------------------------------------

pub async fn warden_key_choice(c: &Ctx) {
    let pick = c
        .choose(
            Sp::Ibrahim,
            "Before you use it: what do you do with Warden's key afterwards?",
            &[
                "Use it once to open the Archive, then tell the world it's compromised so it gets revoked.",
                "Publish it. Let everyone sign as Warden.",
                "Keep it quiet. A key like that is leverage.",
            ],
        )
        .await;
    let (choice, line) = match pick {
        1 => ("publish", "Then every order in the city becomes worthless, his and everyone's who relies on them. Including the people who can least afford chaos. Your call."),
        2 => ("keep", "Then you'd be holding exactly what you came here to take away from him: a key nobody knows you have. Think about who that makes you."),
        _ => ("disclose", "Good. Use it for the one thing it should never have been able to do, then burn it in public. That's how a compromised key is supposed to end."),
    };
    c.state().set("choice.key", choice);
    c.say1(Sp::Ibrahim, line).await;
}

pub async fn run_clipper(c: &Ctx) -> bool {
    if !c.puzzle(Box::new(ClipperLeaf::new(c.world()))).await {
        c.objective("Beat the escrow on Ibrahim's phone (the payphone by the Mint).");
        return false;
    }
    c.set("ch4.clipper");
    c.codex(&["clipper"]);
    say!(
        c,
        (Ibrahim, "Sixteen bits. They built a whole national wiretap on sixteen bits, in our city as in the real one. Matt Blaze found it in 1994 and the Clipper chip never recovered."),
        (Ibrahim, "And look outside: the escrow taps on the avenues are dark. They listen for LEAFs; ours now say nothing they can use."),
    )
    .await;
    c.objective("Tell Ibrahim the phone is clean.");
    true
}

pub async fn phone(c: Ctx) {
    if !c.flag("ch4.taproot") {
        say!(
            c,
            (Narrator, "A Directorate \"Lawful Access\" payphone. A sticker: ALL CALLS ESCROWED FOR YOUR SAFETY."),
            (Nonce, "Ibrahim will want to talk about this one. Later."),
        )
        .await;
        return;
    }
    if !c.flag("ch4.clipper") {
        if !c.flag("ch4.phoneIntro") {
            c.set("ch4.phoneIntro");
            say!(
                c,
                (Ibrahim, "This is the phone they make us use. Every call carries a field the escrow office can open: the call's own key, wrapped for them. They call it lawful access."),
                (Ibrahim, "Your grandfather had a word for it: a skeleton key with a ribbon on it. Let me show you the ribbon is made of sixteen bits."),
            )
            .await;
        }
        run_clipper(&c).await;
        return;
    }
    c.say1(Sp::Narrator, "The payphone still carries its sticker. Its LEAFs now decrypt, at the escrow office, into noise.").await;
}

// --- Chapter 5: the remailers, Hashcash -----------------------------------------

pub async fn run_remail(c: &Ctx) -> bool {
    if !c.puzzle(Box::new(RemailerPuzzle::new(c.clone()))).await {
        c.objective("Answer Kestrel through the remailers (Lin's rack at the Masquerade).");
        return false;
    }
    c.set("ch5.remail");
    c.codex(&["remailers", "mixing"]);
    let pick = c
        .choose(
            Sp::Lin,
            "Your answer got through. Now your own node: run it like anon.penet.fi, with pseudonyms and a table that maps them back so people can get replies? Or like Mixmaster: no table, no logs, no replies?",
            &["No table. Nothing to hand over.", "Keep the table. Replies matter to people."],
        )
        .await;
    if pick == 1 {
        c.state().set("choice.logs", true);
        say!(
            c,
            (Lin, "Penet ran for three years like that. Half a million users. Then a court asked for one name, and he had it."),
            (Lin, "I hope nobody ever asks you."),
        )
        .await;
    } else {
        c.say1(Sp::Lin, "No table. Then the only answer you can ever give is the true one: \"I don't know.\"").await;
    }
    c.give("onion-route");
    c.objective("Price the remailers' postage (the notice board at the Masquerade).");
    true
}

pub async fn remailer(c: Ctx) {
    if !c.flag("ch5.extract") {
        say!(
            c,
            (Narrator, "A rack of quiet machines, cables taped down neatly. A card on top: \"MIXMASTER · 4 NODES · BATCH 8\"."),
            (Nonce, "Lin's remailers. She'll want you to understand proofs first."),
        )
        .await;
        return;
    }
    if !c.flag("ch5.remail") {
        if !c.flag("ch5.remailIntro") {
            c.set("ch5.remailIntro");
            say!(
                c,
                (Lin, "A message came in last night, through my rack. Someone inside the Directorate who calls themselves Kestrel. They have the Archive's missing volume."),
                (Lin, "Two keys on the keyserver claim to be Kestrel's. Answer the real one, through three remailers, so nobody learns who's talking to whom. Not even me."),
            )
            .await;
        }
        run_remail(&c).await;
        return;
    }
    c.say1(Sp::Narrator, "The rack hums. Every few seconds it swallows a batch of eight and spits out eight, in a different order.").await;
}

pub async fn run_stamps(c: &Ctx) -> bool {
    if !c.puzzle(Box::new(Stamps::new(c.clone()))).await {
        c.objective("Set the remailers' postage at the notice board.");
        return false;
    }
    c.set("ch5.stamps");
    c.codex(&["hashcash"]);
    c.say1(Sp::Lin, "Postage. Cheap for one person, ruinous for a flood. Keep those stamps: the broadcast will need them.").await;
    c.objective("Tell Lin the remailers are holding.");
    true
}

pub async fn board(c: Ctx) {
    if !c.flag("ch5.remail") {
        c.say1(Sp::Narrator, "A cork board covered in notes: offers, warnings, a lost cat that is definitely Nonce. Half the notes are spam from \"CONCERNED CITIZENS FOR LAWFUL ACCESS\".").await;
        return;
    }
    if !c.flag("ch5.stamps") {
        if !c.flag("ch5.boardIntro") {
            c.set("ch5.boardIntro");
            say!(
                c,
                (Lin, "The Directorate found a cheaper attack than breaking anything: flooding. Fill every batch with your own junk, and the one real message stands out. They're doing it to the remailers now."),
                (Lin, "So we charge postage. Not money. Work."),
            )
            .await;
        }
        run_stamps(&c).await;
        return;
    }
    let n = c.state_ref().data.stamps.len();
    if n == 0 {
        c.say1(Sp::Lin, "Out of stamps? The board will sell you postage for work, as often as you like.").await;
        run_stamps(&c).await;
        return;
    }
    c.say1(Sp::Narrator, &format!("The junk has thinned out. You hold {n} stamp(s).")).await;
}
