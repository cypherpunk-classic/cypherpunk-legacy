//! The Exchange: which panels stand where, who is on each line, what they
//! say, and the order the district opens in.
//!
//! Countersigns and the tower's code come from the save's seed; every key
//! on every line is a real Diffie-Hellman exchange. The ideas are fixed:
//!
//! | site | idea |
//! |---|---|
//! | junction box | a cord carries what is said to whoever is on the other end |
//! | call box row | nobody can see who answered: listen, then answer as the desk |
//! | yard post | a scrambled line, and a relay that is each end's other end |
//! | camera street | words read off the key catch a relay, unless you turn its knob until they agree |
//! | tower line | a sealed handshake beats the knob; a small group does not need one |
//! | LOGJAM | baby steps and giant steps through the export group |
//! | switchboard | their relay on your line, and a LEAF with a 16-bit checksum |

use cpl_crypto::clipper::Escrow;
use cpl_crypto::dh::{DRONE_GROUP, WEAK_GROUP};

use super::{Effect, End, Jack, Lines, Panel, Party, RelaySet, Role, Sec, Side, Step, Tape, Via, Word, cipher, decipher};
use crate::signals::{Locker, Pose, Signals, Site, SiteKind, Walk, Watcher};
use crate::state::GameState;

pub const GATE_SQUARE: &str = "x.gate.square";
pub const POST2: &str = "x.post2";
pub const RELAY: &str = "x.relay";
pub const POST3: &str = "x.post3";
pub const CAM_OFF: &str = "x.cam.off";
/// the call number recorded off the tower line, plus one
pub const TAPE: &str = "x.tape";
pub const DIVE_LOGJAM: &str = "x.dive.logjam";
/// the tower's code, read off the tape
pub const CODE: &str = "x.code";
pub const TOWER: &str = "x.tower";
pub const ESCROW_SEEN: &str = "x.escrow";
pub const FREE: &str = "x.free";
pub const END: &str = "x.end";

pub const GUARD_RADIUS: f32 = 30.0;
pub const GUARD_GAIN: f32 = 45.0;
pub const POST_RADIUS: f32 = 26.0;
pub const POST_GAIN: f32 = 40.0;
pub const FOOTSTEPS: f32 = 84.0;

const fn t(n: f32) -> f32 {
    n * 16.0
}

/// Where the two posts stand (bottom-centre, pixels), shared with the map.
pub const POST2_AT: (f32, f32) = (t(12.5) + 8.0, t(17.0) + 14.0);
pub const POST3_AT: (f32, f32) = (t(34.5) + 8.0, t(17.0) + 14.0);

/// The agent who walks the tram square.
pub fn agent_walk() -> Walk {
    Walk { pts: vec![(t(13.0), t(28.2), 0.8), (t(44.0), t(28.2), 0.8), (t(44.0), t(33.0), 0.8), (t(13.0), t(33.0), 0.8)], speed: 30.0 }
}

const POOL: [Word; 16] = ["KITE", "SKIFF", "HERON", "OTTER", "LARK", "PIKE", "WREN", "FINCH", "CUTTER", "TERN", "MARTEN", "RAVEN", "SLOOP", "ADDER", "BADGER", "PLOVER"];

struct Secrets {
    post2: Vec<(Word, Word)>,
    post3: Vec<(Word, Word)>,
    code: Vec<u8>,
}

fn secrets(st: &GameState) -> Secrets {
    let mut rng = st.world().rng("exchange/lines");
    let mut pool = POOL.to_vec();
    for i in (1..pool.len()).rev() {
        let j = rng.int(i + 1);
        pool.swap(i, j);
    }
    let pairs = |w: &[Word]| -> Vec<(Word, Word)> { w.chunks(2).map(|c| (c[0], c[1])).collect() };
    let code = (0..4).map(|_| 1 + rng.int(9) as u8).collect();
    Secrets { post2: pairs(&pool[0..6]), post3: pairs(&pool[6..12]), code }
}

/// The tower's code for this save.
pub fn code(st: &GameState) -> Vec<u8> {
    secrets(st).code
}

fn party(id: &'static str, name: &'static str, role: Role, sec: Sec) -> Party {
    Party { id, name, role, sec, tamper: 25.0 }
}

fn jack(end: End, label: &'static str) -> Jack {
    Jack { end, label, needs: None }
}

fn relay_jacks() -> [Jack; 2] {
    [Jack { end: End::Relay(Side::L), label: "RELAY L", needs: Some(RELAY) }, Jack { end: End::Relay(Side::R), label: "RELAY R", needs: Some(RELAY) }]
}

fn panel(id: &'static str, site: &'static str, title: &'static str, jacks: Vec<Jack>, cords: Vec<(usize, usize)>, max_cords: usize, schedule: Option<(usize, f32, f32)>) -> Panel {
    Panel {
        id,
        site,
        title,
        jacks,
        initial: cords.clone(),
        cords,
        max_cords,
        schedule,
        until: None,
        log: Vec::new(),
        call: None,
        calls: 0,
        heard: Vec::new(),
        recorder: false,
        recording: false,
        phone: None,
        forged: false,
        relay: RelaySet::default(),
        t: 0.0,
    }
}

const STRONG: Sec = Sec::Scrambled { group: DRONE_GROUP, words: 0, sealed: false };
const WORDED: Sec = Sec::Scrambled { group: DRONE_GROUP, words: 1, sealed: false };
const EXPORT_SEALED: Sec = Sec::Scrambled { group: WEAK_GROUP, words: 2, sealed: true };
const PHONE: Sec = Sec::Scrambled { group: DRONE_GROUP, words: 2, sealed: true };

impl Lines {
    /// The Exchange's lines for this save.
    pub fn exchange(st: &GameState) -> Lines {
        let s = secrets(st);
        let desk = |id: &'static str, sec: Sec| party(id, "DESK", Role::Desk { post: "HOLD", camera: "WATCH", code: s.code.clone() }, sec);
        let parties = vec![
            party("depot", "TRAM DEPOT", Role::Depot { open: "OPEN", shut: "SHUT" }, Sec::Clear),
            party("tramgate", "TRAM GATE", Role::Gate { open: "OPEN", flag: None }, Sec::Clear),
            party("sqgate", "SQUARE GATE", Role::Gate { open: "OPEN", flag: Some(GATE_SQUARE) }, Sec::Clear),
            party("post2", "POST TWO", Role::Post { table: s.post2.clone(), obey: vec![("WEST", Effect::Flag(POST2)), ("HOLD", Effect::Nothing), ("EAST", Effect::Nothing), ("NORTH", Effect::Alarm), ("SOUTH", Effect::Alarm)] }, Sec::Clear),
            desk("desk2", Sec::Clear),
            party("post3", "POST THREE", Role::Post { table: s.post3.clone(), obey: vec![("NORTH", Effect::Flag(POST3)), ("HOLD", Effect::Nothing), ("WEST", Effect::Nothing), ("EAST", Effect::Alarm), ("SOUTH", Effect::Alarm)] }, STRONG),
            desk("desk3", STRONG),
            party("camera", "CAMERA", Role::Camera { obey: vec![("SLEEP", Effect::Flag(CAM_OFF)), ("WATCH", Effect::Nothing)] }, WORDED),
            desk("deskc", WORDED),
            Party { tamper: 45.0, ..desk("deskt", EXPORT_SEALED) },
            party("tower", "TOWER", Role::Tower, EXPORT_SEALED),
            party("trunk", "WHARF TRUNK", Role::Trunk { live: TOWER }, PHONE),
        ];
        let line = |id| jack(End::Line(id), "");
        let [rl, rr] = relay_jacks();
        let [rl2, rr2] = relay_jacks();
        let [rl3, rr3] = relay_jacks();
        let [rl4, rr4] = relay_jacks();
        let mut panels = vec![
            panel("junction", "junction", "Junction box", vec![line("depot"), line("tramgate"), line("sqgate")], vec![(0, 1)], 1, Some((0, 10.0, 4.0))),
            Panel { until: Some(POST2), ..panel("callbox", "callbox", "Call box row", vec![line("post2"), line("desk2"), jack(End::Handset, ""), rl, rr], vec![(0, 1)], 2, Some((0, 9.0, 3.0))) },
            Panel { until: Some(POST3), ..panel("yardbox", "yard", "Yard post", vec![line("post3"), line("desk3"), jack(End::Handset, ""), rl2, rr2], vec![(0, 1)], 2, Some((0, 10.0, 5.0))) },
            Panel { until: Some(CAM_OFF), ..panel("cambox", "camera", "Camera line", vec![line("camera"), line("deskc"), jack(End::Handset, ""), rl3, rr3], vec![(0, 1)], 2, Some((0, 10.0, 6.0))) },
            Panel { recorder: true, ..panel("towerbox", "tower", "Tower line", vec![line("deskt"), line("tower"), jack(End::Handset, ""), rl4, rr4], vec![(0, 1)], 2, Some((0, 14.0, 4.0))) },
            Panel { phone: Some(PHONE), ..panel("switchboard", "switchboard", "The switchboard", vec![jack(End::Handset, "YOUR PHONE"), line("trunk"), jack(End::Monitor(Side::L), "MONITOR L"), jack(End::Monitor(Side::R), "MONITOR R")], vec![(0, 2), (3, 1)], 2, None) },
        ];
        for p in &mut panels {
            for j in &mut p.jacks {
                if j.label.is_empty() {
                    j.label = match j.end {
                        End::Line(id) => parties.iter().find(|x| x.id == id).map(|x| x.name).unwrap_or(id),
                        End::Handset => "YOU",
                        _ => "",
                    };
                }
            }
        }
        if let Some(p) = panels.iter_mut().find(|p| p.id == "switchboard") {
            p.log.push(super::LogLine { t: 0.0, who: String::new(), text: "The night operator left the Directorate's monitor patched in between the public phone and the trunk. It has sat there since the escrow law.".into(), tone: super::Tone::System });
        }
        let escrow = Escrow::new(&mut st.world().rng("exchange/escrow"), 4);
        Lines { parties, panels, escrow, events: Vec::new(), tape_read: false }
    }

    /// The tower line's call `n` as a recorder took it.
    pub fn tape_of(&self, st: &GameState, n: u32) -> Option<Tape> {
        let Sec::Scrambled { group, .. } = EXPORT_SEALED else { return None };
        let base = format!("lines/towerbox/{n}");
        let a = super::secret(st, &group, &format!("{base}/caller"));
        let b = super::secret(st, &group, &format!("{base}/far"));
        let k = group.shared(a, group.public(b));
        let text = format!("TOWER, MAINTENANCE {}", code(st).iter().map(|d| d.to_string()).collect::<String>());
        Some(Tape { n, group, a_pub: group.public(a), b_pub: group.public(b), cipher: cipher(&group, k, n, &text) })
    }

    /// The tape in your pocket, if any.
    pub fn tape(&self, st: &GameState) -> Option<Tape> {
        match st.get(TAPE) {
            Some(crate::state::FlagValue::Num(x)) if *x >= 1.0 => self.tape_of(st, *x as u32 - 1),
            _ => None,
        }
    }

    /// The numbers LOGJAM has to break: the desk's on the tape, then the next call's.
    pub fn logjam_targets(&self, st: &GameState) -> Vec<u64> {
        let Some(t) = self.tape(st) else { return Vec::new() };
        let next = self.tape_of(st, t.n + 1).map(|x| x.a_pub);
        [Some(t.a_pub), next].into_iter().flatten().collect()
    }

    /// Once LOGJAM has the desk's number, the tape plays: the key is the
    /// tower's public number raised to the desk's secret.
    pub fn read_tape(&mut self, st: &mut GameState) -> Option<String> {
        let t = self.tape(st)?;
        // the dive found the desk's secret; recompute it the same way the line drew it
        let secret = super::secret(st, &t.group, &format!("lines/towerbox/{}/caller", t.n));
        debug_assert_eq!(t.group.public(secret), t.a_pub);
        let k = t.group.shared(secret, t.b_pub);
        let text = decipher(&t.group, k, t.n, &t.cipher);
        if let Some(p) = self.panel_mut("towerbox") {
            p.line("TAPE", text.clone(), super::Tone::Good);
        }
        Some(text)
    }

    /// District reactions that do not belong to any one call.
    pub fn after(&mut self, st: &mut GameState) {
        if st.flag(DIVE_LOGJAM) && !st.flag(CODE) && self.read_tape(st).is_some() {
            st.set(CODE, true);
            self.tape_read = true;
            self.events.push(super::LineEvent::TapeRead);
        }
    }

    // --- who is walking about ---------------------------------------------------------

    pub fn poses(&self, st: &GameState, t_now: f32) -> Vec<Pose> {
        let (x, y, dx, dy) = agent_walk().at(t_now);
        vec![Pose { id: "x-agent", x, y, dx, dy, flying: false, visible: !st.flag(FREE) }]
    }

    pub fn watchers(&self, st: &GameState, t_now: f32) -> Vec<Watcher> {
        if st.flag(FREE) {
            return Vec::new();
        }
        let (x, y, ..) = agent_walk().at(t_now);
        let mut v = vec![Watcher { id: "x-agent", x, y, radius: crate::campaign::guard_radius(st, crate::state::MapId::Exchange, GUARD_RADIUS), gain: GUARD_GAIN }];
        if !st.flag(POST2) {
            v.push(Watcher { id: "x-post2", x: POST2_AT.0, y: POST2_AT.1, radius: crate::campaign::post_radius(st, POST_RADIUS), gain: POST_GAIN });
        }
        if !st.flag(POST3) {
            v.push(Watcher { id: "x-post3", x: POST3_AT.0, y: POST3_AT.1, radius: crate::campaign::post_radius(st, POST_RADIUS), gain: POST_GAIN });
        }
        v
    }

    pub fn footsteps(&self, st: &GameState, t_now: f32, x: f32, y: f32) -> bool {
        if st.flag(FREE) {
            return false;
        }
        let (gx, gy, dx, dy) = agent_walk().at(t_now);
        let (ox, oy) = (x - gx, y - gy);
        let d = (ox * ox + oy * oy).sqrt();
        d <= FOOTSTEPS && d > GUARD_RADIUS * 0.6 && ox * dx + oy * dy > 0.0
    }

    pub fn exposure(&self, st: &GameState, t_now: f32, x: f32, y: f32) -> f32 {
        self.watchers(st, t_now).iter().filter(|w| ((w.x - x).powi(2) + (w.y - y).powi(2)).sqrt() <= w.radius).map(|w| w.gain).sum()
    }
}

impl Signals {
    /// The Exchange keeps its sites and its one locker where the map and the
    /// locker window look for them; it has no lamp boards.
    pub fn exchange(st: &GameState) -> Signals {
        let lockers = vec![Locker { id: "door", site: "door", title: "Tower door", code: code(st), any_order: false, flag: TOWER, tries: 0 }];
        Signals { boards: Vec::new(), lockers, sites: sites(), t: 0.0, events: Vec::new(), guard_moved_at: None }
    }
}

/// Every place on the district map, in the order the map lists them.
pub fn sites() -> Vec<Site> {
    vec![
        Site {
            id: "junction",
            name: "The junction box",
            blurb: "Three lines and one cord, by the square gate.",
            spawn: "junction",
            needs: &[],
            blocked_by: "",
            done: GATE_SQUARE,
            kind: SiteKind::Boards,
            notes: &[
                "A cord joins two jacks. Whatever one end says, the other end hears.",
                "The depot only ever talks to whoever its cord reaches. It does not know who that is.",
                "Move the depot's cord from the tram gate to the square gate and wait for the depot to call.",
            ],
        },
        Site {
            id: "callbox",
            name: "Call box row",
            blurb: "A guard who takes his orders by phone.",
            spawn: "callbox",
            needs: &[GATE_SQUARE],
            blocked_by: "the square gate",
            done: POST2,
            kind: SiteKind::Boards,
            notes: &[
                "He cannot see who answers his call. He only checks that they know the countersign.",
                "There are only a few challenges, and they come round again. Listen to the desk answer before you answer for it.",
                "Leave his cord on the desk until you have heard two countersigns, then plug him into your handset and answer: the countersign, then WEST.",
            ],
        },
        Site {
            id: "yard",
            name: "The yard post",
            blurb: "Post three's line is scrambled.",
            spawn: "yard",
            needs: &[POST2],
            blocked_by: "post two's boom",
            done: POST3,
            kind: SiteKind::Boards,
            notes: &[
                "A scrambler only talks to another scrambler. Felix has a box that is two of them.",
                "Put the relay in the middle: each end agrees a key with your box and never with each other.",
                "Post three to RELAY L, RELAY R to the desk, set the relay to pass on NORTH, connect when it holds.",
            ],
        },
        Site {
            id: "camera",
            name: "Camera street",
            blurb: "The camera's line reads a word off its key.",
            spawn: "camera",
            needs: &[POST3],
            blocked_by: "post three's boom",
            done: CAM_OFF,
            kind: SiteKind::Boards,
            notes: &[
                "Both ends read a word made from their key. With your relay in the middle there are two keys, so two words.",
                "Your relay holds the call. Every turn of its knob draws a new number for the desk's side, and a new word.",
                "Turn until the desk's word matches the camera's, pass on SLEEP, then connect.",
            ],
        },
        Site {
            id: "tower",
            name: "The tower line",
            blurb: "Sealed handshakes, in an export-grade group.",
            spawn: "tower",
            needs: &[CAM_OFF],
            blocked_by: "the camera",
            done: TAPE,
            kind: SiteKind::Boards,
            notes: &[
                "Sealed: each end commits to its number before it sees the other's. Your relay cannot pick a number after the fact.",
                "You do not have to stand in the middle of a line to read it if the group is small. Read the key exchange: EXPORT GRADE, 32 bits.",
                "Arm the recorder and let the desk call the tower. Then take the tape down to the cabinet: LOGJAM.",
            ],
        },
        Site {
            id: "cellar",
            name: "LOGJAM",
            blurb: "Take the tower's numbers apart.",
            spawn: "cellar",
            needs: &[TAPE],
            blocked_by: "a recording of the tower line",
            done: DIVE_LOGJAM,
            kind: SiteKind::Dive("logjam"),
            notes: &["Count, then build a table and hop.", "The best table holds about the square root of the group's size.", "Every number on the same group falls to the same table."],
        },
        Site {
            id: "door",
            name: "The tower door",
            blurb: "Its code was on the tape.",
            spawn: "door",
            needs: &[DIVE_LOGJAM],
            blocked_by: "the tower's code",
            done: TOWER,
            kind: SiteKind::Boards,
            notes: &["The desk read the tower its maintenance code. You have the recording, and now the key.", "The code is in the tower panel's log.", "Dial the four digits the tape said."],
        },
        Site {
            id: "switchboard",
            name: "The switchboard",
            blurb: "Call Mara on the Wharf.",
            spawn: "switchboard",
            needs: &[TOWER],
            blocked_by: "the tower",
            done: FREE,
            kind: SiteKind::Boards,
            notes: &[
                "Your phone reads two words, and so does Mara's. If they differ, someone is on the line: look at where your cord goes.",
                "Every phone on the trunk sends a LEAF: your session key, for the escrow desk. The phone at the far end only checks a 16-bit checksum.",
                "Plug your phone straight into the trunk, switch the LEAF to FORGED, and call. Let it try LEAFs until Mara's chip takes one.",
            ],
        },
    ]
}

/// The panel at a site.
pub fn panel_of(site: &str) -> Option<&'static str> {
    Some(match site {
        "junction" => "junction",
        "callbox" => "callbox",
        "yard" => "yardbox",
        "camera" => "cambox",
        "tower" => "towerbox",
        "switchboard" => "switchboard",
        _ => return None,
    })
}

// --- a player who knows the answers -----------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum XMove {
    Wait,
    Plug(&'static str, usize, usize),
    Pull(&'static str, usize),
    Answer(&'static str, Option<Word>, Word),
    Rewrite(&'static str, Option<Word>),
    Turn(&'static str),
    Connect(&'static str),
    Record(&'static str),
    Dial(&'static str),
    Forge(&'static str, bool),
    /// ask Felix for his relay
    Felix,
    Locker(&'static str, Vec<u8>),
}

impl XMove {
    pub fn panel(&self) -> Option<&'static str> {
        match self {
            XMove::Plug(p, ..) | XMove::Pull(p, _) | XMove::Answer(p, ..) | XMove::Rewrite(p, _) | XMove::Turn(p) | XMove::Connect(p) | XMove::Record(p) | XMove::Dial(p) | XMove::Forge(p, _) => Some(p),
            _ => None,
        }
    }
}

impl Lines {
    /// The next cord to pull or plug to make the panel's cords exactly `want`.
    fn cords_to(&self, panel: &'static str, want: &[(usize, usize)]) -> Option<XMove> {
        let p = self.panel(panel)?;
        let same = |a: (usize, usize), b: (usize, usize)| a == b || (a.1, a.0) == b;
        if let Some(&(a, _)) = p.cords.iter().find(|c| !want.iter().any(|w| same(**c, *w))) {
            return Some(XMove::Pull(panel, a));
        }
        want.iter().find(|w| !p.cords.iter().any(|c| same(*c, **w))).map(|&(a, b)| XMove::Plug(panel, a, b))
    }

    pub fn next_move(&self, st: &GameState, site: &str) -> Option<XMove> {
        match site {
            "junction" => Some(self.cords_to("junction", &[(0, 2)]).unwrap_or(XMove::Wait)),
            "callbox" => {
                let p = self.panel("callbox")?;
                if p.heard.len() < 2 {
                    return Some(self.cords_to("callbox", &[(0, 1)]).unwrap_or(XMove::Wait));
                }
                if let Some(m) = self.cords_to("callbox", &[(0, 2)]) {
                    return Some(m);
                }
                match &p.call {
                    Some(c) if c.step == Step::Answer && c.far == Some(2) => match c.challenge.and_then(|ch| p.heard.iter().find(|(a, _)| *a == ch)) {
                        Some((_, cs)) => Some(XMove::Answer("callbox", Some(cs), "WEST")),
                        None => Some(XMove::Wait),
                    },
                    _ => Some(XMove::Wait),
                }
            }
            "yard" | "camera" => {
                if !st.flag(RELAY) {
                    return Some(XMove::Felix);
                }
                let (panel, order) = if site == "yard" { ("yardbox", "NORTH") } else { ("cambox", "SLEEP") };
                let p = self.panel(panel)?;
                if p.relay.rewrite != Some(order) {
                    return Some(XMove::Rewrite(panel, Some(order)));
                }
                if let Some(m) = self.cords_to(panel, &[(0, 3), (4, 1)]) {
                    return Some(m);
                }
                match &p.call {
                    Some(c) if c.step == Step::Hold && matches!(c.via, Via::Relay(_)) && !c.connected => {
                        let ok = c.keys.as_ref().is_none_or(|k| k.words == 0 || k.matched());
                        Some(if ok { XMove::Connect(panel) } else { XMove::Turn(panel) })
                    }
                    _ => Some(XMove::Wait),
                }
            }
            "tower" => {
                let p = self.panel("towerbox")?;
                if let Some(m) = self.cords_to("towerbox", &[(0, 1)]) {
                    return Some(m);
                }
                Some(if p.recording { XMove::Wait } else { XMove::Record("towerbox") })
            }
            "door" => Some(XMove::Locker("door", code(st))),
            "switchboard" => {
                let p = self.panel("switchboard")?;
                if let Some(m) = self.cords_to("switchboard", &[(0, 1)]) {
                    return Some(m);
                }
                if p.call.is_some() {
                    return Some(XMove::Wait);
                }
                if st.flag(ESCROW_SEEN) && !p.forged {
                    return Some(XMove::Forge("switchboard", true));
                }
                Some(XMove::Dial("switchboard"))
            }
            _ => None,
        }
    }

    /// Do a move (the autoplayer, and the tests).
    pub fn apply(&mut self, st: &mut GameState, m: &XMove) -> bool {
        match m {
            XMove::Plug(p, a, b) => self.plug(st, p, *a, *b),
            XMove::Pull(p, j) => self.pull(p, *j),
            XMove::Answer(p, cs, o) => self.answer(p, *cs, o),
            XMove::Rewrite(p, w) => {
                self.set_rewrite(p, *w);
                true
            }
            XMove::Turn(p) => self.turn(st, p),
            XMove::Connect(p) => self.connect(p),
            XMove::Record(p) => self.record(p),
            XMove::Dial(p) => self.dial(st, p),
            XMove::Forge(p, f) => {
                self.set_forged(p, *f);
                true
            }
            XMove::Felix => {
                st.set(RELAY, true);
                true
            }
            XMove::Locker(..) | XMove::Wait => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lines::{LineEvent, Tone};
    use cpl_crypto::dh::Bsgs;

    fn fresh(seed: &str) -> (Lines, Signals, GameState) {
        let mut st = GameState::new();
        st.new_game("t", 0, Some(seed));
        let lines = Lines::exchange(&st);
        let sig = Signals::exchange(&st);
        (lines, sig, st)
    }

    /// Run the clock until `done` holds, or give up.
    fn run(lines: &mut Lines, st: &mut GameState, secs: f32, mut done: impl FnMut(&Lines, &GameState) -> bool) -> bool {
        let dt = 1.0 / 30.0;
        let mut t = 0.0;
        while t < secs {
            lines.tick(st, dt);
            lines.after(st);
            if done(lines, st) {
                return true;
            }
            t += dt;
        }
        false
    }

    #[test]
    fn the_depot_opens_whatever_its_cord_reaches() {
        let (mut l, _, mut st) = fresh("x1");
        // as found: the tram gate opens and shuts, the square stays shut
        assert!(run(&mut l, &mut st, 12.0, |l, _| l.panel("junction").unwrap().log.iter().any(|x| x.text == "shuts.")));
        assert!(!st.flag(GATE_SQUARE));
        assert!(l.pull("junction", 0));
        assert!(l.plug(&st, "junction", 0, 2));
        assert!(!l.plug(&st, "junction", 1, 2), "one cord only");
        assert!(run(&mut l, &mut st, 12.0, |_, st| st.flag(GATE_SQUARE)));
        // the latch holds when the depot says SHUT
        assert!(run(&mut l, &mut st, 6.0, |l, _| l.panel("junction").unwrap().log.iter().any(|x| x.text.contains("latch"))));
        assert!(st.flag(GATE_SQUARE));
    }

    #[test]
    fn a_guard_obeys_whoever_knows_the_countersign() {
        let (mut l, _, mut st) = fresh("x2");
        st.set(GATE_SQUARE, true);
        // listen while the desk answers him
        assert!(run(&mut l, &mut st, 40.0, |l, _| l.panel("callbox").unwrap().heard.len() >= 2));
        assert!(!st.flag(POST2), "the desk only ever says HOLD");
        // a wrong countersign is reported, the right one obeyed
        let mut wrong_tried = false;
        let mut right = false;
        let mut l2 = l;
        assert!(l2.pull("callbox", 0));
        assert!(l2.plug(&st, "callbox", 0, 2));
        let dt = 1.0 / 30.0;
        for _ in 0..(120.0 / dt) as usize {
            l2.tick(&mut st, dt);
            let p = l2.panel("callbox").unwrap();
            if let Some(c) = &p.call
                && c.step == Step::Answer
            {
                let known = c.challenge.and_then(|ch| p.heard.iter().find(|(a, _)| *a == ch).map(|(_, b)| *b));
                match known {
                    Some(cs) if wrong_tried => {
                        assert!(l2.answer("callbox", Some(cs), "WEST"));
                        right = true;
                    }
                    _ if !wrong_tried => {
                        let before = st.trace;
                        assert!(l2.answer("callbox", Some("NONSENSE"), "WEST"));
                        for _ in 0..60 {
                            l2.tick(&mut st, dt);
                        }
                        assert!(st.trace > before, "a wrong countersign is reported");
                        assert!(!st.flag(POST2));
                        wrong_tried = true;
                    }
                    _ => {}
                }
            }
            if st.flag(POST2) {
                break;
            }
        }
        assert!(right && st.flag(POST2), "he went west for the right countersign");
    }

    #[test]
    fn a_relay_reads_and_rewrites_a_scrambled_line() {
        let (mut l, _, mut st) = fresh("x3");
        // a cord that only listens hears noise
        assert!(run(&mut l, &mut st, 20.0, |l, _| l.panel("yardbox").unwrap().log.iter().any(|x| x.tone == Tone::Noise && x.who == "DESK")));
        assert!(l.panel("yardbox").unwrap().heard.is_empty());
        // your handset is no scrambler
        assert!(l.pull("yardbox", 0));
        assert!(l.plug(&st, "yardbox", 0, 2));
        assert!(run(&mut l, &mut st, 12.0, |l, _| l.events.iter().any(|e| matches!(e, LineEvent::NoCarrier { .. }))));
        // without the relay its jacks do nothing
        assert!(l.pull("yardbox", 0));
        assert!(!l.plug(&st, "yardbox", 0, 3));
        st.set(RELAY, true);
        while let Some(m) = l.next_move(&st, "yard") {
            match m {
                XMove::Wait => {
                    l.tick(&mut st, 1.0 / 30.0);
                    if st.flag(POST3) || l.panel("yardbox").unwrap().t > 200.0 {
                        break;
                    }
                }
                m => assert!(l.apply(&mut st, &m), "{m:?}"),
            }
        }
        assert!(st.flag(POST3));
        let p = l.panel("yardbox").unwrap();
        assert!(p.log.iter().any(|x| x.tone == Tone::Relay && x.text.contains("Challenge")), "the relay reads the post in the clear");
        assert!(!p.heard.is_empty(), "and the desk's countersign");
    }

    #[test]
    fn words_catch_a_relay_until_its_knob_is_turned() {
        let (mut l, _, mut st) = fresh("x4");
        st.set(RELAY, true);
        assert!(l.pull("cambox", 0));
        assert!(l.plug(&st, "cambox", 0, 3) && l.plug(&st, "cambox", 4, 1));
        // connect blind: the words differ (unless the seed was kind), and the camera reports it
        assert!(run(&mut l, &mut st, 20.0, |l, _| l.panel("cambox").unwrap().call.as_ref().is_some_and(|c| c.step == Step::Hold)));
        let matched = l.panel("cambox").unwrap().call.as_ref().unwrap().keys.as_ref().unwrap().matched();
        let before = st.trace;
        l.connect("cambox");
        run(&mut l, &mut st, 6.0, |l, _| l.panel("cambox").unwrap().call.is_none());
        if !matched {
            assert!(st.trace > before && l.events.iter().any(|e| matches!(e, LineEvent::Tamper { .. })));
        }
        // the solver turns the knob until they agree
        let mut turns = 0;
        while !st.flag(CAM_OFF) && l.panel("cambox").unwrap().t < 300.0 {
            match l.next_move(&st, "camera").unwrap() {
                XMove::Wait => l.tick(&mut st, 1.0 / 30.0),
                XMove::Turn(p) => {
                    assert!(l.turn(&st, p));
                    turns += 1;
                }
                m => assert!(l.apply(&mut st, &m), "{m:?}"),
            }
        }
        assert!(st.flag(CAM_OFF), "after {turns} turns");
        assert!(turns < 400, "a five-bit word takes about 32 turns, not {turns}");
    }

    #[test]
    fn a_sealed_line_cannot_be_ground_and_a_small_group_falls_anyway() {
        let (mut l, _, mut st) = fresh("x5");
        st.set(RELAY, true);
        assert!(l.pull("towerbox", 0));
        assert!(l.plug(&st, "towerbox", 0, 3) && l.plug(&st, "towerbox", 4, 1));
        assert!(run(&mut l, &mut st, 20.0, |l, _| l.panel("towerbox").unwrap().call.as_ref().is_some_and(|c| c.step == Step::Hold)));
        assert!(!l.turn(&st, "towerbox"), "the knob is locked on a sealed line");
        l.connect("towerbox");
        let before = st.trace;
        run(&mut l, &mut st, 6.0, |l, _| l.panel("towerbox").unwrap().call.is_none());
        assert!(st.trace >= before + 44.0, "the tower line reports tampering hard");
        // listen instead: record a call
        l.reset();
        st.trace = 0.0;
        assert!(l.record("towerbox"));
        assert!(run(&mut l, &mut st, 20.0, |_, st| st.flag(TAPE)));
        let tape = l.tape(&st).expect("a tape");
        assert!(tape.group.bits() < 40);
        // baby-step giant-step finds the desk's secret, and the tape plays
        let x = Bsgs::new(tape.group, tape.a_pub, cpl_crypto::dh::sqrt_ceil(tape.group.q)).run().expect("a log");
        assert_eq!(tape.group.public(x), tape.a_pub);
        let k = tape.group.shared(x, tape.b_pub);
        let text = decipher(&tape.group, k, tape.n, &tape.cipher);
        let digits: String = code(&st).iter().map(|d| d.to_string()).collect();
        assert!(text.ends_with(&digits), "{text}");
        assert_eq!(l.logjam_targets(&st).len(), 2);
        st.set(DIVE_LOGJAM, true);
        l.after(&mut st);
        assert!(st.flag(CODE));
        assert!(l.panel("towerbox").unwrap().log.iter().any(|x| x.who == "TAPE" && x.text.contains(&digits)));
    }

    #[test]
    fn the_last_call_needs_a_clean_line_and_a_forged_leaf() {
        let (mut l, _, mut st) = fresh("x6");
        // the trunk is dead until the tower is yours
        assert!(l.dial(&mut st, "switchboard"));
        assert!(run(&mut l, &mut st, 4.0, |l, _| l.panel("switchboard").unwrap().call.is_none()));
        assert!(l.panel("switchboard").unwrap().log.iter().any(|x| x.text.contains("trunk is dead")));
        st.set(TOWER, true);
        // through their monitor the words differ and Mara hangs up
        assert!(l.dial(&mut st, "switchboard"));
        assert!(run(&mut l, &mut st, 10.0, |l, _| l.panel("switchboard").unwrap().call.is_none()));
        assert!(l.panel("switchboard").unwrap().log.iter().any(|x| x.who == "MARA" && x.text.contains("hanging up")));
        assert!(!st.flag(FREE));
        // the solver: a straight cord, an honest call, then a forged LEAF
        let mut guard = 0;
        while !st.flag(FREE) && guard < 30 * 200 {
            guard += 1;
            match l.next_move(&st, "switchboard").unwrap() {
                XMove::Wait => {
                    l.tick(&mut st, 1.0 / 30.0);
                }
                m => assert!(l.apply(&mut st, &m), "{m:?}"),
            }
        }
        assert!(st.flag(ESCROW_SEEN), "the honest LEAF gave the key away first");
        assert!(st.flag(FREE));
        let p = l.panel("switchboard").unwrap();
        assert!(p.log.iter().any(|x| x.who == "ESCROW" && x.text.contains("No key") || x.who == "ESCROW" && x.text.contains("opens nothing")));
    }

    #[test]
    fn every_save_has_its_own_countersigns_and_code() {
        let (a, _, sa) = fresh("one");
        let (b, _, sb) = fresh("two");
        let ta = match &a.party("post2").unwrap().role {
            Role::Post { table, .. } => table.clone(),
            _ => unreachable!(),
        };
        let tb = match &b.party("post2").unwrap().role {
            Role::Post { table, .. } => table.clone(),
            _ => unreachable!(),
        };
        assert!(ta != tb || code(&sa) != code(&sb));
        // challenges and countersigns are all different words
        let mut words: Vec<Word> = ta.iter().flat_map(|(x, y)| [*x, *y]).collect();
        words.sort_unstable();
        words.dedup();
        assert_eq!(words.len(), 6);
    }

    #[test]
    fn sites_open_in_order() {
        let (_, sig, mut st) = fresh("order");
        let mut seen = Vec::new();
        while let Some(s) = sig.next_site(&st) {
            assert!(s.needs.iter().all(|f| st.flag(f)));
            seen.push(s.id);
            st.set(s.done, true);
        }
        assert_eq!(seen, ["junction", "callbox", "yard", "camera", "tower", "cellar", "door", "switchboard"]);
    }
}
