//! Epilogue: the finale at the Vault, the broadcast, and the credits.

use crate::puzzles::Broadcast;
use crate::say;
use crate::script::Ctx;
use crate::speakers::Sp;

/// The bills come due before the broadcast: the mint's list, the remailer's table.
async fn reckoning(c: &Ctx) {
    if c.flag("choice.mintlog") && !c.flag("mintlog.seized") {
        super::missions::mint_seizure(c).await;
    }
    if c.flag("choice.logs") && !c.flag("logs.reckoned") {
        c.set("logs.reckoned");
        let pick = c
            .choose(
                Sp::Warden,
                "*(on the plaza screens)* A court order, served on your remailer node, which keeps a table of pseudonyms. Give us the real address behind \"Kestrel\", or your node goes dark tonight.",
                &["Shut the node down. Burn the table.", "Hand over the address."],
            )
            .await;
        if pick == 1 {
            c.set("choice.gaveup");
            say!(
                c,
                (Narrator, "You send the address. Somewhere in the Directorate, a door opens without a knock."),
                (Lin, "…That's what a table is for, in the end. Someone always comes for it."),
            )
            .await;
        } else {
            c.state().take("onion-route");
            say!(
                c,
                (Narrator, "You wipe the node and pull its plug. The table goes with it. Kestrel's pseudonym now points nowhere at all."),
                (Lin, "Helsingius did the same in 1996. It cost him the service, and it cost the users their pseudonyms. It didn't cost anyone their freedom."),
            )
            .await;
        }
    }
}

async fn finale(c: &Ctx) {
    let (vx, vy) = c.object_pos("vault").unwrap_or((480.0, 360.0));
    reckoning(c).await;
    if c.state_ref().data.stamps.is_empty() {
        say!(
            c,
            (Nonce, "The broadcast goes out through the remailers, and they want postage now. Your doing, remember? Mint some stamps at the Masquerade's notice board first."),
        )
        .await;
        c.objective("Mint Hashcash stamps (the notice board at the Masquerade), then return to the Vault.");
        c.set("ch5.stampsAgain");
        return;
    }
    let allies = c.state_ref().ally_count();
    c.cutscene(async {
        c.music("finale");
        c.pan_to(vx, vy - 20.0, 800.0).await;
        let crowd = match allies {
            0..=4 => "The plaza is quieter than you hoped. A few friends, a few curious faces, a lot of Directorate drones overhead.".to_string(),
            5..=9 => format!("The plaza fills: {allies} people who signed your key and whose keys you signed, and the people they brought."),
            _ => format!("The plaza is packed. {allies} names on your web of trust, and every one of them brought someone. The drones have nowhere to land."),
        };
        say!(
            c,
            (Narrator, crowd),
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
    {
        let mut s = c.state();
        let n = s.data.stamps.len().min(3);
        s.data.stamps.drain(..n);
    }
    c.set("end.broadcast");
    c.codex(&["manifesto", "crypto-wars"]);
    c.give("identity");
    let ending = crate::ending::ending(&c.state_ref());
    c.state().set("end.kind", ending.id());
    let key = crate::ending::key_choice(&c.state_ref());
    c.cutscene(async {
        c.burst(vx, vy - 30.0, 0x3ef0ff, 60);
        c.flash(700.0, 0x3ef0ff);
        c.sfx("success");
        c.clear("warden.on");
        say!(
            c,
            (Narrator, "The signature goes out through three remailers, postage paid, to every node at once. Across Cryptopolis, screens that showed Warden's face flicker, and show the manifesto instead."),
        )
        .await;
        match key.as_str() {
            "disclose" => c.say1(Sp::Ibrahim, "And Warden's key is on the revocation list, with the signed note that explains why. Every verifier in the city stopped trusting it an hour ago. That's how it should end.").await,
            "publish" => c.say1(Sp::Ibrahim, "Warden's key is still out there, posted on every board. Half the orders in the city are forgeries now, and nobody can tell which half. We'll be cleaning that up for years.").await,
            "keep" => c.say1(Sp::Warden, "*(a private message, to you alone)* You kept my key. So you understand power after all. We should talk.").await,
            _ => {}
        }
        match ending {
            crate::ending::Ending::OpenSociety => {
                say!(
                    c,
                    (Mara, "He used to say the city was never the point. The code was the point. Look at them: every one of these people can check the signature themselves. Nobody has to take anyone's word for it."),
                    (Ada, "And the next ones can check our work. That was always the whole bet."),
                    (Nonce, "The masts are going dark, one by one. Not because anyone ordered it. Because nobody's listening to them any more."),
                )
                .await;
            }
            crate::ending::Ending::QuietVictory => {
                say!(
                    c,
                    (Mara, "It's out. The Archive is public and nobody can un-sign it. That's not nothing."),
                    (Juno, "But we're fewer than we should be, and they know more about us than they should. We'll have to rebuild the web, slower this time."),
                    (Nonce, "Some masts are still lit. Keep your phone in the pouch."),
                )
                .await;
            }
            crate::ending::Ending::LongWinter => {
                say!(
                    c,
                    (Juno, "The manifesto went out. By morning the Directorate will have a new law, and a new list, and some of the names on it are ours."),
                    (Felix, "Then we do what they did in the nineties. Print it. Copy it. Teach it. Code doesn't need permission to exist."),
                    (Nonce, "The winter is long. It isn't forever."),
                )
                .await;
            }
        }
        say!(
            c,
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
