//! Cryptopolis in general: arrival, Nonce the daemon, zones, citizens, gates.

use crate::say;
use crate::script::Ctx;
use crate::speakers::Sp;
use crate::state::MapId;

pub async fn city_ready(c: Ctx) {
    if c.flag("city.arrived") {
        return;
    }
    c.set("city.arrived");
    c.cutscene(async {
        c.flash(400.0, 0x73eff7);
        c.sfx("glitch");
        c.wait(700.0).await;
        say!(
            c,
            (Narrator, "The cabin dissolves into scanlines and re-forms as a street at night. Rain-slick asphalt, neon reflected in black water, the hum of a city that is mostly made of numbers."),
            (Nonce, "Took you long enough."),
            (You, "…the cat talks."),
            (Nonce, "In here I'm not a cat. I'm a daemon wearing a cat. Your grandfather thought that was funny. He was right."),
            (Nonce, "Welcome to Cryptopolis. He and his friends built it in the nineties: a place where people could talk without being overheard. It's smaller now. The Directorate has been *regulating* it."),
        )
        .await;
        c.sfx("warden");
        say!(
            c,
            (Warden, "*(broadcast on every screen)* Citizens of Cryptopolis. This is Warden, Directorate of Lawful Access."),
            (Warden, "Under the Lawful Access Act, all private keys are to be escrowed with the Directorate by the end of the month. Encryption without escrow is obstruction."),
            (Warden, "Those with nothing to hide have nothing to fear. Your keys will be safe with us."),
            (Nonce, "They always say that part."),
            (Nonce, "Your grandfather left you something in the Vault on Legacy Plaza. It is locked with a key split into five shares; any three open it. His friends hold some of them."),
            (Nonce, "Start with Mara. She's by the radio shack at the west end of the Wharf. Follow the antenna."),
        )
        .await;
    })
    .await;
    c.set("warden.on");
    c.codex(&["going-dark"]);
    c.objective("Find Mara at the radio shack on the Wharf.");
    c.chapter(1);
}

pub fn zone_title(id: &str) -> Option<(&'static str, &'static str)> {
    Some(match id {
        "wharf" => ("THE WHARF", "Chapter 1 · One time only"),
        "exchange" => ("THE EXCHANGE", "Chapter 2 · Strangers agree"),
        "commons" => ("THE COMMONS", "Chapter 3 · Threshold"),
        "plaza" => ("LEGACY PLAZA", "The Vault"),
        "mint" => ("THE MINT", "Chapter 4 · Signatures"),
        "masks" => ("THE MASQUERADE", "Chapter 5 · Knowledge"),
        "observatory" => ("THE OBSERVATORY", "Chapter 6 · After the quantum"),
        _ => return None,
    })
}

pub async fn city_zone(c: Ctx, id: String) {
    if let Some((name, sub)) = zone_title(&id) {
        c.location(name, sub);
    }
}

pub fn hint(c: &Ctx) -> &'static str {
    let f = |k: &str| c.flag(k);
    if f("ch1.ctr") && !f("ch1.done") {
        return if f(crate::signals::wharf::END) {
            "Mara, at the radio shack. The harbour is free; she will want to hear it."
        } else {
            "Mara will send you down to the harbour. Every lock down there is a lamp board, and a card flips the lamps behind its holes."
        };
    }
    if !f("ch1.done") {
        return if f("ch1.met") {
            "Mara's terminal is the kiosk by her radio. The boathouse gate is at the east end of the Wharf."
        } else {
            "Mara is by the radio shack at the west end of the Wharf. Look for the antenna."
        };
    }
    if !f("ch2.logjam") {
        return "The Exchange gate is open now: west avenue, through the fence. Felix runs the switchboard.";
    }
    if !f("ch2.book") {
        return "Felix's press, in the Exchange. Grandpa's code is on paper; the scan needs fixing before it can be printed.";
    }
    if !f(crate::lines::exchange::END) {
        return "Back to Felix. The book is out, and he has one more job: the Exchange's own street, where every order comes down a wire.";
    }
    if !f("ch2.done") {
        return "Felix, at the switchboard. The lines are free; he owes you a share.";
    }
    if !f("ch3.feldman") {
        return "The Commons, east of the plaza. Juno is in the park. Shares hide in dead drops: a loose brick, a hollow tree.";
    }
    if !f("ch3.cash") {
        return "The Commons Mint, in the park. Sol signs coins without ever seeing them. You will want a wallet.";
    }
    if !f("ch3.done") {
        return "Juno again. She has the last piece of the Vault puzzle.";
    }
    if !f("vault.opened") {
        return "The Vault. Legacy Plaza. Now. I have waited a year for this.";
    }
    if !f("party.done") {
        return "Felix is throwing a key-signing party at the Exchange. Check every fingerprint, and every ID.";
    }
    if !f("ch4.nonce") {
        return "Ibrahim is at the Mint, north-east. Warden sealed the Archive door there.";
    }
    if !f("ch4.rogue") {
        return "The Archive door at the Mint. \"Joint custody\", the Directorate calls it.";
    }
    if !f("ch4.taproot") {
        return "Ibrahim again. He found something in Grandpa's Vault address.";
    }
    if !f("ch4.clipper") {
        return "The payphone by the Mint. The Directorate's escrow chip sits in it, sixteen bits of checksum and all.";
    }
    if !f("ch4.done") {
        return "Ibrahim, at the Mint. The Archive is waiting.";
    }
    if !f("ch5.remail") {
        return "The Masquerade, north-west. Lin's remailer is the server in the corner. Kestrel needs an answer that cannot be traced.";
    }
    if !f("ch5.stamps") {
        return "The notice board in the Masquerade. Postage, paid in hashes.";
    }
    if !f("ch5.done") {
        return "Lin will teach you to prove things without telling them.";
    }
    if !f("ch6.done") {
        return "Ada is at the Observatory, north of the plaza. Dress warmly; the future is cold.";
    }
    if f("ch5.stampsAgain") && c.state_ref().data.stamps.is_empty() {
        return "The broadcast needs postage. Mint stamps at the Masquerade notice board, then the Vault.";
    }
    if !f("end.done") {
        return "Back to the Vault. Time to finish what he started.";
    }
    "You did it. I am going to sleep on a warm server rack for a week."
}

pub async fn pet_nonce_city(c: Ctx) {
    c.sfx("meow");
    let pick = c.choose(Sp::Nonce, "Mrrp?", &["Where should I go?", "What are you, really?", "Good cat."]).await;
    match pick {
        0 => c.say1(Sp::Nonce, hint(&c)).await,
        1 => {
            say!(
                c,
                (Nonce, "A small program your grandfather wrote to look after the city while he was away. Mostly I keep logs and judge people."),
                (Nonce, "He gave me the cat's face so that I would not feel like a program. It worked. Please do not tell the cat."),
            )
            .await
        }
        _ => c.say1(Sp::Nonce, "I know.").await,
    }
}

const CITIZEN_LINES: [(&str, &str); 5] = [
    ("A dockworker shrugs.", "\"Escrow my keys? With the people who lost the last escrow database? Sure. Right after they find it.\""),
    ("A woman with a laptop sticker that says \"NO\".", "\"My grandmother used PGP to write to her sister during a war. People forget why this matters until they need it.\""),
    ("A kid in a hoodie.", "\"Did you know you can break a one-time pad if somebody uses it twice? My teacher says the Soviets did. Twice.\""),
    ("A park regular, feeding pixelated pigeons.", "\"Shares are like pigeons. Three of them come back, you have got a secret. Two of them, you have got nothing. Even if you think you have.\""),
    ("A courier, checking a delivery tablet.", "\"If the Directorate can read every message, so can whoever breaks into the Directorate. That is just arithmetic.\""),
];

pub async fn citizen(c: Ctx, n: usize) {
    let (a, b) = CITIZEN_LINES[n - 1];
    say!(c, (Narrator, a), (Narrator, b)).await;
    if n == 3 {
        c.codex(&["venona"]);
    }
}

pub async fn gate(c: Ctx, which: &'static str) {
    let (a, b) = match which {
        "exchange" => ("A shimmering barrier: DIRECTORATE OF LAWFUL ACCESS — RESTRICTED ZONE.", "Mara can get this open. Help her first."),
        "commons" => ("The barrier crackles. A placard: CLOSED FOR YOUR SAFETY.", "Felix has the override for the Commons. One thing at a time."),
        "mint" => ("The Mint is sealed: NOTICE OF LAWFUL SEIZURE.", if c.flag("vault.opened") { "Felix's key-signing party first. The people inside only open the door for keys they can check." } else { "Open the Vault first. Your grandfather wrote down what to do about this." }),
        "masks" => ("The barrier pulses in time with a checkpoint scanner beyond it.", "Finish at the Mint. Ibrahim will know who can let you in."),
        _ => ("Beyond the barrier, the Observatory dome is open to the sky.", "Ada only takes visitors who can pass a checkpoint without showing their papers. Lin first."),
    };
    say!(c, (Narrator, a), (Nonce, b)).await;
}

pub async fn gatepad(c: Ctx) {
    let pick = c.choose(Sp::System, "GATEWAY PAD — link to cabin-03", &["Return to the cabin", "Stay in Cryptopolis"]).await;
    if pick == 0 {
        c.goto(MapId::Cabin, Some("computer")).await;
    }
}

pub async fn billboard(c: Ctx) {
    say!(
        c,
        (Narrator, "LAWFUL ACCESS KEEPS YOU SAFE. In smaller print: \"Key escrow is mandatory for all citizens. Keys are stored securely in a single, highly secure location.\""),
        (You, "A single location. What could go wrong."),
    )
    .await;
    c.codex(&["clipper"]);
}

pub async fn bench(c: Ctx) {
    c.say1(Sp::Narrator, "You sit for a while. Rain drums on the neon. Somewhere a modem sings a song from 1996.").await;
}

pub async fn statue(c: Ctx) {
    say!(
        c,
        (Narrator, "A statue holding a key over its head. The plaque reads: \"TO EVERYONE WHO WROTE THE CODE INSTEAD OF ASKING PERMISSION.\""),
        (Narrator, "Below, smaller, scratched in by hand: \"and to everyone who read it and found the bugs.\""),
    )
    .await;
    c.codex(&["manifesto"]);
}

pub async fn street_vending(c: Ctx) {
    c.say1(Sp::Narrator, "The vending machine offers PADS (single use), SALT (per user), and PEPPER (keep it secret). You are out of coins.").await;
}

pub async fn stall(c: Ctx) {
    c.say1(Sp::Narrator, "A stall selling second-hand hardware: modems, a box of smart cards, and a sign reading \"ENTROPY: FRESHLY HARVESTED FROM LAVA LAMPS.\"").await;
}

pub async fn warden_holo(c: Ctx) {
    c.sfx("warden");
    if !c.flag("vault.opened") {
        c.say1(Sp::Warden, "*(looping)* …all private keys are to be escrowed with the Directorate by the end of the month. Those with nothing to hide have nothing to fear…").await;
    } else if !c.flag("ch4.done") {
        c.say1(Sp::Warden, "*(looping)* The Archive at the Mint has been secured under Order 0417. The Directorate will curate its contents for public safety.").await;
    } else if !c.flag("ch5.done") {
        c.say1(Sp::Warden, "*(looping)* Checkpoints at the Masquerade are for your protection. Present your key for verification. Cooperation is appreciated.").await;
    } else if !c.flag("ch6.done") {
        c.say1(Sp::Warden, "*(looping)* Project HARVEST proceeds on schedule. Encryption is a temporary inconvenience.").await;
    } else {
        c.say1(Sp::Narrator, "The projector flickers, shows a test pattern, and gives up.").await;
    }
}
