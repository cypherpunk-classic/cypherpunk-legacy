//! Side quest: the Source. Rosa's source leaves encrypted drops (AES-GCM under
//! a key they share). Her reader shows whatever decrypts and skips the tag
//! check. Tonight two drops are not what they say. Verify each one, decide,
//! then choose a safe nonce for her reply.

use std::rc::Rc;

use cpl_crypto::aead::{GcmBox, ctr_decrypt, gcm_decrypt_text, gcm_encrypt_text};
use cpl_crypto::aead::CtrTicket;
use cpl_crypto::bytes::to_hex;
use egui::{Color32, RichText};

use crate::puzzle::{Host, Puzzle, StepView, steps};
use crate::ui::theme::{AMBER, CYAN, DIM, GREEN, RED, TEXT, mono};
use crate::ui::widgets::{Kind, Tone, button, card, lede, note, select_card};
use crate::world::World;

const TEXTS: [&str; 4] = [
    "Volume 7 exists. They moved it out of the Mint before the seal.",
    "The documents are real. Publish on Friday, not before.",
    "Change of plan: meet me at the Plaza fountain, alone, tonight.",
    "Next drop in three days. Same place, same time.",
];

struct Drop {
    sealed: GcmBox,
    /// what Rosa's careless reader shows (CTR part only, no tag check)
    preview: String,
    genuine: bool,
    checked: Option<bool>,
    verdict: Option<bool>,
}

pub struct SourceCheck {
    key: [u8; 32],
    drops: Vec<Drop>,
    nonce_choice: Option<usize>,
    reply: Option<String>,
}

/// GCM is CTR mode for the text, starting at counter 2: show what a reader that ignores the tag would print.
fn naive_preview(key: &[u8; 32], b: &GcmBox) -> String {
    let mut iv = [0u8; 16];
    iv[..12].copy_from_slice(&b.nonce);
    iv[15] = 2;
    let body = &b.ct[..b.ct.len().saturating_sub(16)];
    ctr_decrypt(key, &CtrTicket { nonce: iv, ct: body.to_vec() })
}

impl SourceCheck {
    pub fn new(world: Rc<World>) -> Self {
        let mut r = world.rng("q/source");
        let key: [u8; 32] = r.array();
        let mut drops: Vec<Drop> = Vec::new();
        for (i, t) in TEXTS.iter().enumerate() {
            let mut nonce = [0u8; 12];
            nonce[11] = (i as u8) * 2 + 2;
            let mut sealed = gcm_encrypt_text(&key, &nonce, t);
            if i == 1 {
                // the Directorate flips "real" into "fake" on the wire: CTR lets it
                let pos = t.find("real").unwrap();
                for (j, (a, b)) in "real".bytes().zip("fake".bytes()).enumerate() {
                    sealed.ct[pos + j] ^= a ^ b;
                }
            }
            if i == 2 {
                // a forgery: drop 1's keystream (it guessed drop 1's text) over new words, same nonce, made-up tag
                let first = &drops[0].sealed;
                let mut ct: Vec<u8> = t.bytes().zip(TEXTS[0].bytes()).zip(first.ct.iter()).map(|((n, o), c)| c ^ o ^ n).collect();
                ct.extend_from_slice(&r.array::<16>());
                sealed = GcmBox { nonce: first.nonce, ct };
            }
            let preview = naive_preview(&key, &sealed);
            drops.push(Drop { sealed, preview, genuine: i == 0 || i == 3, checked: None, verdict: None });
        }
        Self { key, drops, nonce_choice: None, reply: None }
    }

    fn check(&mut self, i: usize) {
        self.drops[i].checked = Some(gcm_decrypt_text(&self.key, &self.drops[i].sealed).is_ok());
    }

    fn judge(&mut self, i: usize, genuine: bool, host: &mut Host) {
        let d = &mut self.drops[i];
        if d.checked.is_none() {
            host.status("Verify the tag before you decide anything about a message.", Tone::Warn);
            return;
        }
        if genuine == d.genuine {
            d.verdict = Some(genuine);
            host.sfx("confirm");
            host.status(if genuine { "Tag verifies under your shared key: it came from the source, unaltered." } else { "Tag fails. Someone changed it on the wire or made it with another key. Rosa's reader would have shown it anyway." }, Tone::Ok);
        } else {
            host.status(if genuine { "Its tag does not verify. A decryption that reads well proves nothing; CTR decrypts anything." } else { "That tag verifies under the key only you and the source share." }, Tone::Bad);
            host.sfx("error");
        }
    }

    fn reply(&mut self, choice: usize, host: &mut Host) {
        self.nonce_choice = Some(choice);
        let last = self.drops[3].sealed.nonce;
        match choice {
            0 => host.status("Same key, same nonce: the keystreams cancel, and anyone with both messages gets their XOR. Worse, GCM's authentication key falls out and the Directorate can forge tags (the \"forbidden attack\").", Tone::Bad),
            1 => host.status("The source counts up by two from 2. Its next drop will use 10, and so would your reply if you take the next number: a collision waiting to happen. Each direction needs its own nonce space.", Tone::Bad),
            2 => host.status("All zeros is just another fixed nonce: the second message you send with it repeats the keystream. (Look at drop 3: the forger reused drop 1's nonce and keystream to write its own words.)", Tone::Bad),
            _ => {
                let mut nonce = [0u8; 12];
                nonce[0] = 0x80; // Rosa's half of the nonce space
                nonce[4..].copy_from_slice(&last[4..]);
                nonce[11] = nonce[11].wrapping_add(1);
                let b = gcm_encrypt_text(&self.key, &nonce, "Got it. Friday. Stay safe. -R");
                self.reply = Some(format!("nonce {} · {}", to_hex(&b.nonce), to_hex(&b.ct[..16])));
                host.sfx("unlock");
                host.solve("Rosa's reply goes out under a nonce from her own half of the space. Her reader checks tags now, too.");
            }
        }
    }

    fn judged(&self) -> bool {
        self.drops.iter().all(|d| d.verdict.is_some())
    }
}

impl Puzzle for SourceCheck {
    fn id(&self) -> &'static str {
        "q.source"
    }

    fn title(&self) -> &str {
        "ROSA'S DROPS"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("AES-GCM · verify before you trust")
    }

    fn accent(&self) -> Color32 {
        AMBER
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Four drops. Rosa's reader shows them all. Which are really from her source?", Tone::Info);
    }

    fn steps(&self) -> Vec<StepView> {
        steps(&[
            ("Verify each drop's tag", "GCM's tag is computed with the shared key over the ciphertext; any change, or another key, breaks it.", self.drops.iter().all(|d| d.checked.is_some())),
            ("Mark each drop genuine or forged", "Trust the tag, not how plausible the text sounds.", self.judged()),
            ("Pick a nonce for Rosa's reply", "One key, two writers: the nonces must never collide, in either direction.", self.reply.is_some()),
        ])
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        lede(ui, "Rosa writes for the *Harbour Light*. Her source inside the Directorate leaves drops encrypted with AES-GCM under a key the two of them share. Her reader decrypts and displays; it never checks the tag. Tonight's drops:");
        let mut act: Option<(u8, usize)> = None;
        for (i, d) in self.drops.iter().enumerate() {
            card(ui, Some(format!("Drop {}", i + 1).as_str()), d.verdict.map(|g| if g { GREEN } else { RED }), |ui| {
                ui.label(RichText::new(format!("reader shows: \"{}\"", d.preview)).font(mono(13.0)).color(TEXT));
                ui.label(RichText::new(format!("nonce {} · tag {}", to_hex(&d.sealed.nonce), to_hex(&d.sealed.ct[d.sealed.ct.len() - 16..]))).font(mono(11.5)).color(DIM));
                ui.horizontal(|ui| {
                    match d.checked {
                        None => {
                            if button(ui, "Verify tag", Kind::Normal).clicked() {
                                act = Some((0, i));
                            }
                        }
                        Some(ok) => {
                            ui.label(RichText::new(if ok { "tag ✓" } else { "tag ✕ (authentication failed)" }).font(mono(12.5)).color(if ok { GREEN } else { RED }));
                        }
                    }
                    if d.verdict.is_none() {
                        if button(ui, "Genuine", Kind::Primary).clicked() {
                            act = Some((1, i));
                        }
                        if button(ui, "Forged", Kind::Danger).clicked() {
                            act = Some((2, i));
                        }
                    }
                });
            });
        }
        if self.judged() {
            card(ui, Some("Rosa's reply"), None, |ui| {
                note(ui, "She wants to answer under the same key. Which nonce?");
                for (i, o) in ["the nonce of drop 4 (it worked before)", "drop 4's nonce + 1", "twelve zero bytes", "a nonce from Rosa's own half of the space (first bit set), never used before"].iter().enumerate() {
                    if select_card(ui, self.nonce_choice == Some(i), CYAN, |ui| {
                        ui.label(RichText::new(*o).font(mono(13.0)).color(TEXT));
                    }) {
                        act = Some((3, i));
                    }
                }
                if let Some(r) = &self.reply {
                    ui.label(RichText::new(r).font(mono(12.0)).color(GREEN));
                }
            });
        }
        match act {
            Some((0, i)) => self.check(i),
            Some((1, i)) => self.judge(i, true, host),
            Some((2, i)) => self.judge(i, false, host),
            Some((3, i)) => self.reply(i, host),
            _ => {}
        }
    }

    fn autoplay(&mut self, host: &mut Host) {
        for i in 0..self.drops.len() {
            self.check(i);
            let g = self.drops[i].checked == Some(true);
            self.judge(i, g, host);
        }
        self.reply(3, host);
    }
}
