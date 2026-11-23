//! The Exchange: arriving from the Wharf, Felix, the few things anyone says,
//! being caught, the last call, and walking out. Everything else is the lines.

use crate::lines::exchange::{self as x, END, FREE, POST2, RELAY, sites};
use crate::say;
use crate::script::{BoxFut, Ctx};
use crate::speakers::Sp;
use crate::state::{GameState, MapId};

/// The first open site that is not done yet, from flags alone.
fn next_site(st: &GameState) -> Option<crate::signals::Site> {
    sites().into_iter().find(|s| s.needs.iter().all(|f| st.flag(f)) && !st.flag(s.done))
}

pub async fn ready(c: Ctx) {
    if c.flag("x.intro") {
        return;
    }
    c.set("x.intro");
    c.location("THE EXCHANGE", "every call goes through their desk");
    c.objective("Every line in the district runs through the Directorate's desk. Make one say something it was never told. (M: map)");
    c.wait(900.0).await;
    say!(
        c,
        (Felix, "The tram square. Every call in this district goes through their desk, and the desk never asks who is calling."),
        (Felix, "Start with the junction box by the square gate. Watch what the depot says, and to whom."),
    )
    .await;
}

pub fn interaction(c: Ctx, id: &str) -> Option<BoxFut> {
    let id = id.to_string();
    if let Some(p) = id.strip_prefix("x.panel.") {
        let p = p.to_string();
        return Some(Box::pin(async move { c.board(&p).await }));
    }
    if let Some(l) = id.strip_prefix("x.locker.") {
        let l = l.to_string();
        return Some(Box::pin(async move {
            let before = c.flag(x::TOWER);
            c.locker(&l).await;
            if !before && c.flag(x::TOWER) {
                c.objective("The tower is yours, and the trunk to the Wharf with it. Call Mara from the switchboard behind the Exchange.");
            }
        }));
    }
    if let Some(d) = id.strip_prefix("x.dive.") {
        let d = d.to_string();
        return Some(Box::pin(async move {
            if d == "logjam" && !c.flag(x::TAPE) {
                c.say1(Sp::Narrator, "A cabinet humming in the tower's equipment shed: LOGJAM. It wants a recording of a key exchange to chew on, and you have none.").await;
                return;
            }
            c.sfx("power");
            if c.dive(&d).await {
                c.sfx("success");
            }
        }));
    }
    Some(match id.as_str() {
        "x.felix" => Box::pin(felix(c)),
        "x.gate.square" => Box::pin(async move { c.say1(Sp::Narrator, "The square gate. It opens for anyone on its line who says OPEN, and its line runs back to the junction box beside it.").await }),
        "x.gate.boom2" => Box::pin(async move { c.say1(Sp::Narrator, "Post two's boom. He lifts it when the desk tells him to go somewhere else.").await }),
        "x.gate.boom3" => Box::pin(async move { c.say1(Sp::Narrator, "Post three's boom. His call box hisses: a scrambled line.").await }),
        "x.gate.tower" => Box::pin(async move { c.say1(Sp::Narrator, "The tower gate is slaved to the camera on the corner: shut for as long as it watches.").await }),
        "x.gate.north" => Box::pin(async move { c.say1(Sp::Narrator, "The road north, back to Felix's switchboard. Not while every call still goes through their desk.").await }),
        "x.citizen.1" => Box::pin(async move {
            c.say1(Sp::Narrator, "*(a tram conductor, on her break)* The depot rings the tram gate every few seconds: OPEN, then SHUT. The gate never asks who is calling. Gates never do.").await;
        }),
        "x.citizen.2" => Box::pin(async move {
            c.say1(Sp::Narrator, "*(a man with a newspaper)* Says here the new scramblers read you a word made from the key, so you can tell a friend from a tap. Two words on the tower line, and sealed, whatever that means.").await;
        }),
        "x.citizen.3" => Box::pin(async move {
            c.say1(Sp::Narrator, "*(a linesman, coiling cable)* The posts check in every few seconds. Challenge, countersign, orders. Same three challenges all week.").await;
        }),
        "x.exit" => Box::pin(walk_out(c)),
        "nonce" => Box::pin(nonce(c)),
        _ => return None,
    })
}

async fn felix(c: Ctx) {
    if c.flag(FREE) {
        c.say1(Sp::Felix, "Listen to the lines. Half of them are talking to people the desk will never hear again. The road north is open.").await;
        return;
    }
    if c.flag(POST2) && !c.flag(RELAY) {
        say!(
            c,
            (Felix, "You answered a guard as the desk and he went. Good. Post three is harder: his line is scrambled, and a scrambler only talks to another scrambler."),
            (Felix, "So take this. My old relay: two scramblers back to back. Plug his line into one side and the desk into the other, and each of them agrees a key with your box instead of with each other."),
        )
        .await;
        c.set(RELAY);
        c.sfx("pickup");
        c.location("FELIX'S RELAY", "two scramblers back to back");
        c.objective("Post three's line is scrambled. Felix's relay goes in the middle of it, at the junction box by his boom.");
        return;
    }
    let line = match next_site(&c.state_ref()) {
        Some(s) => format!("{}. {}", s.name, s.notes.first().copied().unwrap_or(s.blurb)),
        None => "Nothing left on these lines that answers to them.".to_string(),
    };
    c.say1(Sp::Felix, &line).await;
}

async fn nonce(c: Ctx) {
    c.sfx("meow");
    let line = match next_site(&c.state_ref()) {
        Some(s) => format!("Mrrp. *(it sits down facing {})*", s.name.to_lowercase()),
        None => "Mrrp.".to_string(),
    };
    c.say1(Sp::Nonce, &line).await;
}

/// The player entered a zone in the Exchange.
pub async fn zone(c: Ctx, id: String) {
    let (name, sub) = match id.as_str() {
        "x.z.square" => ("TRAM SQUARE", "the depot and the junction box"),
        "x.z.row" => ("CALL BOX ROW", "a guard who takes his orders by phone"),
        "x.z.yard" => ("THE YARD", "the Exchange's back door"),
        "x.z.street" => ("CAMERA STREET", ""),
        "x.z.tower" => ("THE TOWER", "the line out of the district"),
        "x.exit" => {
            if c.flag(FREE) && !c.flag(END) {
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

/// The trace hit 100 in the Exchange.
pub async fn flagged(c: Ctx) {
    c.cutscene(async {
        c.flash(350.0, 0xff3355);
        c.sfx("warden");
        let times = {
            let mut s = c.state();
            s.data.stats.flagged += 1;
            s.data.stats.flagged
        };
        say!(
            c,
            (Agent, "*(an agent steps out of a doorway, a lineman's set on his belt)* Somebody's been in the junction boxes. Was it you? Move along."),
            (Nonce, if times == 1 { "They pulled your cords, put theirs back, and walked you to the tram square. Watch the TRACE bar: the agent's round, the posts, the camera, and every line that catches you lying." } else { "The square again, then." }),
        )
        .await;
        c.state().trace = 0.0;
        c.signal("flagged");
        c.goto(MapId::Exchange, Some("start")).await;
    })
    .await;
}

/// The call nobody else heard.
pub async fn freed(c: Ctx) {
    c.cutscene(async {
        c.wait(1400.0).await;
        c.flash(500.0, 0xffcd75);
        c.location("THE EXCHANGE IS FREE", "a call nobody else heard");
        c.wait(1500.0).await;
        say!(
            c,
            (Felix, "Do you know how long it has been since a call left this building without the desk keeping a copy?"),
            (Felix, "Their mathematics was fine. Every line ran an honest key exchange. They never checked who they were talking to, they put the tower in a group small enough to count, and they trusted a sixteen-bit checksum to keep the escrow honest."),
            (Felix, "Take the road north, past the tower, and I'll meet you back at the switchboard. There's a share with your name on it."),
        )
        .await;
        c.objective("The Exchange is free. Take the road north, past the tower, back to Felix.");
    })
    .await;
}

async fn walk_out(c: Ctx) {
    if !c.flag(FREE) || c.flag(END) {
        return;
    }
    c.set(END);
    c.cutscene(async {
        c.location("THE LINES, FREE", "back to the switchboard");
        c.wait(1600.0).await;
        c.goto(MapId::City, Some("felix")).await;
    })
    .await;
    c.objective("The Exchange is free. Felix is waiting at the switchboard.");
}
