//! A gauntlet: a quarter of the city replayed from the campaign map as its
//! terminals back to back, on a fresh seed so every number is new. Leaving a
//! terminal unsolved ends the run; solving the last one scores it.

use crate::campaign::{self, GAUNTLET_DONE};
use crate::script::Ctx;

pub async fn run(c: Ctx, node: &'static str) {
    let Some(n) = campaign::node(node) else { return };
    let ids = campaign::terminals(node);
    c.cutscene(async {
        c.location(&n.name.to_uppercase(), &format!("gauntlet · {} terminals, fresh numbers", ids.len()));
        c.wait(1400.0).await;
    })
    .await;
    for (i, id) in ids.iter().enumerate() {
        let Some(p) = crate::puzzles::make(id, &c) else { continue };
        if i > 0 {
            c.sfx("blip");
            c.wait(350.0).await;
        }
        if !c.puzzle(p).await {
            // walked away: back to the real save, nothing scored
            c.signal("replay.end");
            return;
        }
    }
    c.set(GAUNTLET_DONE);
    // let the result card land before the real save comes back
    c.wait(3200.0).await;
    c.signal("replay.end");
}
