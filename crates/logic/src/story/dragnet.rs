//! The Dragnet: what happens when the Directorate's sensors have logged you
//! long enough.

use crate::say;
use crate::script::Ctx;
use crate::state::MapId;

/// Safe houses: (spawn name, the flag that opens it, x, y in pixels).
pub const SAFE_HOUSES: &[(&str, &str, f32, f32)] = &[
    ("safe-wharf", "city.arrived", 13.0 * 16.0, 36.0 * 16.0),
    ("safe-exchange", "ch1.done", 12.0 * 16.0, 25.0 * 16.0),
    ("safe-commons", "ch2.done", 50.0 * 16.0, 22.0 * 16.0),
    ("safe-mint", "party.done", 50.0 * 16.0, 10.0 * 16.0),
    ("safe-masks", "ch4.done", 12.0 * 16.0, 10.0 * 16.0),
    ("safe-obs", "ch5.done", 30.0 * 16.0, 10.0 * 16.0),
];

/// The nearest open safe house to (x, y).
pub fn nearest_safe_house(c: &Ctx, x: f32, y: f32) -> &'static str {
    SAFE_HOUSES
        .iter()
        .filter(|(_, cond, _, _)| c.flag(cond))
        .min_by(|a, b| {
            let da = (a.2 - x).powi(2) + (a.3 - y).powi(2);
            let db = (b.2 - x).powi(2) + (b.3 - y).powi(2);
            da.partial_cmp(&db).unwrap()
        })
        .map(|s| s.0)
        .unwrap_or("safe-wharf")
}

pub async fn flagged(c: Ctx, x: f32, y: f32) {
    c.cutscene(async {
        c.flash(350.0, 0xff3355);
        c.sfx("warden");
        let times = {
            let mut s = c.state();
            s.data.stats.flagged += 1;
            s.data.stats.flagged
        };
        let fined = c.state().fine(2);
        let fine_line = if fined > 0 { format!("Fine: {fined} credits, collected. Move along.") } else { "No funds on you. A note goes in your file instead. Move along.".to_string() };
        say!(
            c,
            (Agent, "*(a drone drops to eye level, then an agent steps out of a doorway)* Device logged at three sensors in ten minutes. Name and purpose."),
            (Agent, fine_line),
            (Nonce, if times == 1 { "That was the Dragnet. The masts log every phone that pings them; the cameras sweep. It's not what you say, it's that you were there. Metadata.".to_string() } else { "Again? Stay out of the fields, or carry less that talks.".to_string() }),
        )
        .await;
        c.state().trace = 0.0;
        let spawn = nearest_safe_house(&c, x, y);
        c.goto(MapId::City, Some(spawn)).await;
        if times == 1 {
            say!(
                c,
                (Nonce, "A safe house. Your friends keep a few around the city; you wake up at the nearest one."),
                (Nonce, "Watch the TRACE bar. Masts pulse, cameras sweep. Every tool you earn makes you quieter: encrypted messages, signed keys, cash that does not carry your name."),
            )
            .await;
        }
    })
    .await;
    c.codex(&["metadata"]);
}
