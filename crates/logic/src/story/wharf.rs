//! The Wharf: arriving, the few things anyone says, the gates, being
//! caught, the mast, and walking out. Everything else is the boards.

use crate::say;
use crate::script::{BoxFut, Ctx};
use crate::signals::wharf::{END, MAST, sites};
use crate::speakers::Sp;
use crate::state::{GameState, MapId};

/// The first open site that is not done yet, from flags alone.
fn next_site(st: &GameState) -> Option<crate::signals::Site> {
    sites().into_iter().find(|s| s.needs.iter().all(|f| st.flag(f)) && !st.flag(s.done))
}

pub async fn ready(c: Ctx) {
    if c.flag("w.intro") {
        return;
    }
    c.set("w.intro");
    c.set("pro.metCat");
    c.location("THE WHARF", "where the Directorate speaks in lamps");
    c.objective("Every lamp board on the Wharf talks for the Directorate. Make one say something it was never told. (M: map)");
    c.wait(900.0).await;
    say!(
        c,
        (Mara, "The harbour. Every lamp board down here talks for the Directorate: the gates, the patrols, the camera, the mast. They speak in pictures, and they hide the pictures under noise."),
        (Mara, "Your grandfather's card first. Try it on the lamps out on the piers, and watch what it does."),
    )
    .await;
}

pub fn interaction(c: Ctx, id: &str) -> Option<BoxFut> {
    let id = id.to_string();
    if let Some(b) = id.strip_prefix("w.board.") {
        let b = b.to_string();
        return Some(Box::pin(async move { c.board(&b).await }));
    }
    if let Some(l) = id.strip_prefix("w.locker.") {
        let l = l.to_string();
        return Some(Box::pin(async move { c.locker(&l).await }));
    }
    if let Some(d) = id.strip_prefix("w.dive.") {
        let d = d.to_string();
        return Some(Box::pin(async move {
            if d == "lamplighter" && !c.flag("w.lamplighter.met") {
                c.set("w.lamplighter.met");
                say!(
                    c,
                    (Narrator, "*(a drone the size of a dinghy hangs over the mole, a floodlight under it like a lamp on a pole)* LAMPLIGHTER UNIT 7. RESTORING DIRECTORATE SIGNAL TO THE MAST."),
                    (Mara, "They want it back. Its light takes three lanes and leaves one dark. Which one is two bits of its keystream, every beat."),
                    (Nonce, "Dodge it and you live. Call the dark lane before the light shows and the light comes back at it. A call is a prediction. You will have to earn it."),
                )
                .await;
            }
            c.sfx("power");
            if c.dive(&d).await {
                c.sfx("success");
            }
        }));
    }
    let gate_board = |g: &str| -> Option<&'static str> {
        match g {
            "w.gate.q" => Some("q"),
            "w.gate.c0" => Some("b0"),
            "w.gate.c1" => Some("b1"),
            "w.gate.m" => Some("c3"),
            "w.gate.w" => Some("d1"),
            _ => None,
        }
    };
    if let Some(b) = gate_board(&id) {
        // a gate is only its lock: look at the gate and you are looking at its board
        return Some(Box::pin(async move { c.board(b).await }));
    }
    Some(match id.as_str() {
        "w.gate.ice" => Box::pin(async move { c.say1(Sp::Narrator, "Chained from the inside. The chain runs back to a locker in the market, the one with signs painted on it.").await }),
        "w.gate.city" => Box::pin(async move { c.say1(Sp::Narrator, "The steps back up to the radio shack. The mast reports everyone who passes under it. Not while it is theirs.").await }),
        "w.pick.gull" => Box::pin(async move {
            c.signal("pick:gull");
            c.sfx("pickup");
            c.say1(Sp::Narrator, "A punched card, tucked under a crate on the second pier. Someone wrote GULL on it and left it for whoever came next.").await;
        }),
        "w.mara" => Box::pin(mara(c)),
        "w.citizen.1" => Box::pin(async move {
            c.say1(Sp::Narrator, "*(the fishwife, gutting mackerel)* That notice board? Shows the Directorate's crest every day they've nothing to say. Which is most days.").await;
        }),
        "w.citizen.2" => Box::pin(async move {
            c.say1(Sp::Narrator, "*(a docker on his break)* Supply drone comes in over the mole every half minute. They open the gate for it and shut it behind. Same signal every time, regular as the tide.").await;
        }),
        "w.citizen.3" => Box::pin(async move {
            c.say1(Sp::Narrator, "*(a customs clerk)* New clerks train on the rig by the radio shack. There's a monitor on it that shows what the lock reads. Saves them guessing.").await;
        }),
        "w.exit" => Box::pin(walk_out(c)),
        "nonce" => Box::pin(nonce(c)),
        _ => return None,
    })
}

async fn mara(c: Ctx) {
    if c.flag(MAST) {
        c.say1(Sp::Mara, "Listen to it. Every board on the wharf saying his sign. The road north is open whenever you are.").await;
        return;
    }
    let next = next_site(&c.state_ref());
    match next {
        Some(s) => {
            let line = format!("{}. {}", s.name, s.notes.first().copied().unwrap_or(s.blurb));
            c.say1(Sp::Mara, &line).await;
        }
        None => c.say1(Sp::Mara, "Nothing left on the wharf that answers to them but the mast.").await,
    }
}

async fn nonce(c: Ctx) {
    c.sfx("meow");
    let line = match next_site(&c.state_ref()) {
        Some(s) => format!("Mrrp. *(it looks toward the {})*", s.name.to_lowercase().trim_start_matches("the ")),
        None => "Mrrp.".to_string(),
    };
    c.say1(Sp::Nonce, &line).await;
}

/// The player entered a zone on the Wharf.
pub async fn zone(c: Ctx, id: String) {
    let (name, sub) = match id.as_str() {
        "w.z.market" => ("FISH MARKET", "notice boards and a sign locker"),
        "w.z.plaza" => ("QUAY PLAZA", "the radio shack and the training rig"),
        "w.z.customs" => ("CUSTOMS YARD", ""),
        "w.z.alley" => ("THE ALLEY", "mind the guard"),
        "w.z.shed" => ("HARBOUR MASTER'S YARD", "the tide board"),
        "w.z.mole" => ("THE MOLE", "the drone gate"),
        "w.z.mast" => ("THE MAST", ""),
        "w.exit" => {
            if c.flag(MAST) && !c.flag(END) {
                walk_out(c).await;
            }
            return;
        }
        _ => return,
    };
    let key = format!("{id}.seen");
    if !c.flag(&key) {
        c.set(&key);
        c.location(name, sub);
    }
}

/// The trace hit 100 on the Wharf.
pub async fn flagged(c: Ctx, x: f32, y: f32) {
    c.cutscene(async {
        c.flash(350.0, 0xff3355);
        c.sfx("warden");
        let (times, dropped, lost) = {
            let mut s = c.state();
            s.data.stats.flagged += 1;
            let (d, l) = crate::campaign::drop_loose(&mut s, MapId::Wharf, x, y);
            (s.data.stats.flagged, d, l)
        };
        say!(
            c,
            (Agent, "*(a drone drops to eye level, then an agent steps out of a doorway)* Device logged near Directorate equipment. Turn out your pockets. Move along."),
            (Nonce, if times == 1 { "They emptied every reader on the wharf and sent you back to the boathouse. Your cards are still yours. Watch the TRACE bar: the guard, the camera, and every lock that does not like what it reads." } else { "Again. The boathouse, then." }),
        )
        .await;
        if dropped > 0 || lost > 0 {
            let mut line = String::new();
            if dropped > 0 {
                line.push_str(&format!("You dropped {dropped} {} where they caught you: everything you had rubbed or cut since you last rested at a lantern. Walk back and pick {} up. Get caught again first and {} gone.", if dropped == 1 { "card" } else { "cards" }, if dropped == 1 { "it" } else { "them" }, if dropped == 1 { "it's" } else { "they're" }));
            }
            if lost > 0 {
                line.push_str(&format!(" The {lost} you dropped last time are gone for good."));
            }
            c.say1(Sp::Nonce, line.trim()).await;
        }
        c.state().trace = 0.0;
        c.signal("flagged");
        c.goto(MapId::Wharf, Some("start")).await;
    })
    .await;
}

/// The mast answers to the Underground.
pub async fn mast_freed(c: Ctx) {
    c.cutscene(async {
        c.wait(1200.0).await;
        c.flash(500.0, 0xffcd75);
        c.sfx("success");
        c.location("THE WHARF IS FREE", "every board says his sign");
        c.wait(1600.0).await;
        say!(
            c,
            (Mara, "There. Every board on the wharf just started saying your grandfather's name, and not one of them was ever told his key."),
            (Mara, "Their pads were perfect, you know. Nobody beats a one-time pad. You beat the people using it."),
            (Mara, "Walk back up through the market's north gate when you are ready. I'll meet you at the radio shack."),
        )
        .await;
        c.objective("The harbour is free. Walk out through the north gate of the fish market, back up to Mara at the radio shack.");
    })
    .await;
}

/// Out through the market's north gate, and back up to the radio shack.
async fn walk_out(c: Ctx) {
    if !c.flag(MAST) || c.flag(END) {
        return;
    }
    c.set(END);
    if c.state_ref().data.replay_of.is_some() {
        // a replay from the campaign map: back to the real save, stars and all
        c.sfx("door");
        c.signal("replay.end");
        return;
    }
    c.cutscene(async {
        c.location("THE HARBOUR, FREE", "back up to the radio shack");
        c.wait(1600.0).await;
        c.goto(MapId::City, Some("mara")).await;
    })
    .await;
    c.objective("The harbour is free. Tell Mara.");
}
