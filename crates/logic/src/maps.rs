//! Map data: tiles, props, people, spawn points and zones. Pure data; the
//! engine renders it and the story refers to objects by id.

use std::collections::HashMap;

use crate::Facing;
use crate::state::MapId;

pub const T: f32 = 16.0;

const fn t(n: f32) -> f32 {
    n * 16.0
}

#[derive(Clone, Debug, PartialEq)]
pub struct Light {
    pub tex: &'static str,
    pub scale: f32,
    pub alpha: f32,
    pub flicker: bool,
    pub pulse: bool,
    pub dx: f32,
    /// Offset from the object's anchor; `None` means "middle of the sprite".
    pub dy: Option<f32>,
}

impl Light {
    pub fn new(tex: &'static str, scale: f32, alpha: f32, dy: f32) -> Self {
        Self { tex, scale, alpha, flicker: false, pulse: false, dx: 0.0, dy: Some(dy) }
    }

    pub fn pulse(mut self) -> Self {
        self.pulse = true;
        self
    }

    pub fn flicker(mut self) -> Self {
        self.flicker = true;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    pub w: f32,
    pub h: f32,
    pub dy: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapObject {
    pub id: &'static str,
    pub tex: &'static str,
    /// Bottom-centre of the sprite, in pixels.
    pub x: f32,
    pub y: f32,
    pub frame: usize,
    pub anim: bool,
    /// Collision box anchored at the bottom-centre; dy lifts it.
    pub body: Option<Body>,
    pub interact: Option<&'static str>,
    pub label: Option<&'static str>,
    pub radius: f32,
    /// Where the interaction point is, relative to the bottom-centre.
    pub ix: f32,
    pub iy: f32,
    pub floor: bool,
    pub depth: f32,
    /// Flag condition: "a", "!a", or "a,!b" (all must hold).
    pub show_if: Option<&'static str>,
    pub light: Option<Light>,
    pub frame_if: Vec<(&'static str, usize)>,
    pub alpha: f32,
}

impl MapObject {
    pub fn new(id: &'static str, tex: &'static str, x: f32, y: f32) -> Self {
        Self {
            id,
            tex,
            x,
            y,
            frame: 0,
            anim: false,
            body: None,
            interact: None,
            label: None,
            radius: 20.0,
            ix: 0.0,
            iy: -4.0,
            floor: false,
            depth: 0.0,
            show_if: None,
            light: None,
            frame_if: Vec::new(),
            alpha: 1.0,
        }
    }

    pub fn body(mut self, w: f32, h: f32) -> Self {
        self.body = Some(Body { w, h, dy: 0.0 });
        self
    }

    pub fn interact(mut self, id: &'static str, label: &'static str) -> Self {
        self.interact = Some(id);
        self.label = Some(label);
        self
    }

    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r;
        self
    }

    pub fn iy(mut self, iy: f32) -> Self {
        self.iy = iy;
        self
    }

    pub fn anim(mut self) -> Self {
        self.anim = true;
        self
    }

    pub fn floor(mut self) -> Self {
        self.floor = true;
        self
    }

    pub fn depth(mut self, d: f32) -> Self {
        self.depth = d;
        self
    }

    pub fn show_if(mut self, c: &'static str) -> Self {
        self.show_if = Some(c);
        self
    }

    pub fn light(mut self, l: Light) -> Self {
        self.light = Some(l);
        self
    }

    pub fn frame_if(mut self, rules: &[(&'static str, usize)]) -> Self {
        self.frame_if = rules.to_vec();
        self
    }

    pub fn frame(mut self, f: usize) -> Self {
        self.frame = f;
        self
    }

    pub fn alpha(mut self, a: f32) -> Self {
        self.alpha = a;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NpcDef {
    pub id: &'static str,
    pub char: &'static str,
    pub x: f32,
    pub y: f32,
    pub facing: Facing,
    pub interact: Option<&'static str>,
    pub label: Option<&'static str>,
    pub show_if: Option<&'static str>,
    pub wander: Option<Rect>,
    pub hologram: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Zone {
    pub id: &'static str,
    pub rect: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ambient {
    Motes,
    Dust,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CatMode {
    Wander,
    Follow,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spawn {
    pub x: f32,
    pub y: f32,
    pub facing: Facing,
}

#[derive(Clone, Debug)]
pub struct MapDef {
    pub id: MapId,
    pub title: &'static str,
    pub w: usize,
    pub h: usize,
    /// tiles[y][x] by tile name
    pub tiles: Vec<Vec<&'static str>>,
    pub objects: Vec<MapObject>,
    pub npcs: Vec<NpcDef>,
    pub spawns: HashMap<&'static str, Spawn>,
    pub zones: Vec<Zone>,
    pub music: &'static str,
    /// Colour multiplied over the map (0xRRGGBB).
    pub tint: u32,
    pub ambient: Option<Ambient>,
    pub cat_mode: Option<CatMode>,
    pub cat_area: Option<Rect>,
}

impl MapDef {
    pub fn object(&self, id: &str) -> Option<&MapObject> {
        self.objects.iter().find(|o| o.id == id)
    }

    pub fn spawn(&self, name: &str) -> Spawn {
        self.spawns.get(name).or_else(|| self.spawns.get("start")).copied().expect("map has a start spawn")
    }

    pub fn tile(&self, tx: i32, ty: i32) -> &'static str {
        if tx < 0 || ty < 0 || tx >= self.w as i32 || ty >= self.h as i32 {
            return "void";
        }
        self.tiles[ty as usize][tx as usize]
    }

    /// Every interaction id on this map (objects and NPCs).
    pub fn interactions(&self) -> Vec<&'static str> {
        self.objects.iter().filter_map(|o| o.interact).chain(self.npcs.iter().filter_map(|n| n.interact)).collect()
    }
}

pub fn map(id: MapId) -> MapDef {
    match id {
        MapId::Cabin => cabin(),
        MapId::City => city(),
    }
}

// ---------------------------------------------------------------------------
// Grandpa's cabin: one room, rain outside, a fire going. 20 x 13 tiles.
// ---------------------------------------------------------------------------

fn cabin() -> MapDef {
    const W: usize = 20;
    const H: usize = 13;
    let mut g: Vec<Vec<&'static str>> = Vec::with_capacity(H);
    for y in 0..H {
        let mut row = Vec::with_capacity(W);
        for x in 0..W {
            let tile = if y == 0 || y == H - 1 || x == 0 || x == W - 1 {
                "walltop"
            } else if y == 1 || y == 2 {
                if (x == 4 || x == 12) && y == 1 { "logwindow" } else { "logwall" }
            } else if (x * 7 + y * 3) % 5 == 0 {
                "wood2"
            } else {
                "wood"
            };
            row.push(tile);
        }
        g.push(row);
    }
    g[3][9] = "hearth";
    g[3][10] = "hearth";

    let objects = vec![
        MapObject::new("rug", "obj-rug", t(9.5) + 4.0, t(7.0) + 4.0).floor(),
        MapObject::new("bookshelf", "obj-bookshelf", t(2.0), t(3.0) + 4.0).body(30.0, 8.0).interact("cabin.bookshelf", "Browse the shelf").radius(22.0),
        MapObject::new("photo", "obj-photo", t(6.5), t(2.0) + 6.0).depth(-40.0).interact("cabin.photo", "Look at the photo").iy(14.0).radius(20.0),
        MapObject::new("dresser", "obj-dresser", t(6.5), t(3.0) + 10.0).body(22.0, 8.0).interact("cabin.dresser", "Open the drawer").frame_if(&[("pro.letter", 1)]),
        MapObject::new("fireplace", "obj-fireplace", t(10.0), t(3.0) + 6.0)
            .anim()
            .body(30.0, 10.0)
            .interact("cabin.fireplace", "Warm your hands")
            .light(Light::new("glow-fire", 1.6, 0.55, -10.0).flicker()),
        MapObject::new("armchair", "obj-armchair", t(13.0) - 4.0, t(6.0) + 2.0).body(16.0, 8.0).interact("cabin.armchair", "Sit for a moment"),
        MapObject::new("desk", "obj-deskcrt", t(15.5), t(3.0) + 14.0).body(32.0, 12.0).interact("cabin.computer", "Use the computer").frame_if(&[("pro.booted", 1)]),
        MapObject::new("crtglow", "obj-note", t(15.5) - 2.0, t(3.0) + 4.0).alpha(0.0).show_if("pro.booted").light(Light::new("glow-green", 0.8, 0.5, -14.0).pulse()),
        MapObject::new("door", "obj-door", t(18.0) + 4.0, t(3.0)).depth(-30.0).interact("cabin.door", "Open the door").iy(2.0).radius(18.0),
        MapObject::new("coatrack", "obj-coatrack", t(18.0) + 8.0, t(7.0) + 4.0).body(8.0, 5.0).interact("cabin.coat", "Check the coat"),
        MapObject::new("bed", "obj-bed", t(2.0) + 2.0, t(11.0) + 14.0).body(28.0, 28.0).interact("cabin.bed", "Look at the bed").iy(-14.0).radius(26.0),
        MapObject::new("sidetable", "obj-sidetable", t(4.0) + 4.0, t(11.0) + 6.0).body(12.0, 6.0).light(Light::new("glow-warm", 0.9, 0.45, -18.0)),
        MapObject::new("plant", "obj-plant", t(18.0) + 4.0, t(11.0) + 14.0).body(10.0, 6.0).interact("cabin.plant", "Inspect the plant"),
        MapObject::new("boxes", "obj-boxes", t(15.0) + 8.0, t(11.0) + 12.0).body(18.0, 8.0).interact("cabin.boxes", "Open the boxes"),
        MapObject::new("window1", "obj-note", t(4.5), t(2.0) + 14.0).alpha(0.0).interact("cabin.window", "Look outside").radius(18.0),
        MapObject::new("window2", "obj-note", t(12.5), t(2.0) + 14.0).alpha(0.0).interact("cabin.window", "Look outside").radius(18.0),
    ];

    let mut spawns = HashMap::new();
    spawns.insert("start", Spawn { x: t(17.0) + 8.0, y: t(4.0) + 10.0, facing: Facing::Down });
    spawns.insert("computer", Spawn { x: t(15.5), y: t(5.0), facing: Facing::Up });

    MapDef {
        id: MapId::Cabin,
        title: "Grandpa's Cabin",
        w: W,
        h: H,
        tiles: g,
        objects,
        npcs: Vec::new(),
        spawns,
        zones: Vec::new(),
        music: "cabin",
        tint: 0xf0e6d2,
        ambient: Some(Ambient::Dust),
        cat_mode: Some(CatMode::Wander),
        cat_area: Some(Rect { x: t(6.0), y: t(5.0), w: t(7.0), h: t(4.0) }),
    }
}

// ---------------------------------------------------------------------------
// Cryptopolis: six districts around Legacy Plaza, cut by four avenues. Each
// district is fenced, with one gate that opens when the previous chapter is
// done. 60 x 44 tiles.
//
//   Masquerade (5)   Observatory (6)   Mint (4)
//   Exchange (2)     Legacy Plaza      Commons (3)
//                 The Wharf (1, you arrive here)
// ---------------------------------------------------------------------------

const CW: usize = 60;
const CH: usize = 44;

type Grid = Vec<Vec<&'static str>>;

fn fill(g: &mut Grid, x: i32, y: i32, w: i32, h: i32, tile: impl Fn(i32, i32) -> &'static str) {
    for yy in y..y + h {
        for xx in x..x + w {
            if yy < 0 || yy >= CH as i32 || xx < 0 || xx >= CW as i32 {
                continue;
            }
            g[yy as usize][xx as usize] = tile(xx, yy);
        }
    }
}

fn solid(name: &'static str) -> impl Fn(i32, i32) -> &'static str {
    move |_, _| name
}

fn h_road(g: &mut Grid, y: i32, x0: i32, x1: i32) {
    fill(g, x0, y, x1 - x0 + 1, 3, |x, _| if (x * 13) % 7 == 0 { "asphalt2" } else { "asphalt" });
    for x in x0..=x1 {
        if x % 2 == 0 {
            g[(y + 1) as usize][x as usize] = "dash_h";
        }
    }
}

fn v_road(g: &mut Grid, x: i32, y0: i32, y1: i32) {
    fill(g, x, y0, 3, y1 - y0 + 1, |_, y| if (y * 11) % 7 == 0 { "asphalt2" } else { "asphalt" });
    for y in y0..=y1 {
        if y % 2 == 0 {
            g[y as usize][(x + 1) as usize] = "dash_v";
        }
    }
}

fn city_tiles() -> Grid {
    let mut g: Grid = vec![vec!["sidewalk"; CW]; CH];
    for y in 0..CH {
        for x in 0..CW {
            if (x * 31 + y * 17) % 23 == 0 {
                g[y][x] = "sidewalk2";
            }
        }
    }
    let (w, h) = (CW as i32, CH as i32);
    // outer edge
    fill(&mut g, 0, 0, w, 1, solid("hedge"));
    fill(&mut g, 0, 0, 1, h, solid("hedge"));
    fill(&mut g, w - 1, 0, 1, h, solid("hedge"));
    // district grounds
    fill(&mut g, 1, 1, 17, 10, |x, y| if (x + y) % 2 == 1 { "plaza2" } else { "plaza" });
    fill(&mut g, 23, 1, 14, 10, solid("metal"));
    fill(&mut g, 42, 1, 16, 10, solid("sidewalk"));
    fill(&mut g, 1, 16, 17, 11, solid("metal"));
    fill(&mut g, 42, 16, 16, 11, |x, y| {
        if (x * 7 + y * 5) % 9 == 0 {
            "grass2"
        } else if (x * 3 + y * 11) % 17 == 0 {
            "flowers"
        } else {
            "grass"
        }
    });
    // a path through the park
    fill(&mut g, 42, 21, 16, 2, solid("sand"));
    fill(&mut g, 48, 16, 2, 11, solid("sand"));
    // plaza
    fill(&mut g, 22, 15, 16, 13, |x, y| if (x + y) % 2 == 1 { "plaza2" } else { "plaza" });
    fill(&mut g, 22, 21, 16, 1, solid("plazaglow"));
    fill(&mut g, 29, 15, 2, 13, solid("plazaglow"));
    // avenues
    h_road(&mut g, 12, 1, 58);
    h_road(&mut g, 28, 1, 58);
    v_road(&mut g, 19, 12, 30);
    v_road(&mut g, 38, 12, 30);
    // crosswalks where avenues meet
    for (x, y) in [(19, 12), (38, 12), (19, 28), (38, 28)] {
        fill(&mut g, x, y, 3, 3, solid("asphalt"));
        fill(&mut g, x - 1, y, 1, 3, solid("cross_v"));
        fill(&mut g, x + 3, y, 1, 3, solid("cross_v"));
        fill(&mut g, x, y - 1, 3, 1, solid("cross_h"));
        fill(&mut g, x, y + 3, 3, 1, solid("cross_h"));
    }
    // fences and hedges around gated districts (gaps are the gates)
    fill(&mut g, 1, 11, 17, 1, solid("fence")); // Masquerade south
    fill(&mut g, 18, 1, 1, 11, solid("fence")); // Masquerade east
    fill(&mut g, 22, 1, 1, 11, solid("fence")); // Observatory west
    fill(&mut g, 37, 1, 1, 11, solid("fence")); // Observatory east
    fill(&mut g, 22, 11, 16, 1, solid("fence")); // Observatory south
    fill(&mut g, 41, 1, 1, 11, solid("fence")); // Mint west
    fill(&mut g, 41, 11, 18, 1, solid("fence")); // Mint south
    fill(&mut g, 1, 15, 18, 1, solid("fence")); // Exchange north
    fill(&mut g, 1, 27, 18, 1, solid("fence")); // Exchange south
    fill(&mut g, 18, 15, 1, 13, solid("fence")); // Exchange east
    fill(&mut g, 41, 15, 18, 1, solid("hedge")); // Commons north
    fill(&mut g, 41, 27, 18, 1, solid("hedge")); // Commons south
    fill(&mut g, 41, 15, 1, 13, solid("hedge")); // Commons west
    // gates
    fill(&mut g, 9, 11, 2, 1, solid("sidewalk"));
    fill(&mut g, 29, 11, 2, 1, solid("metal"));
    fill(&mut g, 48, 11, 2, 1, solid("sidewalk"));
    fill(&mut g, 18, 21, 1, 2, solid("metal"));
    fill(&mut g, 41, 21, 1, 2, solid("sand"));
    // the waterfront
    fill(&mut g, 1, 37, 58, 1, solid("dock_h"));
    fill(&mut g, 0, 38, w, 1, solid("wateredge"));
    fill(&mut g, 0, 39, w, 5, solid("water"));
    fill(&mut g, 33, 38, 3, 6, solid("dock_v"));
    fill(&mut g, 10, 38, 3, 4, solid("dock_v"));
    fill(&mut g, 0, 37, 1, 1, solid("hedge"));
    fill(&mut g, w - 1, 37, 1, 1, solid("hedge"));
    g
}

fn lamp(id: &'static str, x: f32, y: f32, tex: &'static str) -> MapObject {
    MapObject::new(id, "obj-lamp", x, y).body(6.0, 4.0).light(Light::new(tex, 1.1, 0.45, -34.0))
}

fn barrier(dir: char, id: &'static str, gate: &'static str, x: f32, y: f32, cond: &'static str) -> MapObject {
    let tex = if dir == 'h' { "obj-barrier_h" } else { "obj-barrier_v" };
    MapObject::new(id, tex, x, y)
        .anim()
        .body(16.0, 16.0)
        .show_if(cond)
        .interact(gate, "Inspect the barrier")
        .light(Light::new("glow-magenta", 0.5, 0.5, -8.0).pulse())
}

fn pylon(id: &'static str, x: f32, y: f32) -> MapObject {
    MapObject::new(id, "obj-pylon", x, y).anim().body(8.0, 6.0)
}

fn kiosk(id: &'static str, x: f32, y: f32, interact: &'static str) -> MapObject {
    MapObject::new(id, "obj-kiosk", x, y).anim().body(14.0, 6.0).interact(interact, "Use the terminal").light(Light::new("glow-cyan", 0.45, 0.4, -22.0))
}

fn npc(id: &'static str, char: &'static str, x: f32, y: f32, facing: Facing, interact: &'static str, label: Option<&'static str>) -> NpcDef {
    NpcDef { id, char, x, y, facing, interact: Some(interact), label, show_if: None, wander: None, hologram: false }
}

fn city() -> MapDef {
    let objects = vec![
        // --- The Wharf ---------------------------------------------------------
        MapObject::new("gatepad", "obj-gatepad", t(30.0), t(35.0) + 4.0).floor().anim().light(Light::new("glow-cyan", 1.0, 0.5, -10.0).pulse()).interact("wharf.gatepad", "Step on the pad").iy(-8.0),
        MapObject::new("sign-wharf", "sign-wharf", t(30.0), t(32.0) + 2.0).anim().light(Light::new("glow-cyan", 0.7, 0.35, -6.0)),
        MapObject::new("radioshack", "bld-radioshack", t(6.0) + 8.0, t(35.0) + 6.0).anim().body(80.0, 30.0),
        MapObject::new("sign-radio", "sign-radio", t(6.0) + 8.0, t(33.0) - 4.0).depth(2.0).anim(),
        MapObject::new("antenna", "obj-antenna", t(2.0) + 6.0, t(35.0) + 4.0).anim().body(8.0, 6.0),
        MapObject::new("radio", "obj-radio", t(11.0) + 8.0, t(36.0) + 2.0).anim().body(24.0, 8.0).interact("wharf.radio", "Use Mara's radio"),
        kiosk("kiosk-wharf", t(14.0), t(36.0) + 2.0, "wharf.kiosk"),
        MapObject::new("billboard", "obj-billboard", t(22.0), t(35.0) + 4.0).anim().body(60.0, 6.0).interact("wharf.billboard", "Read the billboard").light(Light::new("glow-red", 1.0, 0.35, -28.0)),
        MapObject::new("boathouse", "bld-boathouse", t(47.0), t(35.0) + 8.0).anim().body(128.0, 34.0),
        MapObject::new("sign-boathouse", "sign-boathouse", t(47.0), t(33.0) - 2.0).depth(2.0).anim(),
        kiosk("kiosk-boathouse", t(53.0) + 4.0, t(36.0) + 2.0, "wharf.boathouse"),
        MapObject::new("boat", "obj-boat", t(41.0), t(41.0) + 8.0).anim(),
        MapObject::new("crate1", "obj-crate", t(37.0), t(36.0) + 8.0).body(16.0, 8.0),
        MapObject::new("crate2", "obj-crate", t(38.0) + 2.0, t(36.0) + 8.0).body(16.0, 8.0),
        MapObject::new("barrel1", "obj-barrel", t(39.0) + 6.0, t(36.0) + 8.0).body(12.0, 8.0),
        MapObject::new("crate3", "obj-crate", t(26.0), t(36.0) + 8.0).body(16.0, 8.0),
        lamp("lamp-w1", t(18.0), t(32.0) + 8.0, "glow-cyan"),
        lamp("lamp-w2", t(25.0), t(32.0) + 8.0, "glow-cyan"),
        lamp("lamp-w3", t(35.0), t(32.0) + 8.0, "glow-cyan"),
        lamp("lamp-w4", t(42.0), t(32.0) + 8.0, "glow-cyan"),
        lamp("lamp-w5", t(56.0), t(33.0), "glow-cyan"),
        MapObject::new("cone1", "obj-cone", t(16.0), t(36.0) + 8.0),
        MapObject::new("hydrant", "obj-hydrant", t(33.0), t(32.0) + 6.0).body(8.0, 4.0),
        // --- Legacy Plaza ------------------------------------------------------
        MapObject::new("vault", "obj-vault", t(30.0), t(23.0) + 8.0)
            .body(46.0, 16.0)
            .interact("plaza.vault", "Examine the Vault")
            .frame_if(&[("ch2.done", 1), ("vault.opened", 2)])
            .light(Light::new("glow-cyan", 1.6, 0.35, -30.0).pulse()),
        MapObject::new("sign-plaza", "sign-plaza", t(30.0), t(16.0) + 4.0).anim().light(Light::new("glow-cyan", 0.8, 0.3, -6.0)),
        MapObject::new("projector", "obj-projector", t(26.0), t(18.0) + 6.0).anim().body(14.0, 4.0),
        lamp("lamp-p1", t(23.0), t(16.0) + 8.0, "glow-cyan"),
        lamp("lamp-p2", t(37.0), t(16.0) + 8.0, "glow-cyan"),
        lamp("lamp-p3", t(23.0), t(27.0) + 4.0, "glow-cyan"),
        lamp("lamp-p4", t(37.0), t(27.0) + 4.0, "glow-cyan"),
        MapObject::new("bench-p1", "obj-bench", t(25.0), t(25.0) + 8.0).body(30.0, 6.0).interact("plaza.bench", "Sit down"),
        MapObject::new("bench-p2", "obj-bench", t(35.0), t(25.0) + 8.0).body(30.0, 6.0).interact("plaza.bench", "Sit down"),
        MapObject::new("statue", "obj-statue", t(34.0), t(19.0) + 4.0).body(18.0, 8.0).interact("plaza.statue", "Read the plaque"),
        MapObject::new("trash-p", "obj-trash", t(24.0), t(19.0)).body(10.0, 4.0),
        // --- The Exchange (chapter 2) ------------------------------------------
        MapObject::new("exchange", "bld-exchange", t(9.0) + 8.0, t(21.0) + 4.0).anim().body(160.0, 34.0),
        MapObject::new("sign-exchange", "sign-exchange", t(9.0) + 8.0, t(19.0) - 2.0).depth(2.0).anim(),
        MapObject::new("dish", "obj-dish", t(16.0) + 4.0, t(19.0) + 8.0).body(20.0, 8.0),
        MapObject::new("antenna2", "obj-antenna", t(2.0), t(21.0)).anim().body(8.0, 6.0),
        MapObject::new("server1", "obj-server", t(3.0), t(25.0) + 8.0).anim().body(16.0, 6.0),
        MapObject::new("server2", "obj-server", t(4.0) + 2.0, t(25.0) + 8.0).anim().body(16.0, 6.0),
        kiosk("kiosk-exchange", t(15.0), t(24.0) + 8.0, "exchange.kiosk"),
        barrier('v', "gate-exchange-1", "gate.exchange", t(18.0) + 8.0, t(22.0), "!ch1.done"),
        barrier('v', "gate-exchange-2", "gate.exchange", t(18.0) + 8.0, t(23.0), "!ch1.done"),
        pylon("pylon-ex-1", t(18.0) + 8.0, t(21.0)),
        pylon("pylon-ex-2", t(18.0) + 8.0, t(24.0) + 2.0),
        // --- The Commons (chapter 3) -------------------------------------------
        MapObject::new("pavilion", "bld-pavilion", t(53.0), t(19.0) + 6.0).anim().body(96.0, 26.0),
        MapObject::new("sign-commons", "sign-commons", t(53.0), t(17.0) + 4.0).depth(2.0).anim(),
        MapObject::new("dropA", "obj-brickdrop", t(44.0) + 4.0, t(17.0) + 8.0).body(32.0, 8.0).interact("commons.dropA", "Check the loose brick").frame_if(&[("ch3.dropA", 1)]),
        MapObject::new("dropB", "obj-tree", t(56.0), t(26.0) + 4.0).frame(1).body(10.0, 8.0).interact("commons.dropB", "Look into the hollow").iy(-10.0),
        MapObject::new("mailbox", "obj-mailbox", t(51.0), t(25.0) + 8.0).body(10.0, 5.0).interact("commons.mailbox", "Open the mailbox"),
        MapObject::new("tree-c1", "obj-tree", t(43.0) + 8.0, t(26.0) + 8.0).body(10.0, 8.0),
        MapObject::new("tree-c2", "obj-tree", t(57.0), t(20.0)).body(10.0, 8.0),
        MapObject::new("tree-c3", "obj-tree", t(46.0), t(26.0) + 4.0).body(10.0, 8.0),
        MapObject::new("bush-c1", "obj-bush", t(51.0), t(23.0) + 6.0).body(14.0, 5.0),
        MapObject::new("bench-c1", "obj-bench", t(45.0) + 8.0, t(20.0) + 6.0).body(30.0, 6.0).interact("plaza.bench", "Sit down"),
        kiosk("kiosk-commons", t(46.0), t(24.0), "commons.kiosk"),
        lamp("lamp-c1", t(42.0) + 8.0, t(23.0) + 8.0, "glow-green"),
        barrier('v', "gate-commons-1", "gate.commons", t(41.0) + 8.0, t(22.0), "!ch2.done"),
        barrier('v', "gate-commons-2", "gate.commons", t(41.0) + 8.0, t(23.0), "!ch2.done"),
        pylon("pylon-co-1", t(41.0) + 8.0, t(21.0)),
        pylon("pylon-co-2", t(41.0) + 8.0, t(24.0) + 2.0),
        // --- The Mint (chapter 4) ----------------------------------------------
        MapObject::new("mint", "bld-mint", t(49.0) + 8.0, t(6.0) + 8.0).anim().body(160.0, 34.0),
        MapObject::new("sign-mint", "sign-mint", t(49.0) + 8.0, t(4.0) + 2.0).depth(2.0).anim().light(Light::new("glow-gold", 0.7, 0.3, -6.0)),
        MapObject::new("coin", "obj-coin", t(44.0), t(4.0)).anim().depth(2.0),
        MapObject::new("archive", "obj-archive", t(56.0), t(9.0) + 8.0).body(32.0, 10.0).interact("mint.archive", "Examine the sealed door").frame_if(&[("ch4.archive", 1)]),
        MapObject::new("server3", "obj-server", t(43.0), t(9.0) + 8.0).anim().body(16.0, 6.0),
        MapObject::new("server4", "obj-server", t(44.0) + 2.0, t(9.0) + 8.0).anim().body(16.0, 6.0),
        MapObject::new("vending-m", "obj-vending", t(53.0) + 8.0, t(9.0) + 8.0).anim().body(16.0, 6.0).interact("mint.vending", "Use the vending machine"),
        kiosk("kiosk-mint", t(46.0) + 8.0, t(10.0), "mint.kiosk"),
        barrier('h', "gate-mint-1", "gate.mint", t(48.0) + 8.0, t(12.0), "!vault.opened"),
        barrier('h', "gate-mint-2", "gate.mint", t(49.0) + 8.0, t(12.0), "!vault.opened"),
        pylon("pylon-mi-1", t(47.0) + 8.0, t(12.0)),
        pylon("pylon-mi-2", t(50.0) + 8.0, t(12.0)),
        // --- The Masquerade (chapter 5) ------------------------------------------
        MapObject::new("masks-hall", "bld-masks", t(9.0) + 8.0, t(6.0) + 4.0).anim().body(160.0, 34.0),
        MapObject::new("sign-masks", "sign-masks", t(9.0) + 8.0, t(4.0) - 2.0).depth(2.0).anim().light(Light::new("glow-magenta", 0.8, 0.35, -6.0)),
        MapObject::new("masks-deco", "obj-masks", t(4.0), t(5.0) - 2.0).depth(2.0),
        MapObject::new("checkpoint", "obj-checkpoint", t(9.0) + 8.0, t(8.0) + 10.0)
            .anim()
            .body(48.0, 8.0)
            .show_if("!ch5.passed")
            .interact("masks.checkpoint", "Approach the checkpoint")
            .light(Light::new("glow-red", 0.8, 0.35, -16.0).pulse()),
        kiosk("kiosk-masks", t(15.0), t(9.0) + 8.0, "masks.kiosk"),
        lamp("lamp-m1", t(3.0), t(9.0) + 8.0, "glow-violet"),
        lamp("lamp-m2", t(16.0) + 8.0, t(7.0), "glow-violet"),
        barrier('h', "gate-masks-1", "gate.masks", t(9.0) + 8.0, t(12.0), "!ch4.done"),
        barrier('h', "gate-masks-2", "gate.masks", t(10.0) + 8.0, t(12.0), "!ch4.done"),
        pylon("pylon-ma-1", t(8.0) + 8.0, t(12.0)),
        pylon("pylon-ma-2", t(11.0) + 8.0, t(12.0)),
        // --- The Observatory (chapter 6) -----------------------------------------
        MapObject::new("observatory", "bld-observatory", t(30.0), t(7.0) + 8.0).anim().body(176.0, 40.0),
        MapObject::new("sign-observatory", "sign-observatory", t(30.0), t(5.0) + 2.0).depth(2.0).anim().light(Light::new("glow-blue", 0.8, 0.35, -6.0)),
        MapObject::new("telescope", "obj-telescope", t(34.0) + 8.0, t(10.0)).body(14.0, 6.0).interact("obs.telescope", "Look through the telescope"),
        MapObject::new("whiteboard", "obj-whiteboard", t(25.0) + 8.0, t(10.0)).body(36.0, 6.0).interact("obs.whiteboard", "Read the whiteboard"),
        kiosk("kiosk-obs", t(28.0), t(10.0) + 4.0, "obs.kiosk"),
        barrier('h', "gate-obs-1", "gate.obs", t(29.0) + 8.0, t(12.0), "!ch5.done"),
        barrier('h', "gate-obs-2", "gate.obs", t(30.0) + 8.0, t(12.0), "!ch5.done"),
        pylon("pylon-ob-1", t(28.0) + 8.0, t(12.0)),
        pylon("pylon-ob-2", t(31.0) + 8.0, t(12.0)),
        // --- the avenues -----------------------------------------------------------
        lamp("lamp-a1", t(18.0), t(16.0) + 6.0, "glow-cyan"),
        lamp("lamp-a2", t(41.0), t(16.0) + 6.0, "glow-green"),
        MapObject::new("vending-a", "obj-vending", t(23.0) + 8.0, t(30.0) + 14.0).anim().body(16.0, 6.0).interact("street.vending", "Use the vending machine"),
        MapObject::new("stall", "obj-stall", t(35.0), t(30.0) + 14.0).body(30.0, 8.0).interact("street.stall", "Browse the stall"),
    ];

    let wander = |x: f32, y: f32, w: f32, h: f32| Some(Rect { x, y, w, h });
    let mut npcs = vec![
        npc("mara", "mara", t(12.0) + 8.0, t(35.0) + 4.0, Facing::Down, "mara.talk", Some("Talk to Mara")),
        npc("felix", "felix", t(13.0), t(23.0) + 8.0, Facing::Down, "felix.talk", Some("Talk to Felix")),
        npc("juno", "juno", t(48.0) + 8.0, t(20.0), Facing::Down, "juno.talk", Some("Talk to Juno")),
        npc("ibrahim", "ibrahim", t(48.0), t(9.0) + 4.0, Facing::Down, "ibrahim.talk", Some("Talk to Ibrahim")),
        npc("lin", "lin", t(13.0) + 8.0, t(9.0) + 8.0, Facing::Down, "lin.talk", Some("Talk to Lin")),
        NpcDef { show_if: Some("!ch5.passed"), ..npc("agent", "agent", t(6.0) + 8.0, t(9.0), Facing::Right, "masks.agent", Some("Talk to the agent")) },
        npc("ada", "ada", t(32.0), t(9.0) + 8.0, Facing::Down, "ada.talk", Some("Talk to Ada")),
        NpcDef { hologram: true, show_if: Some("warden.on"), ..npc("warden", "warden", t(26.0), t(18.0) + 2.0, Facing::Down, "warden.holo", Some("Watch the broadcast")) },
    ];
    let citizens = [
        ("c1", "citizen1", t(20.0), t(34.0), Facing::Down, "citizen.1", (t(16.0), t(32.0), t(18.0), t(4.0))),
        ("c2", "citizen2", t(40.0), t(33.0), Facing::Left, "citizen.2", (t(36.0), t(32.0), t(16.0), t(3.0))),
        ("c3", "citizen3", t(32.0), t(26.0), Facing::Up, "citizen.3", (t(23.0), t(24.0), t(13.0), t(3.0))),
        ("c4", "citizen4", t(50.0), t(23.0), Facing::Down, "citizen.4", (t(43.0), t(21.0), t(13.0), t(3.0))),
        ("c5", "citizen5", t(8.0), t(13.0) + 8.0, Facing::Right, "citizen.5", (t(3.0), t(16.0) + 8.0, t(12.0), t(5.0))),
    ];
    for (id, ch, x, y, f, inter, (wx, wy, ww, wh)) in citizens {
        npcs.push(NpcDef { wander: wander(wx, wy, ww, wh), ..npc(id, ch, x, y, f, inter, None) });
    }

    let zone = |id: &'static str, x: f32, y: f32, w: f32, h: f32| Zone { id, rect: Rect { x: t(x), y: t(y), w: t(w), h: t(h) } };
    let zones = vec![
        zone("wharf", 1.0, 31.0, 58.0, 7.0),
        zone("plaza", 22.0, 15.0, 16.0, 13.0),
        zone("exchange", 1.0, 16.0, 17.0, 11.0),
        zone("commons", 42.0, 16.0, 16.0, 11.0),
        zone("mint", 42.0, 1.0, 16.0, 10.0),
        zone("masks", 1.0, 1.0, 17.0, 10.0),
        zone("observatory", 23.0, 1.0, 14.0, 10.0),
    ];

    let mut spawns = HashMap::new();
    spawns.insert("start", Spawn { x: t(30.0), y: t(35.0) + 12.0, facing: Facing::Down });
    spawns.insert("gateway", Spawn { x: t(30.0), y: t(35.0) + 12.0, facing: Facing::Down });
    spawns.insert("vault", Spawn { x: t(30.0), y: t(25.0) + 4.0, facing: Facing::Up });

    MapDef {
        id: MapId::City,
        title: "Cryptopolis",
        w: CW,
        h: CH,
        tiles: city_tiles(),
        objects,
        npcs,
        spawns,
        zones,
        music: "city",
        tint: 0x6e7cb0,
        ambient: Some(Ambient::Motes),
        cat_mode: Some(CatMode::Follow),
        cat_area: None,
    }
}

/// Fixed points in the city used by cutscenes.
pub const WARDEN_APPEAR: (f32, f32) = (t(26.0), t(18.0) + 2.0);
pub const VAULT_FRONT: (f32, f32) = (t(30.0), t(25.0) + 4.0);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_are_consistent() {
        for id in [MapId::Cabin, MapId::City] {
            let m = map(id);
            assert_eq!(m.tiles.len(), m.h);
            assert!(m.tiles.iter().all(|r| r.len() == m.w));
            for row in &m.tiles {
                for t in row {
                    assert!(cpl_art::tiles::tile_index(t).is_some(), "unknown tile {t}");
                }
            }
            let sprites: Vec<String> = cpl_art::objects::all_sprites().into_iter().map(|(k, _)| k).collect();
            for o in &m.objects {
                assert!(sprites.contains(&o.tex.to_string()), "unknown sprite {}", o.tex);
            }
            let mut ids: Vec<&str> = m.objects.iter().map(|o| o.id).collect();
            ids.sort();
            ids.dedup();
            assert_eq!(ids.len(), m.objects.len(), "duplicate object ids on {:?}", id);
        }
    }
}
