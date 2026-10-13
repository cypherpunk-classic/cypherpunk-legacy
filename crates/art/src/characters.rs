//! Characters are composed from parts: head + torso + legs per facing, then a
//! hairstyle and accessories are painted on top, then the silhouette gets an
//! automatic outline. Frames are 16x24; feet touch the bottom row.
//!
//! Slot letters: H/G hair, S/T skin, E eyes, C/D top, P/Q trousers, B shoes,
//! A/K accents, V visor glow.

use crate::canvas::{Canvas, PaintOpts, Rgba};
use crate::palette::{OUTLINE, SKIN_BROWN, SKIN_DEEP, SKIN_FAIR, SKIN_LIGHT, SKIN_TAN};

pub const FRAME_W: i32 = 16;
pub const FRAME_H: i32 = 24;

const HEAD_FRONT: [&str; 12] = [
    "................",
    ".....SSSSSS.....",
    "...SSSSSSSSSS...",
    "..SSSSSSSSSSSS..",
    "..SSSSSSSSSSSS..",
    "..SSSSSSSSSSSS..",
    "..SSSSSSSSSSSS..",
    "..SSSESSSSESSS..",
    "..SSSESSSSESSS..",
    "..TSSSSSSSSSST..",
    "...TSSSSSSSST...",
    "....TTTTTTTT....",
];

const HEAD_SIDE: [&str; 12] = [
    "................",
    ".....SSSSSS.....",
    "...SSSSSSSSSS...",
    "..SSSSSSSSSSSS..",
    "..SSSSSSSSSSSS..",
    "..SSSSSSSSSSSS..",
    "..SSSSSSSSSSSS..",
    "..SSESSSSSSSSS..",
    ".SSSESSSSSSSSS..",
    "..SSSSSSSSSSST..",
    "...TSSSSSSSST...",
    "....TTTTTTTT....",
];

const BODY_FRONT: [&str; 6] = [".....CCCCCC.....", "....CCCCCCCC....", "...SCCCCCCCCS...", "...SCCCCCCCCS...", "...SDCCCCCCDS...", "....DDDDDDDD...."];
const BODY_BACK: [&str; 6] = [".....CCCCCC.....", "....CCCCCCCC....", "...SCCCCCCCCS...", "...SCCCCCCCCS...", "...SDDCCCCDDS...", "....DDDDDDDD...."];
const BODY_SIDE: [&str; 6] = [".....CCCCCC.....", ".....CCCCCCC....", ".....CSCCCCD....", ".....CSCCCCD....", ".....DSDCCCD....", ".....DDDDDDD...."];

const LEGS_FRONT: [[&str; 6]; 3] = [
    [".....PPPPPP.....", ".....PPQQPP.....", ".....PP..PP.....", ".....PP..PP.....", ".....BB..BB.....", "................"],
    [".....PPPPPP.....", ".....PPQQPP.....", ".....PP..PP.....", ".....PP..BB.....", ".....BB.........", "................"],
    [".....PPPPPP.....", ".....PPQQPP.....", ".....PP..PP.....", ".....BB..PP.....", ".........BB.....", "................"],
];

const LEGS_SIDE: [[&str; 6]; 3] = [
    ["......PPPPP.....", "......PPPPP.....", "......PP.PP.....", "......PP.PP.....", ".....BBB.BB.....", "................"],
    ["......PPPPP.....", ".....PPP.PPP....", "....PPP...PP....", "....PP.....PP...", "...BBB.....BB...", "................"],
    ["......PPPPP.....", "......PPPP......", ".......PPP......", ".......PP.......", "......BBB.......", "................"],
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Facing {
    Down,
    Up,
    Left,
    Right,
}

pub const FACINGS: [Facing; 4] = [Facing::Down, Facing::Up, Facing::Left, Facing::Right];

impl Facing {
    pub fn index(self) -> usize {
        FACINGS.iter().position(|f| *f == self).unwrap()
    }

    pub fn name(self) -> &'static str {
        match self {
            Facing::Down => "down",
            Facing::Up => "up",
            Facing::Left => "left",
            Facing::Right => "right",
        }
    }

    pub fn parse(s: &str) -> Option<Facing> {
        FACINGS.iter().copied().find(|f| f.name() == s)
    }
}

/// Overlays: per base facing (down, up, left), rows painted over the body; '.' leaves pixels alone.
#[derive(Clone, Copy, Default)]
pub struct Overlay {
    pub down: &'static [&'static str],
    pub up: &'static [&'static str],
    pub left: &'static [&'static str],
}

impl Overlay {
    fn rows(&self, base: Facing) -> &'static [&'static str] {
        match base {
            Facing::Down => self.down,
            Facing::Up => self.up,
            _ => self.left,
        }
    }
}

pub fn hair(name: &str) -> Overlay {
    match name {
        "short" => Overlay {
            down: &["....HHHHHHHH....", "...HHHHHHHHHH...", "..HHHHHHHHHHHH..", "..HHHHHHHHHHHH..", "..HHGHHHHHHGHH..", "..HG.GGHHGG.GH..", "..H..........H.."],
            up: &["....HHHHHHHH....", "...HHHHHHHHHH...", "..HHHHHHHHHHHH..", "..HHHHHHHHHHHH..", "..HHHHHHHHHHHH..", "..HHHHHHHHHHHH..", "..HHHHHHHHHHHH..", "..GHHHHHHHHHHG..", "..GGHHHHHHHHGG..", "...GGGGGGGGGG..."],
            left: &[".....HHHHHH.....", "...HHHHHHHHHH...", "..HHHHHHHHHHHH..", "..HHHHHHHHHHHH..", "..GHHHHHHHHHHH..", "..G...GHHHHHHH..", "......GHHHHHHH..", ".......THHHHHH..", ".......THHHHHG..", "........GHHHG...", ".........GGG...."],
        },
        "long" => Overlay {
            down: &["....HHHHHHHH....", "...HHHHHHHHHH...", "..HHHHHHHHHHHH..", ".HHHHHHHHHHHHHH.", ".HHHGHHHHHHGHHH.", ".HHG.GGHHGG.GHH.", ".HH..........HH.", ".HH..........HH.", ".HH..........HH.", ".HG..........GH.", ".GG..........GG.", ".G............G."],
            up: &["....HHHHHHHH....", "...HHHHHHHHHH...", "..HHHHHHHHHHHH..", ".HHHHHHHHHHHHHH.", ".HHHHHHHHHHHHHH.", ".HHHHHHHHHHHHHH.", ".HHHHHHHHHHHHHH.", ".HHHHHHHHHHHHHH.", ".HHHHHHHHHHHHHH.", ".GHHHHHHHHHHHHG.", ".GGHHHHHHHHHHGG.", ".GGGGGGGGGGGGGG.", "...GGGGGGGGGG..."],
            left: &[".....HHHHHH.....", "...HHHHHHHHHH...", "..HHHHHHHHHHHH..", ".HHHHHHHHHHHHHH.", ".GHHHHHHHHHHHHH.", "..G...GHHHHHHHH.", "......GHHHHHHHH.", ".......THHHHHHH.", ".......THHHHHHH.", "........HHHHHHH.", "........GHHHHHG.", "........GGHHHGG.", ".........GGGGG..", "..........GGG..."],
        },
        "bob" => Overlay {
            down: &["....HHHHHHHH....", "...HHHHHHHHHH...", "..HHHHHHHHHHHH..", ".HHHHHHHHHHHHHH.", ".HHHHHHHHHHHHHH.", ".HHGGGGGGGGGGHH.", ".HH..........HH.", ".HH..........HH.", ".HG..........GH.", ".GG..........GG."],
            up: &["....HHHHHHHH....", "...HHHHHHHHHH...", "..HHHHHHHHHHHH..", ".HHHHHHHHHHHHHH.", ".HHHHHHHHHHHHHH.", ".HHHHHHHHHHHHHH.", ".HHHHHHHHHHHHHH.", ".HHHHHHHHHHHHHH.", ".GHHHHHHHHHHHHG.", ".GGGGGGGGGGGGGG."],
            left: &[".....HHHHHH.....", "...HHHHHHHHHH...", "..HHHHHHHHHHHH..", ".HHHHHHHHHHHHHH.", ".GHHHHHHHHHHHHH.", ".GG...GHHHHHHHH.", "......GHHHHHHHH.", ".......THHHHHHH.", ".......GHHHHHHG.", "........GGGGGG.."],
        },
        "bald" => Overlay {
            down: &["................", "................", "................", "..HH........HH..", "..HH........HH..", "..H..........H.."],
            up: &["................", "................", "................", "..HH........HH..", "..HHHHHHHHHHHH..", "..HHHHHHHHHHHH..", "..GHHHHHHHHHHG..", "...GGGGGGGGGG..."],
            left: &["................", "................", "................", "..........HHH...", ".........HHHH...", ".......THHHHH...", ".......TGHHH....", ".........GG....."],
        },
        "spiky" => Overlay {
            down: &["...H.HH.HH.H....", "...HHHHHHHHHH...", "..HHHHHHHHHHHH..", ".HHHHHHHHHHHHHH.", "..HHHGHHHHGHHH..", "..HGG.GHHG.GGH..", "..H..........H.."],
            up: &["...H.HH.HH.H....", "...HHHHHHHHHH...", "..HHHHHHHHHHHH..", ".HHHHHHHHHHHHHH.", "..HHHHHHHHHHHH..", "..HHHHHHHHHHHH..", "..HHHHHHHHHHHH..", "..GHHHHHHHHHHG..", "..GGHHHHHHHHGG..", "...GGGGGGGGGG..."],
            left: &["....H.HH.HH.H...", "...HHHHHHHHHHH..", "..HHHHHHHHHHHHH.", "..HHHHHHHHHHHHH.", "..GHHHHHHHHHHH..", "..G...GHHHHHHH..", "......GHHHHHHH..", ".......THHHHHH..", ".......THHHHHG..", "........GHHHG..."],
        },
        "bun" => Overlay {
            down: &["......HHHH......", "....HHGGGGHH....", "...HHHHHHHHHH...", "..HHHHHHHHHHHH..", "..HHGHHHHHHGHH..", "..HG.GGHHGG.GH..", "..H..........H.."],
            up: &["......HHHH......", ".....HHGGHH.....", "...HHHHHHHHHH...", "..HHHHHHHHHHHH..", "..HHHHHHHHHHHH..", "..HHHHHHHHHHHH..", "..HHHHHHHHHHHH..", "..GHHHHHHHHHHG..", "..GGHHHHHHHHGG..", "...GGGGGGGGGG..."],
            left: &[".........HHHH...", ".....HHHHGGGHH..", "...HHHHHHHHHHH..", "..HHHHHHHHHHHH..", "..GHHHHHHHHHHH..", "..G...GHHHHHHH..", "......GHHHHHHH..", ".......THHHHHH..", ".......THHHHHG..", "........GHHHG..."],
        },
        "hood" => Overlay {
            down: &["....CCCCCCCC....", "...CCCCCCCCCC...", "..CCCCCCCCCCCC..", ".CCCHHHHHHHHCCC.", ".CCHHGHHHHGHHCC.", ".CCG.GGHHGG.GCC.", ".CC..........CC.", ".CC..........CC.", ".CD..........DC.", ".DD..........DD.", "..D..........D.."],
            up: &["....CCCCCCCC....", "...CCCCCCCCCC...", "..CCCCCCCCCCCC..", ".CCCCCCCCCCCCCC.", ".CCCCCCCCCCCCCC.", ".CCCCCCCCCCCCCC.", ".CCCCCCCCCCCCCC.", ".CCCCCCCCCCCCCC.", ".DCCCCCCCCCCCCD.", ".DDCCCCCCCCCCDD.", "..DDDDDDDDDDDD.."],
            left: &[".....CCCCCC.....", "...CCCCCCCCCCC..", "..CCCCCCCCCCCCC.", ".CHHHHCCCCCCCCC.", ".CGHHHHCCCCCCCC.", ".CG...GCCCCCCCC.", ".C.....CCCCCCCC.", ".D......CCCCCCC.", "........DCCCCCC.", ".........DCCCCD.", ".........DDCCDD.", "..........DDDD.."],
        },
        "cap" => Overlay {
            down: &["....AAAAAAAA....", "...AAAAAAAAAA...", "..AAAAAAAAAAAA..", "..AAAAKKAAAAAA..", ".KKKKKKKKKKKKKK.", "..HG........GH..", "..H..........H.."],
            up: &["....AAAAAAAA....", "...AAAAAAAAAA...", "..AAAAAAAAAAAA..", "..AAAAAAAAAAAA..", "..AAAAAAAAAAAA..", "..HHHHHHHHHHHH..", "..HHHHHHHHHHHH..", "..GHHHHHHHHHHG..", "...GGGGGGGGGG..."],
            left: &[".....AAAAAA.....", "...AAAAAAAAAA...", "..AAAAAAAAAAAA..", "..AAAAAAAAAAAA..", "KKKKKKAAAAAAAA..", "......GHHHHHHH..", ".......THHHHHH..", ".......THHHHHG..", "........GHHHG..."],
        },
        _ => Overlay::default(),
    }
}

pub fn accessory(name: &str) -> Overlay {
    const E: &str = "";
    match name {
        "glasses" => Overlay { down: &[E, E, E, E, E, E, E, "...AAAAAAAAAA...", "...A.AA..A.AA..."], up: &[], left: &[E, E, E, E, E, E, E, "..AAAA..........", "..A.AA.........."] },
        "beard" => Overlay {
            down: &[E, E, E, E, E, E, E, E, E, "..G..........G..", "..GH..HHHH..HG..", "...GHHHHHHHHG...", "....GHHHHHHG....", "......GGGG......"],
            up: &[],
            left: &[E, E, E, E, E, E, E, E, E, "........G.......", "..HHHH.HG.......", "..GHHHHHG.......", "...GHHHG........", "....GGG........."],
        },
        "scarf" => Overlay {
            down: &[E, E, E, E, E, E, E, E, E, E, E, "...AAAAAAAAAA...", "....AAAAAAAA....", ".......KK.......", ".......KK......."],
            up: &[E, E, E, E, E, E, E, E, E, E, E, "...AAAAAAAAAA...", "....AAAAAAAA...."],
            left: &[E, E, E, E, E, E, E, E, E, E, E, "...AAAAAAAAA....", "....AAAAAAAA....", "....KK..........", "....KK.........."],
        },
        "headset" => Overlay {
            down: &[E, E, E, E, E, E, ".A..........A...", ".A..........A...", ".AKK............", "...K............"],
            up: &[E, E, E, E, E, E, ".A............A.", ".A............A."],
            left: &[E, E, E, E, E, E, "........A.......", "........A.......", "..KKKKKKA......."],
        },
        "badge" => Overlay { down: &[E, E, E, E, E, E, E, E, E, E, E, E, E, E, ".........AK....."], up: &[], left: &[] },
        "coat" => Overlay {
            down: &[E, E, E, E, E, E, E, E, E, E, E, E, E, E, E, E, E, "....CCCCCCCC....", "....CDD..DDC...."],
            up: &[E, E, E, E, E, E, E, E, E, E, E, E, E, E, E, E, E, "....CCCCCCCC....", "....CDDDDDDC...."],
            left: &[E, E, E, E, E, E, E, E, E, E, E, E, E, E, E, E, E, ".....CCCCCCC....", ".....CDD.DDC...."],
        },
        "visor" => Overlay { down: &[E, E, E, E, E, E, E, "..1111111111111.", "..11VV1111VV11.."], up: &[], left: &[E, E, E, E, E, E, E, ".1111111........", ".1VV1111........"] },
        _ => Overlay::default(),
    }
}

#[derive(Clone, Debug)]
pub struct CharDef {
    pub id: &'static str,
    pub name: &'static str,
    pub hair: &'static str,
    pub accessories: &'static [&'static str],
    pub palette: Vec<(char, Rgba)>,
}

fn pal(skin: (&str, &str), hair: (&str, &str), top: (&str, &str), pants: (&str, &str), shoes: &str, accent: (&str, &str)) -> Vec<(char, Rgba)> {
    let h = Rgba::hex;
    vec![
        ('S', h(skin.0)),
        ('T', h(skin.1)),
        ('H', h(hair.0)),
        ('G', h(hair.1)),
        ('C', h(top.0)),
        ('D', h(top.1)),
        ('P', h(pants.0)),
        ('Q', h(pants.1)),
        ('B', h(shoes)),
        ('E', h("#1a1c2c")),
        ('A', h(accent.0)),
        ('K', h(accent.1)),
        ('V', h("#ff5566")),
    ]
}

const CYAN_ACCENT: (&str, &str) = ("#73eff7", "#257179");

pub fn player_looks() -> Vec<CharDef> {
    vec![
        CharDef { id: "player-a", name: "Hoodie", hair: "hood", accessories: &[], palette: pal(SKIN_LIGHT, ("#3b2a20", "#22160f"), ("#2a9d8f", "#1d6d64"), ("#2a2440", "#1a1c2c"), "#e6e2c3", CYAN_ACCENT) },
        CharDef { id: "player-b", name: "Bob", hair: "bob", accessories: &[], palette: pal(SKIN_TAN, ("#1a1c2c", "#0e0b16"), ("#ef7d57", "#b8552f"), ("#29366f", "#1a1c2c"), "#f4f4f4", CYAN_ACCENT) },
        CharDef { id: "player-c", name: "Spikes", hair: "spiky", accessories: &[], palette: pal(SKIN_BROWN, ("#ff3e9a", "#b13e53"), ("#3b5dc9", "#29366f"), ("#333c57", "#1a1c2c"), "#ffcd75", CYAN_ACCENT) },
    ]
}

pub fn cast() -> Vec<CharDef> {
    vec![
        CharDef { id: "grandpa", name: "Grandpa", hair: "bald", accessories: &["beard", "glasses"], palette: pal(SKIN_FAIR, ("#e6e2c3", "#94b0c2"), ("#8b5a2b", "#5c3a21"), ("#566c86", "#333c57"), "#4a2c1a", ("#c7c7c7", "#94b0c2")) },
        CharDef { id: "mara", name: "Mara", hair: "long", accessories: &["coat"], palette: pal(SKIN_TAN, ("#c9ced6", "#8a93a3"), ("#ffcd75", "#d49a3c"), ("#29366f", "#1a1c2c"), "#2b1810", CYAN_ACCENT) },
        CharDef { id: "felix", name: "Felix", hair: "bald", accessories: &["headset", "beard"], palette: pal(SKIN_FAIR, ("#a8703f", "#7a4a28"), ("#ef7d57", "#b8552f"), ("#333c57", "#1a1c2c"), "#1a1c2c", ("#3d4554", "#73eff7")) },
        CharDef { id: "juno", name: "Juno", hair: "bob", accessories: &["scarf"], palette: pal(SKIN_DEEP, ("#1a1c2c", "#0e0b16"), ("#38b764", "#257179"), ("#3d4554", "#1f2330"), "#e6e2c3", ("#ffcd75", "#ef7d57")) },
        CharDef { id: "ibrahim", name: "Ibrahim", hair: "short", accessories: &["beard"], palette: pal(SKIN_BROWN, ("#1a1c2c", "#0e0b16"), ("#b13e53", "#7a2638"), ("#3d4554", "#1f2330"), "#2b1810", CYAN_ACCENT) },
        CharDef { id: "lin", name: "Lin", hair: "long", accessories: &["glasses"], palette: pal(SKIN_LIGHT, ("#1a1c2c", "#0e0b16"), ("#7d4bd6", "#5d275d"), ("#1a1c2c", "#0e0b16"), "#c792ff", ("#c792ff", "#7d4bd6")) },
        CharDef { id: "ada", name: "Ada", hair: "bun", accessories: &["glasses", "coat"], palette: pal(SKIN_FAIR, ("#f4f4f4", "#94b0c2"), ("#e6e2c3", "#94b0c2"), ("#29366f", "#1a1c2c"), "#1a1c2c", ("#41a6f6", "#29366f")) },
        CharDef { id: "warden", name: "Warden", hair: "cap", accessories: &["visor", "badge"], palette: pal(SKIN_FAIR, ("#1a1c2c", "#0e0b16"), ("#333c57", "#1f2330"), ("#1f2330", "#0e0b16"), "#0e0b16", ("#262a36", "#1f2330")) },
        CharDef { id: "agent", name: "DLA Agent", hair: "cap", accessories: &["visor"], palette: pal(SKIN_TAN, ("#1a1c2c", "#0e0b16"), ("#3d4554", "#1f2330"), ("#1f2330", "#0e0b16"), "#0e0b16", ("#3d4554", "#1f2330")) },
        CharDef { id: "citizen1", name: "Citizen", hair: "short", accessories: &[], palette: pal(SKIN_DEEP, ("#2b1810", "#1a1c2c"), ("#41a6f6", "#3b5dc9"), ("#e6e2c3", "#94b0c2"), "#1a1c2c", CYAN_ACCENT) },
        CharDef { id: "citizen2", name: "Citizen", hair: "long", accessories: &[], palette: pal(SKIN_FAIR, ("#ffcd75", "#d49a3c"), ("#ff3e9a", "#b13e53"), ("#1a1c2c", "#0e0b16"), "#f4f4f4", CYAN_ACCENT) },
        CharDef { id: "citizen3", name: "Citizen", hair: "spiky", accessories: &["glasses"], palette: pal(SKIN_TAN, ("#38b764", "#257179"), ("#1a1c2c", "#0e0b16"), ("#566c86", "#333c57"), "#ef7d57", CYAN_ACCENT) },
        CharDef { id: "citizen4", name: "Citizen", hair: "bun", accessories: &[], palette: pal(SKIN_BROWN, ("#5d275d", "#2a2440"), ("#a7f070", "#38b764"), ("#29366f", "#1a1c2c"), "#f4f4f4", CYAN_ACCENT) },
        CharDef { id: "citizen5", name: "Citizen", hair: "hood", accessories: &[], palette: pal(SKIN_LIGHT, ("#7a4a28", "#4a2c1a"), ("#566c86", "#333c57"), ("#2a2440", "#1a1c2c"), "#ffcd75", CYAN_ACCENT) },
    ]
}

/// Every character definition (player looks first).
pub fn all_characters() -> Vec<CharDef> {
    let mut v = player_looks();
    v.extend(cast());
    v
}

/// Head + body + legs for the front (down) and side (left) views.
fn frame_rows(base: Facing, frame: usize) -> Vec<&'static str> {
    let mut rows: Vec<&'static str> = Vec::with_capacity(24);
    if base == Facing::Down {
        rows.extend(HEAD_FRONT);
        rows.extend(BODY_FRONT);
        rows.extend(LEGS_FRONT[frame]);
    } else {
        rows.extend(HEAD_SIDE);
        rows.extend(BODY_SIDE);
        rows.extend(LEGS_SIDE[frame]);
    }
    rows
}

/// Frame order in a character sheet: facing-major, 3 frames each (idle, stepA, stepB).
pub fn frame_index(facing: Facing, frame: usize) -> usize {
    facing.index() * 3 + frame
}

fn paint_body(t: &mut Canvas, base: Facing, frame: usize, slots: &[(char, Rgba)]) {
    let opts = PaintOpts { slots: Some(slots), ..Default::default() };
    if base == Facing::Up {
        // back of the head has no eyes
        let head: Vec<String> = HEAD_FRONT.iter().map(|r| r.replace('E', "S")).collect();
        let head_refs: Vec<&str> = head.iter().map(|s| s.as_str()).collect();
        t.paint(&head_refs, 0, 0, opts);
        t.paint(&BODY_BACK, 0, 12, opts);
        t.paint(&LEGS_FRONT[frame], 0, 18, opts);
    } else {
        t.paint(&frame_rows(base, frame), 0, 0, opts);
    }
}

/// A 12-frame sheet: down, up, left, right x (idle, step A, step B).
pub fn character_sheet(def: &CharDef) -> Canvas {
    let mut sheet = Canvas::new(FRAME_W * 12, FRAME_H);
    let slots = def.palette.as_slice();
    for (fi, facing) in FACINGS.iter().enumerate() {
        let base = if *facing == Facing::Right { Facing::Left } else { *facing };
        let mirror = *facing == Facing::Right;
        for f in 0..3 {
            let x0 = (fi as i32 * 3 + f as i32) * FRAME_W;
            let mut t = Canvas::new(FRAME_W, FRAME_H);
            paint_body(&mut t, base, f, slots);
            let opts = PaintOpts { slots: Some(slots), ..Default::default() };
            let hr = hair(def.hair).rows(base);
            if !hr.is_empty() {
                t.paint(hr, 0, 0, opts);
            }
            for acc in def.accessories {
                let ov = accessory(acc).rows(base);
                if !ov.is_empty() {
                    t.paint(ov, 0, 0, opts);
                }
            }
            t.outline(OUTLINE, None);
            sheet.draw(&t, x0, 0, mirror);
        }
    }
    sheet
}

/// A portrait cropped from the front-facing idle frame, scaled up.
pub fn portrait(def: &CharDef, scale: i32) -> Canvas {
    character_sheet(def).crop(0, 0, 16, 15).scaled(scale)
}

// ---------------------------------------------------------------------------
// Nonce the cat (16x16): sit, walk A, walk B.
// ---------------------------------------------------------------------------

fn cat_palette() -> Vec<(char, Rgba)> {
    [('H', "#3d4554"), ('G', "#1f2330"), ('S', "#f4f4f4"), ('E', "#a7f070"), ('A', "#ff3e9a"), ('K', "#ffcd75")].iter().map(|(k, v)| (*k, Rgba::hex(v))).collect()
}

const CAT_SIDE: [[&str; 16]; 3] = [
    [
        "................", "................", "................", "..H...H.........", "..HH.HH.........", "..HHHHH.........", "..HEHEH.........", "..HHSHH.........",
        "...AAAHHHHHH..H.", "...HHHHHHHHHH.H.", "...HHHHHHHHHHHH.", "...HHHHHHHHHH...", "...HHGHHHHGHH...", "...HH.H..H.HH...", "................", "................",
    ],
    [
        "................", "................", "................", "..H...H.........", "..HH.HH.........", "..HHHHH.........", "..HEHEH.........", "..HHSHH.........",
        "...AAAHHHHHH...H", "...HHHHHHHHHH.H.", "...HHHHHHHHHHHH.", "...HHHHHHHHHH...", "..HHG.HHHH.GHH..", "..H.......H.....", "................", "................",
    ],
    [
        "................", "................", "................", "..H...H.........", "..HH.HH.........", "..HHHHH.........", "..HEHEH.........", "..HHSHH.........",
        "...AAAHHHHHH..H.", "...HHHHHHHHHHH..", "...HHHHHHHHHH...", "...HHHHHHHHHH...", "....HGHHHHGH....", "....H.H..H.H....", "................", "................",
    ],
];

const CAT_DOWN: [&str; 16] = [
    "................", "................", "....H....H......", "....HH..HH......", "....HHHHHH......", "....HEHHEH......", "....HHSSHH......", ".....AAAA.......",
    "....HHHHHH......", "...HHHHHHHH.....", "...HHHSSHHH.....", "...HHHSSHHH.H...", "...HHHHHHHHHH...", "....HH..HH......", "................", "................",
];

const CAT_UP: [&str; 16] = [
    "................", "................", "....H....H......", "....HH..HH......", "....HHHHHH......", "....HHHHHH......", "....HHHHHH......", ".....AAAA.......",
    "....HHHHHH......", "...HHHHHHHH.....", "...HHHHHHHH.....", "...HHHHHHHH.....", "...HHHHHHHH.....", "....HH..HH......", ".......HH.......", "........H.......",
];

/// Cat sheet: [down0, up0, left0, left1, left2, right0, right1, right2]
pub fn cat_sheet() -> Canvas {
    let pal = cat_palette();
    let mut frames: Vec<(&[&str], bool)> = vec![(&CAT_DOWN, false), (&CAT_UP, false)];
    for r in &CAT_SIDE {
        frames.push((r, false));
    }
    for r in &CAT_SIDE {
        frames.push((r, true));
    }
    let mut sheet = Canvas::new(16 * frames.len() as i32, 16);
    for (i, (rows, mirror)) in frames.iter().enumerate() {
        let mut t = Canvas::new(16, 16);
        t.paint(rows, 0, 0, PaintOpts { slots: Some(&pal), ..Default::default() });
        t.outline(OUTLINE, None);
        sheet.draw(&t, i as i32 * 16, 0, *mirror);
    }
    sheet
}
