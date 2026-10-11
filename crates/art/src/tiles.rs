//! Ground and wall tiles (16x16), painted procedurally. The tileset is one
//! row-major sheet; maps refer to tiles by name and [`tile_index`] resolves the
//! frame number. Tiles listed in [`SOLID`] block movement.

use crate::canvas::{Canvas, Prng};
use crate::palette::c;

pub const T: i32 = 16;
pub const TILESET_COLS: i32 = 16;

fn noise(g: &mut Canvas, x: i32, y: i32, seed: u32, colors: &[&str], density: f64) {
    let mut r = Prng::new(seed);
    for j in 0..T {
        for i in 0..T {
            if r.next() < density {
                let col = colors[r.below(colors.len() as i32) as usize];
                g.px(x + i, y + j, col);
            }
        }
    }
}

fn wood_floor(g: &mut Canvas, x: i32, y: i32, variant: i32) {
    g.rect(x, y, T, T, c('l'));
    let mut r = Prng::new(11 + variant as u32);
    for row in 0..4 {
        let yy = y + row * 4;
        g.rect(x, yy + 3, T, 1, c('k'));
        let seam = (row * 7 + variant * 5) % 16;
        g.rect(x + seam, yy, 1, 3, c('k'));
        for _ in 0..5 {
            let px = x + r.below(16);
            let py = yy + r.below(3);
            let col = if r.next() < 0.5 { c('m') } else { "#6b4022" };
            g.px(px, py, col);
        }
    }
}

fn log_wall(g: &mut Canvas, x: i32, y: i32) {
    g.rect(x, y, T, T, c('m'));
    for row in 0..4 {
        let yy = y + row * 4;
        g.rect(x, yy, T, 1, "#b97f49");
        g.rect(x, yy + 2, T, 1, c('l'));
        g.rect(x, yy + 3, T, 1, c('k'));
    }
    g.px(x + 3, y + 1, c('l'));
    g.px(x + 11, y + 9, c('l'));
    g.px(x + 7, y + 13, c('l'));
}

fn log_wall_window(g: &mut Canvas, x: i32, y: i32) {
    log_wall(g, x, y);
    // bright overcast daylight outside: sky, distant pines, soft rain
    g.rect(x + 2, y + 2, 12, 11, c('k'));
    g.rect(x + 3, y + 3, 10, 9, "#bcd4e6");
    g.rect(x + 3, y + 3, 10, 3, "#d6e6f0");
    g.rect(x + 3, y + 8, 10, 4, "#5f8f5a");
    let mut r = Prng::new(3);
    for _ in 0..6 {
        let px = x + 3 + r.below(10);
        let py = y + 8 + r.below(4);
        let col = if r.next() < 0.5 { "#4a7a48" } else { "#72a86a" };
        g.px(px, py, col);
    }
    for _ in 0..5 {
        let px = x + 4 + r.below(9);
        let py = y + 3 + r.below(8);
        g.px(px, py, "#e8f2f8");
    }
    g.rect(x + 7, y + 3, 1, 9, "#6b4a2a"); // muntins
    g.rect(x + 3, y + 7, 10, 1, "#6b4a2a");
    g.rect(x + 1, y + 13, 14, 2, c('n')); // sill
    g.rect(x + 1, y + 14, 14, 1, c('m'));
}

fn wall_top(g: &mut Canvas, x: i32, y: i32) {
    g.rect(x, y, T, T, c('j'));
    g.rect(x, y + T - 2, T, 2, c('k'));
    noise(g, x, y, 5, &["#3a2216", "#24140c"], 0.08);
}

fn hearth(g: &mut Canvas, x: i32, y: i32) {
    g.rect(x, y, T, T, "#4b4f5c");
    for (a, b, w, h) in [(0, 0, 7, 5), (8, 0, 8, 5), (0, 6, 4, 4), (5, 6, 6, 4), (12, 6, 4, 4), (0, 11, 9, 5), (10, 11, 6, 5)] {
        g.rect(x + a, y + b, w - 1, h - 1, "#5f6b7d");
        g.px(x + a, y + b, "#7a8699");
    }
}

fn door(g: &mut Canvas, x: i32, y: i32) {
    log_wall(g, x, y);
    g.rect(x + 2, y, 12, T, c('j'));
    g.rect(x + 3, y + 1, 10, T - 1, c('l'));
    for i in 0..3 {
        g.rect(x + 3 + i * 4, y + 1, 1, T - 1, c('k'));
    }
    g.px(x + 11, y + 9, c('e'));
}

fn asphalt(g: &mut Canvas, x: i32, y: i32, v: u32) {
    g.rect(x, y, T, T, c('t'));
    noise(g, x, y, 21 + v, &["#262a38", "#191c27", "#2b3040"], 0.18);
}

fn road_dash(g: &mut Canvas, x: i32, y: i32, vertical: bool) {
    asphalt(g, x, y, 0);
    if vertical {
        g.rect(x + 7, y + 3, 2, 10, "#c9b458");
    } else {
        g.rect(x + 3, y + 7, 10, 2, "#c9b458");
    }
}

fn crosswalk(g: &mut Canvas, x: i32, y: i32, vertical: bool) {
    asphalt(g, x, y, 1);
    for i in (1..16).step_by(4) {
        if vertical {
            g.rect(x + 1, y + i, 14, 2, "#c7ccd6");
        } else {
            g.rect(x + i, y + 1, 2, 14, "#c7ccd6");
        }
    }
}

fn sidewalk(g: &mut Canvas, x: i32, y: i32, v: u32) {
    g.rect(x, y, T, T, "#4a5263");
    g.rect(x, y, T, 1, "#596274");
    g.rect(x, y, 1, T, "#596274");
    g.rect(x + 8, y, 1, T, "#3d4554");
    g.rect(x, y + 8, T, 1, "#3d4554");
    noise(g, x, y, 31 + v, &["#535c6e", "#444c5c"], 0.1);
    if v == 1 {
        g.px(x + 3, y + 4, "#2f3644");
        g.px(x + 4, y + 5, "#2f3644");
        g.px(x + 5, y + 5, "#2f3644");
    }
}

fn plaza(g: &mut Canvas, x: i32, y: i32, v: u32) {
    g.rect(x, y, T, T, if v == 0 { "#1d2236" } else { "#22283f" });
    g.rect(x, y, T, 1, "#2c3452");
    g.rect(x, y, 1, T, "#2c3452");
    noise(g, x, y, 41 + v, &["#262d47", "#1a1f33"], 0.08);
}

fn plaza_glow(g: &mut Canvas, x: i32, y: i32) {
    plaza(g, x, y, 0);
    g.rect(x, y + 7, T, 2, "#1f6b78");
    g.rect(x + 7, y, 2, T, "#1f6b78");
    g.rect(x + 7, y + 7, 2, 2, c('a'));
}

fn grass(g: &mut Canvas, x: i32, y: i32, v: u32) {
    g.rect(x, y, T, T, "#1f4a33");
    noise(g, x, y, 51 + v, &["#2a5e3c", "#173326", "#2f6b44"], 0.35);
    if v == 1 {
        for (a, b) in [(3, 4), (10, 2), (6, 11), (13, 12)] {
            g.px(x + a, y + b, "#3f8a55");
            g.px(x + a, y + b - 1, "#3f8a55");
        }
    }
}

fn flowers(g: &mut Canvas, x: i32, y: i32) {
    grass(g, x, y, 0);
    for (a, b, col) in [(3, 3, c('i')), (11, 5, c('e')), (6, 10, c('a')), (12, 12, c('i')), (2, 13, c('*'))] {
        g.px(x + a, y + b, col);
        g.px(x + a, y + b + 1, "#2f6b44");
    }
}

fn water(g: &mut Canvas, x: i32, y: i32, frame: i32) {
    g.rect(x, y, T, T, "#11203a");
    let mut r = Prng::new(61);
    for i in 0..6 {
        let wx = r.below(12);
        let wy = r.below(15);
        g.rect(x + (wx + frame * 2) % 13, y + wy, 3, 1, if i % 2 == 1 { "#1e3a63" } else { "#24497a" });
    }
    if frame == 1 {
        g.px(x + 9, y + 4, "#3e6fa8");
    } else {
        g.px(x + 4, y + 10, "#3e6fa8");
    }
}

fn water_edge(g: &mut Canvas, x: i32, y: i32) {
    water(g, x, y, 0);
    g.rect(x, y, T, 3, "#4a5263");
    g.rect(x, y + 3, T, 1, "#8fb8d8");
    for i in (0..16).step_by(3) {
        g.px(x + i, y + 4, "#5d86b0");
    }
}

fn dock(g: &mut Canvas, x: i32, y: i32, vertical: bool) {
    g.rect(x, y, T, T, c('l'));
    for i in 0..4 {
        if vertical {
            g.rect(x + i * 4 + 3, y, 1, T, c('k'));
        } else {
            g.rect(x, y + i * 4 + 3, T, 1, c('k'));
        }
    }
    noise(g, x, y, 71, &[c('m'), "#6b4022"], 0.06);
    if vertical {
        g.px(x + 1, y + 2, c('s'));
        g.px(x + 9, y + 12, c('s'));
    } else {
        g.px(x + 2, y + 1, c('s'));
        g.px(x + 12, y + 9, c('s'));
    }
}

fn hedge(g: &mut Canvas, x: i32, y: i32) {
    g.rect(x, y, T, T, "#173326");
    noise(g, x, y, 81, &["#2a5e3c", "#1f4a33", "#2f6b44"], 0.6);
    g.rect(x, y + T - 3, T, 3, "#10241a");
}

fn fence(g: &mut Canvas, x: i32, y: i32) {
    sidewalk(g, x, y, 0);
    g.rect(x, y + 4, T, 2, "#5f6b7d");
    g.rect(x, y + 10, T, 2, "#5f6b7d");
    for i in (1..16).step_by(5) {
        g.rect(x + i, y + 2, 2, 12, "#7a8699");
    }
}

fn railing(g: &mut Canvas, x: i32, y: i32) {
    water_edge(g, x, y);
    g.rect(x, y, T, 1, "#94b0c2");
    for i in (1..16).step_by(5) {
        g.rect(x + i, y, 1, 3, "#94b0c2");
    }
}

fn void(g: &mut Canvas, x: i32, y: i32) {
    g.rect(x, y, T, T, "#07080e");
    g.px(x + 8, y + 8, "#141a2e");
}

fn sand(g: &mut Canvas, x: i32, y: i32) {
    g.rect(x, y, T, T, "#3a3346");
    noise(g, x, y, 91, &["#463e55", "#2f2a3a"], 0.3);
}

fn metal(g: &mut Canvas, x: i32, y: i32) {
    g.rect(x, y, T, T, "#262b3b");
    g.rect(x, y, T, 1, "#323a50");
    g.rect(x, y, 1, T, "#323a50");
    for (a, b) in [(2, 2), (13, 2), (2, 13), (13, 13)] {
        g.px(x + a, y + b, "#4a5470");
    }
}

/// Tile names in sheet order.
pub const TILE_NAMES: &[&str] = &[
    "empty", "wood", "wood2", "logwall", "logwindow", "walltop", "hearth", "door", "asphalt", "asphalt2", "dash_h", "dash_v", "cross_h",
    "cross_v", "sidewalk", "sidewalk2", "plaza", "plaza2", "plazaglow", "grass", "grass2", "flowers", "water", "water_b", "wateredge",
    "dock_v", "dock_h", "hedge", "fence", "railing", "void", "sand", "metal",
];

fn paint_tile(g: &mut Canvas, name: &str, x: i32, y: i32) {
    match name {
        "empty" => {}
        "wood" => wood_floor(g, x, y, 0),
        "wood2" => wood_floor(g, x, y, 1),
        "logwall" => log_wall(g, x, y),
        "logwindow" => log_wall_window(g, x, y),
        "walltop" => wall_top(g, x, y),
        "hearth" => hearth(g, x, y),
        "door" => door(g, x, y),
        "asphalt" => asphalt(g, x, y, 0),
        "asphalt2" => asphalt(g, x, y, 1),
        "dash_h" => road_dash(g, x, y, false),
        "dash_v" => road_dash(g, x, y, true),
        "cross_h" => crosswalk(g, x, y, false),
        "cross_v" => crosswalk(g, x, y, true),
        "sidewalk" => sidewalk(g, x, y, 0),
        "sidewalk2" => sidewalk(g, x, y, 1),
        "plaza" => plaza(g, x, y, 0),
        "plaza2" => plaza(g, x, y, 1),
        "plazaglow" => plaza_glow(g, x, y),
        "grass" => grass(g, x, y, 0),
        "grass2" => grass(g, x, y, 1),
        "flowers" => flowers(g, x, y),
        "water" => water(g, x, y, 0),
        "water_b" => water(g, x, y, 1),
        "wateredge" => water_edge(g, x, y),
        "dock_v" => dock(g, x, y, true),
        "dock_h" => dock(g, x, y, false),
        "hedge" => hedge(g, x, y),
        "fence" => fence(g, x, y),
        "railing" => railing(g, x, y),
        "void" => void(g, x, y),
        "sand" => sand(g, x, y),
        "metal" => metal(g, x, y),
        other => panic!("unknown tile {other}"),
    }
}

pub fn tile_index(name: &str) -> Option<usize> {
    TILE_NAMES.iter().position(|n| *n == name)
}

/// Tiles that block movement.
pub const SOLID: &[&str] = &["logwall", "logwindow", "walltop", "door", "water", "water_b", "wateredge", "hedge", "fence", "railing", "void"];

pub fn is_solid(name: &str) -> bool {
    SOLID.contains(&name)
}

/// Paint one named tile into a fresh 16x16 canvas.
pub fn tile(name: &str) -> Canvas {
    let mut g = Canvas::new(T, T);
    paint_tile(&mut g, name, 0, 0);
    g
}

/// The whole tileset as one sheet (TILESET_COLS tiles per row).
pub fn tileset() -> Canvas {
    let rows = (TILE_NAMES.len() as i32 + TILESET_COLS - 1) / TILESET_COLS;
    let mut g = Canvas::new(TILESET_COLS * T, rows * T);
    for (i, name) in TILE_NAMES.iter().enumerate() {
        let i = i as i32;
        paint_tile(&mut g, name, (i % TILESET_COLS) * T, (i / TILESET_COLS) * T);
    }
    g
}
