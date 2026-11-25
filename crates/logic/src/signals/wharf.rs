//! The Wharf: which boards stand where, what they are told, what their locks
//! obey, and the order the district opens in.
//!
//! Pads, digits and the day's sign come from the save's seed, so every
//! playthrough has its own noise and its own answers. The ideas are fixed:
//!
//! | site | idea |
//! |---|---|
//! | piers | a card is a pad; pads combine |
//! | quay gate | a lock does what the picture says |
//! | training rig, customs gate | flip the cipher and the message flips: no key needed |
//! | patrol orders, camera | the same, prepared on one card and used under pressure |
//! | market | one known message gives up the pad |
//! | ice house | two messages on one pad cancel it |
//! | back gate | an unknown message needs the pad itself |
//! | mole gate | a seal stops forgery, and replay walks past it |
//! | mast | all of it, before the picture changes |

use cpl_crypto::drbg::Drbg;
use cpl_crypto::lamps::{Bits, PICTURE, pad};

use super::glyph::{CAMERA, DIGITS, GATE, ORDERS, SIGNS};
use super::{Board, CardKind, CardRef, Effect, Event, Feed, Glyph, Locker, Pose, Signals, Site, SiteKind, Walk, Watcher};
use crate::maps::T;
use crate::state::GameState;

// --- flags the district sets ---------------------------------------------------------

/// The field kit from the pier locker: wax for rubbings and a light table.
pub const KIT: &str = "w.kit";
pub const GATE_Q: &str = "w.gate.q";
pub const GATE_C0: &str = "w.gate.c0";
pub const GATE_C1: &str = "w.gate.c1";
pub const GUARD_WEST: &str = "w.guard.west";
pub const CAM_OFF: &str = "w.cam.off";
/// The sign locker is open (and with it the ice house).
pub const C1: &str = "w.c1";
/// Grandpa's sigil is in the glyph book.
pub const SIGIL: &str = "w.sigil";
pub const GATE_M: &str = "w.gate.m";
pub const GATE_W: &str = "w.gate.w";
/// You hold a card that is exactly the mast console's pad.
pub const PAD_MAST: &str = "w.pad.mast";
/// The same for the market console (not required, but the map notes it).
pub const PAD_MARKET: &str = "w.pad.market";
/// The mast answers to the Underground: the district is free.
pub const MAST: &str = "w.mast";
pub const GULL: &str = "w.card.gull";
pub const DIVE_INTERCEPTOR: &str = "w.dive.interceptor";
pub const DIVE_STATIC: &str = "w.dive.static";

/// How long the mast keeps one pattern.
pub const MAST_PERIOD: f32 = 26.0;
/// The supply drone: every `DRONE_PERIOD` seconds the mole gate is told to open for `DRONE_OPEN`.
pub const DRONE_PERIOD: f32 = 30.0;
pub const DRONE_AT: f32 = 7.0;
pub const DRONE_OPEN: f32 = 7.0;

pub const GUARD_RADIUS: f32 = 30.0;
pub const GUARD_GAIN: f32 = 45.0;
/// How far off you hear him coming.
pub const FOOTSTEPS: f32 = 84.0;
/// The slice is over: you walked out of the district.
pub const END: &str = "w.end";

const fn t(n: f32) -> f32 {
    n * T
}

fn gate(open: &'static str) -> Vec<(Glyph, Effect)> {
    vec![(Glyph::Open, Effect::Flag(open)), (Glyph::Closed, Effect::Nothing), (Glyph::Alarm, Effect::Alarm)]
}

struct Secrets {
    night: Bits,
    gull: Bits,
    rig: Bits,
    customs: Bits,
    patrol: Bits,
    cam: Bits,
    market: Bits,
    ice: Bits,
    mole: Bits,
    mole_seal: [u8; 16],
    mast: Bits,
    pier_digits: [u8; 2],
    ice_digits: [u8; 2],
    day: Glyph,
    back_pattern: Bits,
    mast_seq: Vec<Bits>,
}

fn two_digits(rng: &mut Drbg) -> [u8; 2] {
    let a = 1 + rng.int(9) as u8;
    let mut b = 1 + rng.int(9) as u8;
    while b == a {
        b = 1 + rng.int(9) as u8;
    }
    [a, b]
}

fn secrets(st: &GameState) -> Secrets {
    let mut rng = st.world().rng("wharf/signals");
    // a pattern that is no glyph and not close to one
    let pattern = |rng: &mut Drbg| loop {
        let p = pad(rng) & PICTURE;
        if Glyph::ALL.iter().all(|g| (g.bits() ^ p).count_ones() > 12) {
            return p;
        }
    };
    Secrets {
        night: pad(&mut rng),
        gull: pad(&mut rng),
        rig: pad(&mut rng),
        customs: pad(&mut rng),
        patrol: pad(&mut rng),
        cam: pad(&mut rng),
        market: pad(&mut rng),
        ice: pad(&mut rng),
        mole: pad(&mut rng),
        mole_seal: rng.array::<16>(),
        mast: pad(&mut rng),
        pier_digits: two_digits(&mut rng),
        ice_digits: two_digits(&mut rng),
        day: *rng.pick(&SIGNS),
        back_pattern: pattern(&mut rng),
        mast_seq: (0..16).map(|_| pattern(&mut rng)).collect(),
    }
}

fn board(id: &'static str, site: &'static str, title: &'static str, console: &'static str, pad: Bits, feed: Feed) -> Board {
    Board { id, site, title, console, pad, seal: None, slots: 3, keys: false, monitor: false, feed, accepts: Vec::new(), reveal: None, legend: Vec::new(), base: 0, picture: 0, hand: 0, cards: Vec::new(), pulls: 0 }
}

impl Signals {
    /// The Wharf's boards for this save.
    pub fn wharf(st: &GameState) -> Signals {
        let s = secrets(st);
        let mut boards = vec![
            Board { reveal: Some("w.a1"), ..board("a1", "pier", "Pier lamp 1", "pier 1", s.night, Feed::Fixed(Glyph::East)) },
            Board { reveal: Some("w.a2"), ..board("a2", "pier", "Pier lamp 2", "pier 2", s.night ^ s.gull, Feed::Fixed(Glyph::of_digit(s.pier_digits[0]))) },
            Board { reveal: Some("w.a3"), ..board("a3", "pier", "Pier lamp 3", "pier 3", s.gull, Feed::Fixed(Glyph::of_digit(s.pier_digits[1]))) },
            Board { keys: true, accepts: gate(GATE_Q), legend: GATE.to_vec(), ..board("q", "quay", "Quay gate", "", 0, Feed::Fixed(Glyph::Closed)) },
            Board { keys: true, monitor: true, accepts: gate(GATE_C0), legend: GATE.to_vec(), ..board("b0", "rig", "Training rig", "rig", s.rig, Feed::Fixed(Glyph::Closed)) },
            Board { keys: true, accepts: gate(GATE_C1), legend: GATE.to_vec(), ..board("b1", "customs", "Customs gate", "customs", s.customs, Feed::Fixed(Glyph::Closed)) },
            Board {
                slots: 1,
                accepts: vec![(Glyph::West, Effect::Flag(GUARD_WEST)), (Glyph::East, Effect::Nothing), (Glyph::North, Effect::Alarm), (Glyph::South, Effect::Alarm)],
                legend: ORDERS.to_vec(),
                ..board("b2", "patrol", "Patrol orders", "watch house", s.patrol, Feed::Fixed(Glyph::East))
            },
            Board { slots: 1, accepts: vec![(Glyph::Shut, Effect::Flag(CAM_OFF)), (Glyph::Eye, Effect::Nothing)], legend: CAMERA.to_vec(), ..board("b3", "camera", "Camera control", "camera", s.cam, Feed::Fixed(Glyph::Eye)) },
            Board { legend: [&[Glyph::Crest][..], &SIGNS[..]].concat(), ..board("c1n", "market", "Market notice", "market", s.market, Feed::Fixed(Glyph::Crest)) },
            Board { reveal: Some("w.c1.read"), legend: [&[Glyph::Crest][..], &SIGNS[..]].concat(), ..board("c1m", "market", "Quartermaster's memo", "market", s.market, Feed::Fixed(s.day)) },
            Board {
                keys: true,
                accepts: SIGNS.iter().map(|g| (*g, if *g == s.day { Effect::Flag(C1) } else { Effect::Alarm })).collect(),
                legend: SIGNS.to_vec(),
                ..board("c1l", "market", "Sign locker", "", 0, Feed::Blank)
            },
            Board { legend: DIGITS.to_vec(), ..board("c2a", "ice", "Ice house, left", "ice house", s.ice, Feed::Fixed(Glyph::of_digit(s.ice_digits[0]))) },
            Board { legend: DIGITS.to_vec(), ..board("c2b", "ice", "Ice house, right", "ice house", s.ice, Feed::Fixed(Glyph::of_digit(s.ice_digits[1]))) },
            Board { accepts: gate(GATE_M), legend: GATE.to_vec(), ..board("c3", "backgate", "Market back gate", "market", s.market, Feed::Pattern(s.back_pattern)) },
            Board {
                keys: true,
                seal: Some(s.mole_seal),
                accepts: gate(GATE_W),
                legend: GATE.to_vec(),
                ..board("d1", "mole", "Mole gate", "mole", s.mole, Feed::Window { rest: Glyph::Closed, open: Glyph::Open, period: DRONE_PERIOD, at: DRONE_AT, len: DRONE_OPEN })
            },
            Board { legend: vec![Glyph::Anchor], ..board("tide", "shed", "Tide board", "mast", s.mast, Feed::Fixed(Glyph::Anchor)) },
            Board {
                accepts: vec![(Glyph::Sigil, Effect::Flag(MAST)), (Glyph::Tower, Effect::Nothing)],
                legend: vec![Glyph::Tower],
                ..board("mast", "mast", "Mast control", "mast", s.mast, Feed::Rotating { seq: s.mast_seq.clone(), period: crate::campaign::mast_period(st) })
            },
        ];
        for b in &mut boards {
            b.picture = b.feed.picture(0.0);
            b.base = b.encode(b.picture);
        }
        let lockers = vec![
            Locker { id: "kit", site: "pier", title: "Boathouse locker", code: s.pier_digits.to_vec(), any_order: true, flag: KIT, tries: 0 },
            Locker { id: "ice", site: "ice", title: "Ice house locker", code: s.ice_digits.to_vec(), any_order: true, flag: SIGIL, tries: 0 },
        ];
        Signals { boards, lockers, sites: sites(), t: 0.0, events: Vec::new(), guard_moved_at: None }
    }

    /// Start of a new game in the district: Grandpa's card is in your pocket.
    pub fn start(&mut self, st: &mut GameState) {
        if st.data.cards.is_empty() && !st.flag("w.started") {
            let s = secrets(st);
            self.add_card(st, "NIGHTJAR".into(), s.night, CardKind::Pad, "Grandpa's card".into());
            st.set("w.started", true);
            self.events.clear();
        }
    }

    /// Pick something up off the ground.
    pub fn pick(&mut self, st: &mut GameState, what: &str) -> bool {
        match what {
            "gull" if !st.flag(GULL) => {
                let s = secrets(st);
                st.set(GULL, true);
                self.add_card(st, "GULL".into(), s.gull, CardKind::Pad, "left on pier 2".into());
                true
            }
            _ => false,
        }
    }

    /// What a freshly set flag brings with it.
    pub(crate) fn reward(&mut self, st: &mut GameState, flag: &str) {
        match flag {
            SIGIL => self.learn(st, &[Glyph::Sigil]),
            KIT => self.learn(st, &GATE),
            GUARD_WEST => {
                let (x, y, ..) = guard_east().at(self.t);
                self.guard_moved_at = Some((self.t, x, y));
            }
            _ => {}
        }
    }

    /// Call after a card is added: holding a console's pad is a milestone.
    pub fn note_pads(&mut self, st: &mut GameState) {
        let s = secrets(st);
        for (flag, p) in [(PAD_MAST, s.mast), (PAD_MARKET, s.market)] {
            if !st.flag(flag) && st.data.cards.iter().any(|c| c.value() == p) {
                st.set(flag, true);
                self.events.push(Event::Solved { board: if flag == PAD_MAST { "tide" } else { "c1n" }, flag });
            }
        }
    }

    // --- who is walking about ---------------------------------------------------------

    pub fn poses(&self, st: &GameState) -> Vec<Pose> {
        let mut v = Vec::new();
        let (x, y, dx, dy) = self.guard_at(st);
        v.push(Pose { id: "w-guard", x, y, dx, dy, flying: false, visible: !st.flag(MAST) });
        let phase = self.t.rem_euclid(DRONE_PERIOD);
        let (a, b) = (DRONE_AT - 3.0, DRONE_AT + DRONE_OPEN + 3.0);
        let away = (t(60.0), t(18.0));
        let over = (t(50.5), t(31.0));
        let (x, y, dx, dy, vis) = if phase >= a && phase < b && !st.flag(MAST) {
            let k = (phase - a) / (b - a);
            // in over the gate, hover, and out again
            let u = if k < 0.35 {
                k / 0.35
            } else if k < 0.65 {
                1.0
            } else {
                (1.0 - k) / 0.35
            };
            let e = u * u * (3.0 - 2.0 * u);
            (away.0 + (over.0 - away.0) * e, away.1 + (over.1 - away.1) * e, if k < 0.35 { -1.0 } else if k < 0.65 { 0.0 } else { 1.0 }, 0.0, true)
        } else {
            (away.0, away.1, 0.0, 0.0, false)
        };
        v.push(Pose { id: "w-drone", x, y, dx, dy, flying: true, visible: vis });
        v
    }

    fn guard_at(&self, st: &GameState) -> (f32, f32, f32, f32) {
        if !st.flag(GUARD_WEST) {
            return guard_east().at(self.t);
        }
        let post = guard_post();
        match self.guard_moved_at {
            Some((t0, x0, y0)) => {
                // from where he stood, out through the customs gate, to the yard's west side
                let via = [(x0, y0), (t(46.5), t(18.5) + 8.0), (t(43.0), t(18.5) + 8.0), (post.pts[0].0, post.pts[0].1)];
                let speed = 46.0;
                let mut left = (self.t - t0) * speed;
                for w in via.windows(2) {
                    let len = ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt();
                    if left < len {
                        let k = left / len.max(0.001);
                        return (w[0].0 + (w[1].0 - w[0].0) * k, w[0].1 + (w[1].1 - w[0].1) * k, (w[1].0 - w[0].0) / len.max(0.001), (w[1].1 - w[0].1) / len.max(0.001));
                    }
                    left -= len;
                }
                post.at(left / speed)
            }
            None => post.at(self.t),
        }
    }

    /// Moving things that raise your trace when you are close.
    pub fn watchers(&self, st: &GameState) -> Vec<Watcher> {
        if st.flag(MAST) {
            return Vec::new();
        }
        let (x, y, ..) = self.guard_at(st);
        vec![Watcher { id: "w-guard", x, y, radius: crate::campaign::guard_radius(st, crate::state::MapId::Wharf, GUARD_RADIUS), gain: GUARD_GAIN }]
    }

    /// Is the guard close and coming closer?
    pub fn footsteps(&self, st: &GameState, x: f32, y: f32) -> bool {
        if st.flag(MAST) {
            return false;
        }
        let (gx, gy, dx, dy) = self.guard_at(st);
        let (ox, oy) = (x - gx, y - gy);
        let d = (ox * ox + oy * oy).sqrt();
        d <= FOOTSTEPS && d > GUARD_RADIUS * 0.6 && ox * dx + oy * dy > 0.0
    }

    /// Trace per second from the watchers at this spot.
    pub fn exposure(&self, st: &GameState, x: f32, y: f32) -> f32 {
        self.watchers(st).iter().filter(|w| ((w.x - x).powi(2) + (w.y - y).powi(2)).sqrt() <= w.radius).map(|w| w.gain).sum()
    }
}

/// The guard's round while his orders say EAST: a loop of the alley.
pub fn guard_east() -> Walk {
    Walk { pts: vec![(t(46.5), t(15.0), 0.6), (t(53.0), t(15.0), 0.6), (t(53.0), t(22.5), 0.6), (t(46.5), t(22.5), 0.6)], speed: 30.0 }
}

/// His post once he is sent WEST: the far side of the customs yard.
pub fn guard_post() -> Walk {
    Walk { pts: vec![(t(34.5), t(21.5), 2.5), (t(38.5), t(21.5), 2.5)], speed: 22.0 }
}

/// Every place on the district map, in the order the map lists them.
pub fn sites() -> Vec<Site> {
    vec![
        Site {
            id: "pier",
            name: "The piers",
            blurb: "Three lamps over the water and a locker by the boathouse.",
            spawn: "pier",
            needs: &[],
            blocked_by: "",
            done: KIT,
            kind: SiteKind::Boards,
            notes: &[
                "A card flips the lamps behind its holes. The right card turns noise into a picture.",
                "Some lamps want two cards at once. Some want only one of the two.",
                "Lamp 1: NIGHTJAR. Lamp 2: both cards. Lamp 3: GULL alone. The two digits open the locker by the boathouse.",
            ],
        },
        Site {
            id: "cabinet",
            name: "Grandpa's cabinet",
            blurb: "INTERCEPTOR. He built it to practise. It still boots.",
            spawn: "cabinet",
            needs: &[],
            blocked_by: "",
            done: DIVE_INTERCEPTOR,
            kind: SiteKind::Dive("interceptor"),
            notes: &[],
        },
        Site {
            id: "quay",
            name: "Quay gate",
            blurb: "A gate, a lamp board with its casing open, and a poster.",
            spawn: "quay",
            needs: &[KIT],
            blocked_by: "the boathouse locker",
            done: GATE_Q,
            kind: SiteKind::Boards,
            notes: &[
                "This lock is not hiding anything. It does what the picture says.",
                "The poster shows the picture that opens the gate. Make the lamps show it.",
                "Flip lamps by hand, or put SHUT and OPEN in the readers together: one erases, the other draws.",
            ],
        },
        Site {
            id: "rig",
            name: "Training rig",
            blurb: "The Directorate trains its clerks here. The monitor shows what the lock sees.",
            spawn: "rig",
            needs: &[GATE_Q],
            blocked_by: "the quay gate",
            done: GATE_C0,
            kind: SiteKind::Boards,
            notes: &[
                "Flip one lamp and watch the monitor.",
                "The lock sees SHUT. Every lamp you flip on the board flips the same lamp for the lock.",
                "Flip exactly the lamps where SHUT and OPEN differ. You never need to read the noise.",
            ],
        },
        Site {
            id: "radio",
            name: "Radio shack",
            blurb: "STATIC. Two harbour broadcasts sent on one key.",
            spawn: "radio",
            needs: &[GATE_Q],
            blocked_by: "the quay gate",
            done: DIVE_STATIC,
            kind: SiteKind::Dive("static"),
            notes: &[],
        },
        Site {
            id: "customs",
            name: "Customs gate",
            blurb: "The same lock as the rig, without the monitor.",
            spawn: "customs",
            needs: &[GATE_C0],
            blocked_by: "the training rig's door",
            done: GATE_C1,
            kind: SiteKind::Boards,
            notes: &[
                "No monitor this time. But you know what the gate is being told right now: it is shut.",
                "The same trick as the rig: the difference between SHUT and OPEN, flipped into the noise.",
                "Stack SHUT and OPEN on the light table to see which lamps differ, then flip those.",
            ],
        },
        Site {
            id: "patrol",
            name: "Patrol orders",
            blurb: "A guard walks the alley. His orders hang behind glass, with one reader.",
            spawn: "patrol",
            needs: &[GATE_C1],
            blocked_by: "the customs gate",
            done: GUARD_WEST,
            kind: SiteKind::Boards,
            notes: &[
                "Glass over the lamps and one reader. Whatever you do has to fit on one card.",
                "He walks the east alley, so the board says EAST. You want it to say WEST.",
                "Cut one card from EAST and WEST stacked. Slot it while he is at the far end, and pull.",
            ],
        },
        Site {
            id: "camera",
            name: "Camera control",
            blurb: "The camera over the mole watches its own control board.",
            spawn: "camera",
            needs: &[GUARD_WEST],
            blocked_by: "the guard in the alley",
            done: CAM_OFF,
            kind: SiteKind::Boards,
            notes: &["It is watching, so you know what its board says.", "Cut the card before you step into view: WATCH and SLEEP.", "Wait for the sweep to pass, step in, slot the card, pull the lever."],
        },
        Site {
            id: "market",
            name: "Fish market",
            blurb: "Two boards on one console, and a locker that wants today's sign.",
            spawn: "market",
            needs: &[GATE_Q],
            blocked_by: "the quay gate",
            done: C1,
            kind: SiteKind::Boards,
            notes: &[
                "Two boards, one console: the same pad on both. On a quiet day the notice board shows the crest.",
                "Noise stacked with the picture it hides leaves the pad. The pad opens every board on that console.",
                "Rub both boards. Stack the notice rubbing, CREST and the memo rubbing. What shows is today's sign: give it to the sign locker.",
            ],
        },
        Site {
            id: "ice",
            name: "Ice house",
            blurb: "Two digits on one pad, and nothing known about either.",
            spawn: "ice",
            needs: &[C1],
            blocked_by: "the sign locker",
            done: SIGIL,
            kind: SiteKind::Boards,
            notes: &[
                "Two digits, one pad, and no known picture this time.",
                "Stack the two rubbings. The pad cancels, and the two digits lie on top of each other.",
                "Add one digit stencil at a time to the stack. When what is left is a clean digit, you have found both.",
            ],
        },
        Site {
            id: "backgate",
            name: "Market back gate",
            blurb: "Its board shows a pattern nobody could guess.",
            spawn: "backgate",
            needs: &[C1],
            blocked_by: "the sign locker",
            done: GATE_M,
            kind: SiteKind::Boards,
            notes: &[
                "You cannot flip a difference into a picture you do not know.",
                "This board hangs off the market console. You have met its pad before.",
                "Reader 1: a fresh rubbing of this board (it blanks it). Reader 2: the market pad (notice rubbing + CREST). Reader 3: OPEN.",
            ],
        },
        Site {
            id: "shed",
            name: "Harbour master's shed",
            blurb: "The tide board, on the mast's console.",
            spawn: "shed",
            needs: &[GATE_M],
            blocked_by: "the market back gate",
            done: PAD_MAST,
            kind: SiteKind::Boards,
            notes: &["The tide board shows the anchor whenever the harbour is open. It is open.", "It hangs off the mast's console.", "Rub the tide board and stack it with ANCHOR. Cut that card: it is the mast's pad."],
        },
        Site {
            id: "mole",
            name: "Mole gate",
            blurb: "Sealed. A supply drone is let through every half minute.",
            spawn: "mole",
            needs: &[CAM_OFF],
            blocked_by: "the camera over the mole",
            done: GATE_W,
            kind: SiteKind::Boards,
            notes: &[
                "Change one lamp of the picture and the bottom row no longer matches. That is the seal.",
                "The seal cannot tell today's order from one it has already seen.",
                "Rub the board while the drone has it open. Rub it again when it is shut. Slot both rubbings: shut cancels, open remains.",
            ],
        },
        Site {
            id: "mast",
            name: "Harbour mast",
            blurb: "The Directorate's voice over the Wharf. Its pattern changes before you can learn it.",
            spawn: "mast",
            needs: &[GATE_W],
            blocked_by: "the mole gate",
            done: MAST,
            kind: SiteKind::Boards,
            notes: &[
                "The picture changes before you can learn it. You do not need to learn it.",
                "A rubbing of the board erases whatever it shows right now. Then draw what you want, through the pad.",
                "Fresh rubbing, mast pad and NIGHTJAR, all three in the readers before the pattern changes. Pull.",
            ],
        },
    ]
}

// --- a player who knows the answers (tests and the demo) -----------------------------------

/// One thing a player does.
#[derive(Clone, Debug, PartialEq)]
pub enum Move {
    Pick(&'static str),
    Slot(&'static str, CardRef),
    Unslot(&'static str, usize),
    Toggle(&'static str, usize, usize),
    Copy(&'static str),
    Cut(Vec<CardRef>),
    Pull(&'static str),
    Dial(&'static str, Vec<u8>),
    /// nothing to do yet (waiting for a broadcast)
    Wait,
}

impl Move {
    /// The board this move happens at, if any.
    pub fn board(&self) -> Option<&'static str> {
        match self {
            Move::Slot(b, _) | Move::Unslot(b, _) | Move::Toggle(b, _, _) | Move::Copy(b) | Move::Pull(b) => Some(b),
            _ => None,
        }
    }
}

impl Signals {
    fn own(&self, st: &GameState, bits: Bits) -> Option<CardRef> {
        st.data.cards.iter().find(|c| c.value() == bits).map(|c| CardRef::Own(c.id))
    }

    fn named(&self, st: &GameState, name: &str) -> Option<CardRef> {
        st.data.cards.iter().find(|c| c.name == name).map(|c| CardRef::Own(c.id))
    }

    /// Make a board's readers hold exactly `want` (one step at a time).
    fn arrange(&self, board: &'static str, want: &[CardRef]) -> Option<Move> {
        let b = self.board(board)?;
        if let Some(i) = b.cards.iter().position(|c| !want.contains(c)) {
            return Some(Move::Unslot(board, i));
        }
        want.iter().find(|c| !b.cards.contains(c)).map(|c| Move::Slot(board, *c))
    }

    /// Flip lamps by hand until the lock would read `target`.
    fn flip_to(&self, st: &GameState, board: &'static str, target: Glyph) -> Option<Move> {
        let diff = self.lock_view(st, board) ^ target.bits();
        if diff == 0 {
            return None;
        }
        let bit = diff.trailing_zeros() as usize;
        Some(Move::Toggle(board, bit % 8, bit / 8))
    }

    /// The next thing to do at a site, for a player who knows every answer.
    /// `None` when the site is finished.
    pub fn next_move(&self, st: &GameState, site: &str) -> Option<Move> {
        let s = self.site(site)?;
        if st.flag(s.done) {
            return None;
        }
        let base = |id: &str| self.board(id).map(|b| b.base).unwrap_or(0);
        match site {
            "pier" => {
                if !st.flag(GULL) {
                    return Some(Move::Pick("gull"));
                }
                let (n, g) = (self.named(st, "NIGHTJAR")?, self.named(st, "GULL")?);
                for (id, flag, want) in [("a1", "w.a1", vec![n]), ("a2", "w.a2", vec![n, g]), ("a3", "w.a3", vec![g])] {
                    if !st.flag(flag) {
                        return self.arrange(id, &want);
                    }
                }
                Some(Move::Dial("kit", self.locker("kit")?.code.clone()))
            }
            "quay" => Some(self.flip_to(st, "q", Glyph::Open).unwrap_or(Move::Pull("q"))),
            "rig" => Some(self.flip_to(st, "b0", Glyph::Open).unwrap_or(Move::Pull("b0"))),
            "customs" => Some(self.flip_to(st, "b1", Glyph::Open).unwrap_or(Move::Pull("b1"))),
            "patrol" | "camera" => {
                let (id, from, to) = if site == "patrol" { ("b2", Glyph::East, Glyph::West) } else { ("b3", Glyph::Eye, Glyph::Shut) };
                let Some(card) = self.own(st, from.bits() ^ to.bits()) else { return Some(Move::Cut(vec![CardRef::Glyph(from), CardRef::Glyph(to)])) };
                Some(self.arrange(id, &[card]).unwrap_or(Move::Pull(id)))
            }
            "market" => {
                let Some(n) = self.own(st, base("c1n")) else { return Some(Move::Copy("c1n")) };
                let Some(m) = self.own(st, base("c1m")) else { return Some(Move::Copy("c1m")) };
                let day = self.board("c1m")?.picture;
                let Some(sign) = self.own(st, day) else { return Some(Move::Cut(vec![n, CardRef::Glyph(Glyph::Crest), m])) };
                Some(self.arrange("c1l", &[sign]).unwrap_or(Move::Pull("c1l")))
            }
            "ice" => {
                let Some(a) = self.own(st, base("c2a")) else { return Some(Move::Copy("c2a")) };
                let Some(b) = self.own(st, base("c2b")) else { return Some(Move::Copy("c2b")) };
                if self.own(st, base("c2a") ^ base("c2b")).is_none() {
                    return Some(Move::Cut(vec![a, b]));
                }
                Some(Move::Dial("ice", self.locker("ice")?.code.clone()))
            }
            "backgate" => {
                let pad = self.board("c3")?.pad;
                let Some(p) = self.own(st, pad) else {
                    let Some(n) = self.own(st, base("c1n")) else { return Some(Move::Copy("c1n")) };
                    return Some(Move::Cut(vec![n, CardRef::Glyph(Glyph::Crest)]));
                };
                let Some(c) = self.own(st, base("c3")) else { return Some(Move::Copy("c3")) };
                Some(self.arrange("c3", &[c, p, CardRef::Glyph(Glyph::Open)]).unwrap_or(Move::Pull("c3")))
            }
            "mole" => {
                let b = self.board("d1")?;
                let (open, shut) = (b.encode(Glyph::Open.bits()), b.encode(Glyph::Closed.bits()));
                let is_open = b.picture == Glyph::Open.bits();
                let Some(o) = self.own(st, open) else { return Some(if is_open && b.cards.is_empty() && b.hand == 0 { Move::Copy("d1") } else { Move::Wait }) };
                if is_open {
                    return Some(Move::Wait);
                }
                let Some(c) = self.own(st, shut) else { return Some(if b.cards.is_empty() && b.hand == 0 { Move::Copy("d1") } else { Move::Wait }) };
                Some(self.arrange("d1", &[o, c]).unwrap_or(Move::Pull("d1")))
            }
            "shed" => {
                let Some(c) = self.own(st, base("tide")) else { return Some(Move::Copy("tide")) };
                Some(Move::Cut(vec![c, CardRef::Glyph(Glyph::Anchor)]))
            }
            "mast" => {
                let b = self.board("mast")?;
                let p = self.own(st, b.pad)?;
                let fresh = self.own(st, b.base);
                // start only with time to finish before the pattern changes
                let left = b.feed.until_change(self.t).unwrap_or(99.0);
                let Some(c) = fresh else {
                    if let Some(i) = b.cards.iter().position(|c| matches!(c, CardRef::Own(_)) && *c != p) {
                        return Some(Move::Unslot("mast", i));
                    }
                    return Some(if left > crate::campaign::mast_period(st) * 0.6 { Move::Copy("mast") } else { Move::Wait });
                };
                Some(self.arrange("mast", &[c, p, CardRef::Glyph(Glyph::Sigil)]).unwrap_or(Move::Pull("mast")))
            }
            _ => None,
        }
    }

    /// Carry out a move. Returns false if it could not be done.
    pub fn apply(&mut self, st: &mut GameState, m: &Move) -> bool {
        let ok = match m {
            Move::Pick(w) => self.pick(st, w),
            Move::Slot(b, c) => self.slot(st, b, *c),
            Move::Unslot(b, i) => self.unslot(st, b, *i).is_some(),
            Move::Toggle(b, x, y) => self.toggle(st, b, *x, *y),
            Move::Copy(b) => self.copy(st, b).is_some(),
            Move::Cut(refs) => self.cut(st, refs).is_some(),
            Move::Pull(b) => !matches!(self.pull(st, b), super::Outcome::NoLock),
            Move::Dial(l, d) => self.dial(st, l, d),
            Move::Wait => true,
        };
        self.note_pads(st);
        ok
    }
}

#[cfg(test)]
mod tests {
    use super::super::{ALARM, BUZZ, Outcome, SEAL_BROKEN};
    use super::*;

    fn fresh(seed: &str) -> (Signals, GameState) {
        let mut st = GameState::new();
        st.new_game("t", 0, Some(seed));
        let mut sig = Signals::wharf(&st);
        sig.start(&mut st);
        (sig, st)
    }

    /// Play every site in map order with the answers in hand.
    fn play(sig: &mut Signals, st: &mut GameState, stop_before: Option<&str>) {
        let order: Vec<&'static str> = sig.sites.iter().filter(|s| s.kind == SiteKind::Boards).map(|s| s.id).collect();
        for site in order {
            if Some(site) == stop_before {
                return;
            }
            assert!(sig.site_open(st, site), "{site} should be open by now");
            let mut steps = 0;
            while let Some(m) = sig.next_move(st, site) {
                if m == Move::Wait {
                    sig.tick(st, 0.5);
                } else {
                    assert!(sig.apply(st, &m), "{site}: {m:?} failed");
                }
                steps += 1;
                assert!(steps < 400, "{site} does not finish");
            }
            assert!(sig.site_done(st, site), "{site} should be done");
        }
    }

    #[test]
    fn the_district_can_be_freed_and_nothing_is_open_early() {
        for seed in ["wharf-1", "wharf-2", "wharf-3"] {
            let (mut sig, mut st) = fresh(seed);
            assert_eq!(st.data.cards.len(), 1, "you start with Grandpa's card");
            for s in &sig.sites {
                let open = sig.site_open(&st, s.id);
                assert_eq!(open, s.needs.is_empty(), "{} open at the start: {open}", s.id);
            }
            play(&mut sig, &mut st, None);
            assert!(st.flag(MAST));
            assert!(st.trace < 1.0, "the clean route raises no trace ({})", st.trace);
        }
    }

    #[test]
    fn every_save_has_its_own_noise_and_answers() {
        let (a, _) = fresh("one");
        let (b, _) = fresh("two");
        assert_ne!(a.board("a1").unwrap().base, b.board("a1").unwrap().base);
        assert_ne!(a.board("mast").unwrap().pad, b.board("mast").unwrap().pad);
        let (c, _) = fresh("one");
        assert_eq!(a.board("b1").unwrap().base, c.board("b1").unwrap().base);
        assert_eq!(a.locker("kit").unwrap().code, c.locker("kit").unwrap().code);
    }

    #[test]
    fn a_plain_picture_on_an_encrypted_lock_is_noise_and_costs_trace() {
        let (mut sig, mut st) = fresh("forge");
        play(&mut sig, &mut st, Some("customs"));
        // drawing OPEN in clear: the lock decrypts it into garbage
        let diff = sig.shown(&st, "b1") ^ Glyph::Open.bits();
        for bit in 0..64 {
            if diff >> bit & 1 == 1 {
                assert!(sig.toggle(&mut st, "b1", bit % 8, bit / 8));
            }
        }
        assert_eq!(sig.shown(&st, "b1"), Glyph::Open.bits());
        assert_eq!(sig.pull(&mut st, "b1"), Outcome::NotASignal);
        assert_eq!(st.trace, BUZZ);
        assert!(!st.flag(GATE_C1));
        // the difference between SHUT and OPEN, flipped into the cipher, opens it
        sig.clear(&mut st, "b1");
        let delta = Glyph::Closed.bits() ^ Glyph::Open.bits();
        assert_eq!(delta.count_ones(), 10);
        for bit in 0..64 {
            if delta >> bit & 1 == 1 {
                sig.toggle(&mut st, "b1", bit % 8, bit / 8);
            }
        }
        assert_eq!(sig.pull(&mut st, "b1"), Outcome::Done(Glyph::Open, GATE_C1));
        // ... and the difference between SHUT and ALARM rings the bell
        let (mut sig, mut st) = fresh("forge");
        play(&mut sig, &mut st, Some("customs"));
        let d = sig.cut(&mut st, &[CardRef::Glyph(Glyph::Closed), CardRef::Glyph(Glyph::Alarm)]).unwrap();
        sig.slot(&mut st, "b1", CardRef::Own(d));
        assert_eq!(sig.pull(&mut st, "b1"), Outcome::Alarm(Glyph::Alarm));
        assert_eq!(st.trace, ALARM);
    }

    #[test]
    fn two_boards_on_one_pad_give_each_other_away() {
        let (mut sig, mut st) = fresh("echo");
        play(&mut sig, &mut st, Some("ice"));
        let a = sig.copy(&mut st, "c2a").unwrap();
        let b = sig.copy(&mut st, "c2b").unwrap();
        let both = sig.xor(&st, &[CardRef::Own(a), CardRef::Own(b)]);
        let code = sig.locker("ice").unwrap().code.clone();
        assert_eq!(both, Glyph::of_digit(code[0]).bits() ^ Glyph::of_digit(code[1]).bits(), "the pad is gone");
        // peel: exactly the two right stencils leave a digit behind
        let leaves: Vec<u8> = (0..10u8).filter(|d| Glyph::from_bits(both ^ Glyph::of_digit(*d).bits()).is_some_and(|g| g.digit().is_some())).collect();
        let mut want = code.clone();
        want.sort_unstable();
        assert_eq!(leaves, want);
        // the dial takes them in either order, and a wrong guess is noticed
        assert!(!sig.dial(&mut st, "ice", &[code[0], (code[0] + 1) % 10]) || code[1] == (code[0] + 1) % 10);
        assert!(sig.dial(&mut st, "ice", &[code[1], code[0]]));
        assert!(st.flag(SIGIL));
        assert!(sig.known_glyphs(&st).contains(&Glyph::Sigil));
    }

    #[test]
    fn the_sign_locker_punishes_guessing() {
        let (mut sig, mut st) = fresh("signs");
        play(&mut sig, &mut st, Some("market"));
        let day = Glyph::from_bits(sig.board("c1m").unwrap().picture).unwrap();
        let wrong = SIGNS.iter().copied().find(|g| *g != day).unwrap();
        sig.slot(&mut st, "c1l", CardRef::Glyph(wrong));
        assert_eq!(sig.pull(&mut st, "c1l"), Outcome::Alarm(wrong));
        sig.clear(&mut st, "c1l");
        // the pad card reads the memo in place
        let n = sig.copy(&mut st, "c1n").unwrap();
        let pad = sig.cut(&mut st, &[CardRef::Own(n), CardRef::Glyph(Glyph::Crest)]).unwrap();
        sig.note_pads(&mut st);
        assert!(st.flag(PAD_MARKET));
        sig.slot(&mut st, "c1m", CardRef::Own(pad));
        assert_eq!(sig.shown(&st, "c1m"), day.bits());
        assert!(st.flag("w.c1.read"));
    }

    #[test]
    fn a_seal_stops_forgery_and_replay_walks_past_it() {
        let (mut sig, mut st) = fresh("seal");
        play(&mut sig, &mut st, Some("mole"));
        st.trace = 0.0;
        // the rig's trick: flip SHUT into OPEN
        let delta = sig.cut(&mut st, &[CardRef::Glyph(Glyph::Closed), CardRef::Glyph(Glyph::Open)]).unwrap();
        sig.slot(&mut st, "d1", CardRef::Own(delta));
        assert_eq!(sig.pull(&mut st, "d1"), Outcome::SealBroken);
        assert_eq!(st.trace, SEAL_BROKEN);
        sig.clear(&mut st, "d1");
        // wait for the drone, record, wait, record, replay
        while sig.board("d1").unwrap().picture != Glyph::Open.bits() {
            sig.tick(&mut st, 0.25);
        }
        let open = sig.copy(&mut st, "d1").unwrap();
        while sig.board("d1").unwrap().picture != Glyph::Closed.bits() {
            sig.tick(&mut st, 0.25);
        }
        let shut = sig.copy(&mut st, "d1").unwrap();
        sig.slot(&mut st, "d1", CardRef::Own(open));
        sig.slot(&mut st, "d1", CardRef::Own(shut));
        assert_eq!(sig.pull(&mut st, "d1"), Outcome::Done(Glyph::Open, GATE_W));
    }

    #[test]
    fn the_mast_needs_its_pad_and_a_fresh_rubbing() {
        let (mut sig, mut st) = fresh("mast");
        play(&mut sig, &mut st, Some("mast"));
        st.trace = 0.0;
        let pad = sig.own(&st, sig.board("mast").unwrap().pad).unwrap();
        // a stale rubbing: the pattern has moved on
        let old = sig.copy(&mut st, "mast").unwrap();
        sig.tick(&mut st, MAST_PERIOD + 1.0);
        for c in [CardRef::Own(old), pad, CardRef::Glyph(Glyph::Sigil)] {
            sig.slot(&mut st, "mast", c);
        }
        assert_eq!(sig.pull(&mut st, "mast"), Outcome::NotASignal);
        // a fresh one (rub the bare board, then put all three back)
        sig.clear(&mut st, "mast");
        let new = sig.copy(&mut st, "mast").unwrap();
        for c in [CardRef::Own(new), pad, CardRef::Glyph(Glyph::Sigil)] {
            sig.slot(&mut st, "mast", c);
        }
        assert_eq!(sig.pull(&mut st, "mast"), Outcome::Done(Glyph::Sigil, MAST));
        assert!(sig.watchers(&st).is_empty(), "the guard stands down once the mast is ours");
    }

    #[test]
    fn the_guard_walks_his_loop_and_leaves_when_ordered() {
        let (mut sig, mut st) = fresh("guard");
        let p0 = sig.poses(&st)[0];
        sig.tick(&mut st, 3.0);
        let p1 = sig.poses(&st)[0];
        assert!((p0.x - p1.x).abs() + (p0.y - p1.y).abs() > 20.0);
        assert!(sig.exposure(&st, p1.x, p1.y) > 0.0);
        assert_eq!(sig.exposure(&st, p1.x + 200.0, p1.y), 0.0);
        st.set(GUARD_WEST, true);
        sig.reward(&mut st, GUARD_WEST);
        sig.tick(&mut st, 40.0);
        let p2 = sig.poses(&st)[0];
        assert!(p2.x < t(40.0), "he is in the customs yard now ({})", p2.x / T);
        // the drone shows up once per period
        let mut seen = 0;
        let mut was = false;
        for _ in 0..(DRONE_PERIOD * 2.0 / 0.25) as usize {
            sig.tick(&mut st, 0.25);
            let v = sig.poses(&st)[1].visible;
            if v && !was {
                seen += 1;
            }
            was = v;
        }
        assert!((2..=3).contains(&seen), "one visit per period, saw {seen}");
    }
}
