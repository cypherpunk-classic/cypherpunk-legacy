//! Epilogue: the finale at the Vault, the broadcast, and the credits.

use crate::puzzles::Broadcast;
use crate::say;
use crate::script::Ctx;

async fn finale(c: &Ctx) {
    let (vx, vy) = c.object_pos("vault").unwrap_or((480.0, 360.0));
    c.cutscene(async {
        c.music("finale");
        c.pan_to(vx, vy - 20.0, 800.0).await;
        say!(
            c,
            (Narrator, "The plaza is full. Mara from the Wharf, Felix from the Exchange, Juno, Ibrahim, Lin, Ada. Everyone who handed you a piece of this."),
            (Warden, "*(the last broadcast, smaller now)* Stop. Whatever you are about to publish, the Directorate orders you to—"),
            (Nonce, "He does not get to finish that sentence. You do."),
        )
        .await;
        c.follow();
    })
    .await;
    if !c.puzzle(Box::new(Broadcast::new(c.world()))).await {
        return;
    }
    c.set("end.broadcast");
    c.codex(&["manifesto", "crypto-wars"]);
    c.give("identity");
    c.cutscene(async {
        c.burst(vx, vy - 30.0, 0x3ef0ff, 60);
        c.flash(700.0, 0x3ef0ff);
        c.sfx("success");
        c.clear("warden.on");
        say!(
            c,
            (Narrator, "The signature goes out to every node at once. Across Cryptopolis, screens that showed Warden's face flicker, and show the manifesto instead. The Directorate's broadcast simply… stops. You cannot order a verified signature to un-exist."),
            (Mara, "He used to say the city was never the point. The code was the point. The code outlives all of us."),
            (Ada, "And now the next ones can check our work. That was always the whole bet."),
            (Nonce, "Your grandfather would be unbearable right now. In the good way."),
            (Nonce, "Thank you, {{handle}}. For finishing his sentence."),
        )
        .await;
        c.set("end.done");
        c.chapter(7);
    })
    .await;
    c.objective("");
    c.state().save();
    c.wait(600.0).await;
    c.credits();
}

/// The Vault, with the finale taking over once Ada is done.
pub async fn vault(c: Ctx) {
    if c.flag("end.ready") && !c.flag("end.done") {
        finale(&c).await;
        return;
    }
    if c.flag("end.done") {
        say!(
            c,
            (Narrator, "The Vault stands open and lit, the manifesto scrolling across its face. People come here to read it, and to add their own code to the Archive."),
            (Nonce, "We did good. Go get some sleep. I will watch the city."),
        )
        .await;
        c.credits();
        return;
    }
    super::ch3::vault(c).await;
}
