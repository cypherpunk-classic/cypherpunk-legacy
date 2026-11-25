//! The lanterns: a safe house's lamp in the city, and one at the start of
//! each district. Rest at one and the trace drops, chaff recharges, the cards
//! in your pocket are safe from your next fall, and the street sends you
//! back here when it catches you. Stars are spent here and nowhere else.

use crate::script::Ctx;
use crate::state::MapId;

pub async fn rest(c: Ctx, at: String) {
    c.cutscene(async {
        c.sfx("power");
        c.flash(450.0, 0xffb347);
        let banked = {
            let mut s = c.state();
            crate::campaign::rest(&mut s, &at);
            s.data.banked.len()
        };
        let sub = if c.current_map() == MapId::City { "the street forgets you for a while" } else if banked > 0 { "your cards are safe here" } else { "the street forgets you for a while" };
        c.location("YOU REST", sub);
        c.wait(700.0).await;
    })
    .await;
    c.state().save();
    // the kit is bought at a lantern, nowhere else
    c.signal("rest");
}
