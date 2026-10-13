//! Chapter 3 · Threshold. Juno, Shamir sharing, dead drops, Feldman VSS, and
//! opening the Vault.

use cpl_crypto::aead::gcm_decrypt_text;

use crate::puzzles::{FeldmanVerify, ShamirReconstruct};
use crate::say;
use crate::script::Ctx;
use crate::speakers::Sp;

async fn run_shamir(c: &Ctx) {
    if !c.puzzle(Box::new(ShamirReconstruct::new(c.world()))).await {
        c.objective("Learn how shares work at Juno's workshop.");
        return;
    }
    c.set("ch3.shamir");
    c.codex(&["shamir"]);
    say!(
        c,
        (Juno, "Feel it? Two shares and the secret is anywhere. A thousand people with two shares each learn nothing. Three shares and it collapses to one answer."),
        (Juno, "That is Shamir, 1979. We split the Vault key five ways. You have Mara's and Felix's. You need one more good one."),
        (Juno, "I would just give you mine, but the Directorate got clever. They leave fake shares in our dead drops. A loose brick by the north wall, the hollow of the old oak. One of those is poisoned."),
        (Juno, "Go collect them both. Then come back and we will sort the real from the fake. Properly."),
    )
    .await;
    c.objective("Collect the two dead-drop shares in the Commons (loose brick; hollow tree).");
}

async fn run_feldman(c: &Ctx) {
    if !c.puzzle(Box::new(FeldmanVerify::new(c.world()))).await {
        c.objective("Verify the shares and reconstruct the Vault key.");
        return;
    }
    c.set("ch3.feldman");
    c.codex(&["feldman"]);
    c.give("vault-key");
}

pub async fn juno(c: Ctx) {
    if !c.flag("ch3.met") {
        c.set("ch3.met");
        say!(
            c,
            (Juno, "You are the one undoing the Directorate's knots. Word travels. I'm Juno. I split secrets so that no single person, me included, can leak them."),
            (Juno, "Your grandfather and I designed the Vault lock together. Five shares, any three open it, any two are useless. Even to us."),
            (Juno, "Before I trust you with the real thing, let me show you why two is useless and three is everything. Here, in miniature."),
        )
        .await;
        run_shamir(&c).await;
        return;
    }
    if !c.flag("ch3.shamir") {
        return run_shamir(&c).await;
    }
    if !c.flag("ch3.gotDrops") {
        let (a, b) = (c.flag("ch3.dropA"), c.flag("ch3.dropB"));
        if !a && !b {
            c.say1(Sp::Juno, "Both drops are still out there. The loose brick is by the north wall; the hollow oak is east. Bring me what you find.").await;
        } else {
            let text = format!("You have {} share. Still need {}.", if a { "the brick" } else { "the oak" }, if a { "the oak (east)" } else { "the brick (north wall)" });
            c.say1(Sp::Juno, &text).await;
        }
        return;
    }
    if !c.flag("ch3.feldman") {
        c.say1(Sp::Juno, "Four shares, then. Two from friends, two from the ground. One of the ground ones lies. Let's find out which, without ever exposing the secret.").await;
        return run_feldman(&c).await;
    }
    if !c.flag("ch3.done") {
        c.set("ch3.done");
        c.chapter(3);
        c.sfx("unlock");
        say!(
            c,
            (Juno, "There it is. The Vault key, whole and verified. You caught the forgery before it ever touched the reconstruction. That is what commitments are for."),
            (Juno, "Go. The Vault has waited a year for him, and now for you. Legacy Plaza, the monolith in the centre."),
            (Nonce, "Finally."),
        )
        .await;
        c.objective("Open the Vault on Legacy Plaza.");
        return;
    }
    c.say1(Sp::Juno, "The Vault is yours to open. We will be watching the plaza. All of us.").await;
}

fn check_drops(c: &Ctx) {
    if c.flag("ch3.dropA") && c.flag("ch3.dropB") {
        c.set("ch3.gotDrops");
        c.objective("Bring the shares to Juno to verify and reconstruct.");
    }
}

pub async fn drop_a(c: Ctx) {
    if !c.flag("ch3.shamir") {
        c.say1(Sp::Narrator, "A brick in the north wall sits slightly proud of the others. Chalk mark beside it. But you do not yet know what to do with what is behind it.").await;
        return;
    }
    if c.flag("ch3.dropA") {
        c.say1(Sp::Narrator, "The brick is back in place. Nothing more behind it.").await;
        return;
    }
    c.set("ch3.dropA");
    c.give("share3");
    c.sfx("pickup");
    say!(
        c,
        (Narrator, "Behind the loose brick: a share card, x = 3. The chalk mark looks fresh, and a little too neat."),
        (Nonce, "Directorate neat. Verify that one carefully."),
    )
    .await;
    check_drops(&c);
}

pub async fn drop_b(c: Ctx) {
    if !c.flag("ch3.shamir") {
        c.say1(Sp::Narrator, "The old oak has a hollow at shoulder height. Something is tucked inside, but not yet.").await;
        return;
    }
    if c.flag("ch3.dropB") {
        c.say1(Sp::Narrator, "The hollow is empty now.").await;
        return;
    }
    c.set("ch3.dropB");
    c.give("share4");
    c.sfx("pickup");
    c.say1(Sp::Narrator, "Inside the hollow oak: a share card, x = 4, wrapped in oilcloth the way Grandpa wrapped everything.").await;
    check_drops(&c);
}

pub async fn kiosk(c: Ctx) {
    if !c.flag("ch3.met") {
        c.say1(Sp::System, "COMMONS TERMINAL\nCommunity board. Talk to Juno in the park.").await;
        return;
    }
    if !c.flag("ch3.shamir") {
        return run_shamir(&c).await;
    }
    if c.flag("ch3.gotDrops") && !c.flag("ch3.feldman") {
        return run_feldman(&c).await;
    }
    c.say1(Sp::System, "COMMONS TERMINAL\nShare verification complete. The Commons thanks you.").await;
}

pub async fn mailbox(c: Ctx) {
    c.say1(Sp::Narrator, "A community mailbox, painted over with a mask stencil. A notice inside: \"DROP ETIQUETTE: take only what is yours. Verify everything. Trust the math.\"").await;
}

/// The Vault before the finale.
pub async fn vault(c: Ctx) {
    if c.flag("vault.opened") {
        c.say1(Sp::Narrator, "The Vault stands open, its five sockets lit. Inside, the space where the letter was.").await;
        return;
    }
    if !c.has("vault-key") {
        say!(
            c,
            (Narrator, "A black monolith, taller than you. Five sockets glow faintly along its face; three are dark."),
            (Nonce, "You need the Vault key. Three good shares. Juno, in the Commons."),
        )
        .await;
        return;
    }
    c.cutscene(open_vault(&c)).await;
}

async fn open_vault(c: &Ctx) {
    let (vx, vy) = c.object_pos("vault").unwrap_or((480.0, 360.0));
    c.sfx("power");
    c.set_object_frame("vault", 2);
    c.burst(vx, vy - 40.0, 0x3ef0ff, 40);
    c.flash(500.0, 0x73eff7);
    c.set("vault.opened");
    say!(
        c,
        (Narrator, "You press the Vault key into the lock. Five sockets flare cyan, one after another. The monolith splits with a sound like a held breath let go."),
        (Narrator, "Inside: a single envelope, and a small data shard on a chain."),
    )
    .await;
    c.set("ch3.vaultletter");
    read_vault_letter(c).await;
    c.chapter(4);
    c.set("warden.on");
    c.sfx("warden");
    say!(
        c,
        (Warden, "*(every screen in the plaza turns red)* Unauthorised Vault access detected. Legacy Plaza is now a restricted zone."),
        (Warden, "Whoever you are: the Archive you are looking for is already ours. Order 0417. It is sealed at the Mint, under a Directorate key. You will never open it."),
        (Nonce, "He sounds nervous. Good. Ibrahim is at the Mint. He has been studying Warden's signatures for months. The gate is open."),
    )
    .await;
    c.objective("Find Ibrahim at the Mint (north-east). The Archive is sealed there.");
}

async fn read_vault_letter(c: &Ctx) {
    let text = {
        let w = c.world();
        let v = w.vault();
        gcm_decrypt_text(&v.letter_key, &v.letter).unwrap_or_else(|_| "(the letter is water-damaged beyond reading)".into())
    };
    c.codex(&["manifesto"]);
    let paras: Vec<(Sp, String)> = text.split("\n\n").map(|p| (Sp::Letter, p.trim().to_string())).filter(|(_, p)| !p.is_empty()).collect();
    c.say(paras).await;
    c.give("identity");
}
