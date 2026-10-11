//! Props and furniture. Each entry paints one or more frames; the game turns
//! them into sprite sheets named `obj-<key>`.

use std::sync::Arc;

use crate::canvas::{Canvas, Prng, Rgba};
use crate::font::{draw_text, text_width};
use crate::palette::{OUTLINE, c};

pub type DrawFn = Arc<dyn Fn(&mut Canvas, i32) + Send + Sync>;

/// A code-drawn sprite: `draw(canvas, frame)` paints one frame into a w x h canvas.
#[derive(Clone)]
pub struct ObjectArt {
    pub w: i32,
    pub h: i32,
    pub frames: i32,
    pub outline: bool,
    pub draw: DrawFn,
}

fn art(w: i32, h: i32, frames: i32, outline: bool, draw: fn(&mut Canvas, i32)) -> ObjectArt {
    ObjectArt { w, h, frames, outline, draw: Arc::new(draw) }
}

impl ObjectArt {
    /// Frame size including the 1px outline padding.
    pub fn frame_size(&self) -> (i32, i32) {
        let pad = i32::from(self.outline);
        (self.w + 2 * pad, self.h + 2 * pad)
    }

    /// All frames side by side.
    pub fn sheet(&self) -> Canvas {
        let pad = i32::from(self.outline);
        let (fw, fh) = self.frame_size();
        let mut sheet = Canvas::new(fw * self.frames, fh);
        for f in 0..self.frames {
            let mut inner = Canvas::new(self.w, self.h);
            (self.draw)(&mut inner, f);
            let mut tmp = Canvas::new(fw, fh);
            tmp.draw(&inner, pad, pad, false);
            if self.outline {
                tmp.outline(OUTLINE, None);
            }
            sheet.draw(&tmp, f * fw, 0, false);
        }
        sheet
    }
}

// ---------------------------------------------------------------------------
// Cabin
// ---------------------------------------------------------------------------

fn bed(g: &mut Canvas, _f: i32) {
    g.rect(0, 0, 28, 6, c('k')); // headboard
    g.rect(1, 1, 26, 3, c('l'));
    g.rect(1, 6, 26, 27, c('n')); // sheet
    g.rect(4, 7, 9, 6, c('6')); // pillows
    g.rect(15, 7, 9, 6, c('6'));
    g.rect(4, 12, 9, 1, c('5'));
    g.rect(15, 12, 9, 1, c('5'));
    g.rect(1, 15, 26, 18, "#7a2e45"); // quilt
    for y in (15..33).step_by(4) {
        let x0 = 1 + ((y / 4) % 2) * 3;
        for x in (x0..27).step_by(6) {
            g.rect(x, y, 3, 2, "#9c3d5a");
        }
    }
    g.rect(1, 15, 26, 1, "#b8546f");
    g.rect(0, 33, 28, 1, c('k'));
}

fn bookshelf(g: &mut Canvas, _f: i32) {
    g.rect(0, 0, 30, 34, c('k'));
    g.rect(2, 2, 26, 30, c('j'));
    let mut r = Prng::new(7);
    let spines = [c('g'), c('b'), c('e'), "#3b5dc9", c('c'), c('f'), "#7d4bd6", c('n')];
    for shelf in 0..3 {
        let y = 3 + shelf * 10;
        g.rect(2, y + 8, 26, 2, c('l'));
        let mut x = 3;
        while x < 26 {
            let w = 2 + r.below(2);
            let h = 5 + r.below(3);
            if r.next() < 0.12 {
                x += 2;
                continue;
            }
            let col = r.pick(&spines);
            g.rect(x, y + 8 - h, w, h, col);
            g.px(x, y + 8 - h, "#ffffff33");
            x += w;
        }
    }
    g.rect(0, 32, 30, 2, c('j'));
}

fn desk_crt(g: &mut Canvas, f: i32) {
    // desk
    g.rect(0, 14, 32, 10, c('l'));
    g.rect(0, 14, 32, 2, c('m'));
    g.rect(1, 24, 3, 6, c('k'));
    g.rect(28, 24, 3, 6, c('k'));
    g.rect(4, 24, 24, 2, c('k'));
    // CRT
    g.rect(6, 0, 20, 16, "#cfc6a8");
    g.rect(6, 13, 20, 3, "#b3a98a");
    g.rect(8, 2, 16, 10, "#101a14");
    if f == 1 {
        g.rect(9, 3, 9, 1, "#3cff8a");
        g.rect(9, 5, 12, 1, "#2bbf66");
        g.rect(9, 7, 7, 1, "#2bbf66");
        g.rect(9, 9, 2, 1, "#3cff8a");
    } else {
        g.rect(9, 3, 2, 1, "#1d3a29");
    }
    g.px(23, 13, if f == 1 { "#5dff9a" } else { "#7a2e1e" });
    // keyboard
    g.rect(9, 17, 14, 4, "#d8d0b3");
    for i in (10..22).step_by(2) {
        g.rect(i, 18, 1, 2, "#9c9480");
    }
    // sticky note
    g.rect(24, 9, 5, 5, c('z'));
    g.rect(25, 10, 3, 1, "#b8a94a");
    g.rect(25, 12, 2, 1, "#b8a94a");
}

fn dresser(g: &mut Canvas, f: i32) {
    g.rect(0, 0, 22, 22, c('l'));
    g.rect(0, 0, 22, 2, c('m'));
    for i in 0..3 {
        let y = 4 + i * 6;
        g.rect(2, y, 18, 5, c('k'));
        g.rect(3, y + 1, 16, 3, c('m'));
        g.px(11, y + 2, c('e'));
    }
    if f == 1 {
        g.rect(2, 4, 18, 7, c('j'));
        g.rect(4, 5, 9, 4, c('o'));
        g.rect(5, 6, 6, 1, "#8a7a60");
        g.rect(5, 8, 4, 1, "#8a7a60");
    }
    g.rect(1, 22, 3, 2, c('k'));
    g.rect(18, 22, 3, 2, c('k'));
}

fn fireplace(g: &mut Canvas, f: i32) {
    g.rect(0, 0, 32, 34, "#4b4f5c");
    let mut r = Prng::new(3);
    for y in (0..34).step_by(4) {
        let x0 = if (y / 4) % 2 == 1 { 0 } else { 3 };
        for x in (x0..32).step_by(7) {
            let col = if r.next() < 0.5 { "#5f6b7d" } else { "#565e6e" };
            g.rect(x, y, 6, 3, col);
        }
    }
    g.rect(0, 10, 32, 3, c('l')); // mantel
    g.rect(0, 10, 32, 1, c('m'));
    g.rect(6, 15, 20, 17, "#140d0b");
    g.rect(7, 28, 18, 3, c('k')); // logs
    g.rect(9, 27, 14, 2, c('l'));
    for (i, (x, y, w, h)) in [(10, 18, 4, 10), (14, 16, 5, 12), (19, 19, 3, 9)].into_iter().enumerate() {
        let dy = (f + i as i32) % 3;
        g.rect(x, y + dy, w, h - dy, c('y'));
        g.rect(x + 1, y + 3 + dy, w - 2, h - 4 - dy, c('z'));
    }
    g.px(15, 14 + f, c('x'));
    // candles + photo on mantel
    g.rect(3, 6, 2, 4, c('o'));
    g.px(3, 5, c('z'));
    g.rect(24, 4, 6, 6, c('k'));
    g.rect(25, 5, 4, 4, "#94b0c2");
}

fn plant(g: &mut Canvas, _f: i32) {
    g.rect(3, 14, 8, 8, c('q'));
    g.rect(3, 14, 8, 2, "#c06a4a");
    for (x, y, w, h) in [(6, 1, 2, 13), (2, 4, 4, 3), (8, 3, 4, 3), (1, 8, 5, 3), (8, 7, 5, 3), (4, 11, 6, 3)] {
        g.rect(x, y, w, h, c('v'));
        g.px(x, y, "#3f8a55");
    }
}

fn armchair(g: &mut Canvas, _f: i32) {
    g.rect(1, 0, 16, 10, "#5d275d");
    g.rect(0, 6, 4, 12, "#4a1f4a");
    g.rect(14, 6, 4, 12, "#4a1f4a");
    g.rect(4, 9, 10, 8, "#7a3a7a");
    g.rect(4, 9, 10, 1, "#9c4f9c");
    g.rect(1, 18, 2, 2, c('k'));
    g.rect(15, 18, 2, 2, c('k'));
}

fn side_table(g: &mut Canvas, _f: i32) {
    g.rect(1, 12, 12, 4, c('l'));
    g.rect(1, 12, 12, 1, c('m'));
    g.rect(2, 16, 2, 8, c('k'));
    g.rect(10, 16, 2, 8, c('k'));
    g.rect(5, 9, 4, 3, "#5f6b7d"); // lamp base
    g.rect(3, 2, 8, 7, "#e8c39e"); // shade
    g.rect(3, 8, 8, 1, "#c79c6e");
}

fn photo(g: &mut Canvas, _f: i32) {
    g.rect(0, 0, 12, 10, c('m'));
    g.rect(1, 1, 10, 8, "#2a3f66");
    let heads = ["#e8c39e", "#c98a5d", "#9a6440", "#f6d2b0", "#6b4129"];
    let shirts = ["#ef7d57", "#38b764", "#ffcd75", "#3b5dc9", "#b13e53"];
    for i in 0..5 {
        g.px(2 + i as i32 * 2, 4, heads[i]);
        g.px(2 + i as i32 * 2, 6, shirts[i]);
    }
    g.px(6, 2, "#ffcd75");
}

fn rug(g: &mut Canvas, _f: i32) {
    g.rect(2, 0, 44, 30, "#5d275d");
    g.rect(0, 2, 48, 26, "#5d275d");
    g.rect(3, 3, 42, 24, "#7a2e45");
    g.rect(6, 6, 36, 18, "#2a2440");
    g.rect(8, 8, 32, 14, "#7a2e45");
    for i in 0..4 {
        g.rect(14 + i * 6, 13, 3, 3, c('e'));
        g.px(15 + i * 6, 12, c('f'));
    }
    for x in (4..44).step_by(4) {
        g.px(x, 0, "#b8546f");
        g.px(x, 29, "#b8546f");
    }
}

fn coat_rack(g: &mut Canvas, _f: i32) {
    g.rect(5, 2, 2, 24, c('k'));
    g.rect(2, 25, 8, 2, c('k'));
    g.rect(1, 2, 10, 2, c('k'));
    g.rect(1, 4, 5, 10, "#3b5dc9"); // a jacket
    g.rect(2, 13, 3, 3, "#29366f");
    g.rect(7, 3, 4, 6, "#b13e53"); // scarf
}

fn boxes(g: &mut Canvas, _f: i32) {
    g.rect(0, 6, 12, 12, c('m'));
    g.rect(0, 6, 12, 2, c('n'));
    g.rect(5, 6, 2, 12, c('l'));
    g.rect(7, 0, 11, 10, "#b98a55");
    g.rect(7, 0, 11, 2, c('n'));
    g.rect(11, 0, 2, 10, c('l'));
    draw_text(g, "PGP", 8, 4, c('k'), 0);
}

fn note(g: &mut Canvas, _f: i32) {
    g.rect(0, 0, 10, 8, c('z'));
    g.rect(1, 2, 7, 1, "#9c8f3a");
    g.rect(1, 4, 5, 1, "#9c8f3a");
}

// ---------------------------------------------------------------------------
// Cryptopolis
// ---------------------------------------------------------------------------

fn lamp(g: &mut Canvas, _f: i32) {
    g.rect(6, 6, 2, 32, "#3d4554");
    g.rect(4, 37, 6, 3, "#3d4554");
    g.rect(2, 2, 10, 4, "#3d4554");
    g.rect(3, 5, 8, 2, c('a'));
    g.rect(5, 0, 4, 2, "#3d4554");
}

fn bench(g: &mut Canvas, _f: i32) {
    g.rect(0, 0, 30, 4, c('l'));
    g.rect(0, 0, 30, 1, c('m'));
    g.rect(0, 6, 30, 5, c('l'));
    g.rect(0, 6, 30, 1, c('m'));
    g.rect(2, 11, 2, 5, "#3d4554");
    g.rect(26, 11, 2, 5, "#3d4554");
}

fn tree(g: &mut Canvas, f: i32) {
    g.rect(13, 26, 6, 15, "#4a2c1a");
    g.rect(14, 26, 2, 15, "#6b4022");
    let mut r = Prng::new(17);
    for (cx, cy, rad) in [(16, 14, 13), (8, 18, 8), (24, 18, 8), (16, 6, 9), (10, 9, 7), (22, 9, 7)] {
        for y in -rad..=rad {
            for x in -rad..=rad {
                if x * x + y * y > rad * rad {
                    continue;
                }
                let shade = if y > rad / 3 {
                    "#173326"
                } else if r.next() < 0.15 {
                    "#3f8a55"
                } else {
                    "#2a5e3c"
                };
                g.px(cx + x, cy + y, shade);
            }
        }
    }
    if f == 1 {
        // hollow with a dead drop
        g.rect(14, 31, 4, 5, "#140d0b");
        g.px(16, 33, c('e'));
    }
}

fn bush(g: &mut Canvas, _f: i32) {
    let mut r = Prng::new(23);
    for y in 0..14 {
        for x in 0..16 {
            let dx = (x as f64 - 7.5) / 8.0;
            let dy = (y as f64 - 8.0) / 7.0;
            if dx * dx + dy * dy > 1.0 {
                continue;
            }
            let col = if y > 9 {
                "#173326"
            } else if r.next() < 0.2 {
                "#3f8a55"
            } else {
                "#2a5e3c"
            };
            g.px(x, y, col);
        }
    }
}

fn crate_box(g: &mut Canvas, _f: i32) {
    g.rect(0, 0, 16, 16, c('m'));
    g.rect(0, 0, 16, 2, c('n'));
    g.rect(0, 14, 16, 2, c('l'));
    for i in 0..16 {
        g.px(i, i, c('l'));
    }
    g.rect(0, 0, 2, 16, c('l'));
    g.rect(14, 0, 2, 16, c('l'));
}

fn barrel(g: &mut Canvas, _f: i32) {
    g.rect(1, 0, 12, 16, "#2a5e8a");
    g.rect(0, 2, 14, 12, "#2a5e8a");
    g.rect(0, 3, 14, 1, "#1a3a5c");
    g.rect(0, 12, 14, 1, "#1a3a5c");
    g.rect(3, 0, 8, 2, "#3e7fb0");
    draw_text(g, "!", 6, 6, c('e'), 1);
}

fn vending(g: &mut Canvas, f: i32) {
    g.rect(0, 0, 16, 30, "#b13e53");
    g.rect(2, 3, 9, 18, "#1a1c2c");
    let cans = [c('a'), c('e'), c('d'), c('i')];
    for y in 0..4 {
        for x in 0..3 {
            g.rect(3 + x * 3, 4 + y * 4, 2, 3, cans[((x + y) % 4) as usize]);
        }
    }
    g.rect(12, 4, 3, 6, if f != 0 { c('a') } else { "#2b6b78" });
    g.rect(2, 23, 9, 4, "#0e0b16");
    g.rect(0, 0, 16, 2, "#d0566f");
}

fn kiosk(g: &mut Canvas, f: i32) {
    g.rect(2, 12, 12, 18, "#3d4554");
    g.rect(2, 12, 12, 2, "#5f6b7d");
    g.rect(0, 0, 16, 13, "#262a36");
    g.rect(1, 1, 14, 10, if f != 0 { "#0f3b44" } else { "#0b2a31" });
    g.rect(2, 2, 8, 1, c('a'));
    g.rect(2, 4, 11, 1, "#3aa8b8");
    g.rect(2, 6, 6, 1, "#3aa8b8");
    if f != 0 {
        g.rect(2, 8, 2, 1, c('a'));
    }
    g.rect(5, 16, 6, 3, "#1a1c2c");
    g.px(7, 17, c('d'));
}

fn antenna(g: &mut Canvas, f: i32) {
    g.rect(7, 4, 2, 44, "#5f6b7d");
    for y in (10..46).step_by(6) {
        g.rect(4, y, 8, 1, "#5f6b7d");
        g.px(5, y + 1, "#3d4554");
        g.px(10, y + 1, "#3d4554");
    }
    g.rect(6, 0, 4, 4, if f != 0 { "#ff5566" } else { "#7a2e3a" });
}

fn dish(g: &mut Canvas, _f: i32) {
    g.rect(12, 18, 6, 12, "#5f6b7d");
    g.rect(8, 27, 14, 3, "#3d4554");
    for y in 0..20 {
        for x in 0..30 {
            let dx = (x as f64 - 15.0) / 15.0;
            let dy = (y as f64 - 9.0) / 9.0;
            if dx * dx + dy * dy <= 1.0 {
                g.px(x, y, if dy > 0.3 { "#94b0c2" } else { "#c7d3dc" });
            }
        }
    }
    g.rect(14, 7, 2, 6, "#3d4554");
    g.px(15, 6, c('i'));
}

fn boat(g: &mut Canvas, f: i32) {
    let dy = f;
    g.rect(2, 8 + dy, 42, 10, "#e6e2c3");
    g.rect(0, 8 + dy, 46, 3, "#b13e53");
    g.rect(4, 18 + dy, 38, 3, "#94b0c2");
    g.rect(14, 1 + dy, 16, 8, "#c7ccd6");
    g.rect(16, 3 + dy, 4, 3, "#3a5a8a");
    g.rect(22, 3 + dy, 4, 3, "#3a5a8a");
    draw_text(g, "OTP", 32, 12 + dy, "#29366f", 1);
}

fn barrier_h(g: &mut Canvas, f: i32) {
    g.rect(0, 5, 16, 6, if f != 0 { "#ff3e9a55" } else { "#ff3e9a33" });
    for x in ((f * 3) % 6..16).step_by(6) {
        g.rect(x, 6, 2, 4, "#ff7ab8");
    }
    g.rect(0, 5, 16, 1, "#ff9acb");
    g.rect(0, 10, 16, 1, "#ff9acb");
}

fn barrier_v(g: &mut Canvas, f: i32) {
    g.rect(5, 0, 6, 16, if f != 0 { "#ff3e9a55" } else { "#ff3e9a33" });
    for y in ((f * 3) % 6..16).step_by(6) {
        g.rect(6, y, 4, 2, "#ff7ab8");
    }
    g.rect(5, 0, 1, 16, "#ff9acb");
    g.rect(10, 0, 1, 16, "#ff9acb");
}

fn pylon(g: &mut Canvas, f: i32) {
    g.rect(2, 4, 6, 16, "#3d4554");
    g.rect(1, 17, 8, 3, "#262a36");
    g.rect(3, 0, 4, 5, if f != 0 { "#ff3e9a" } else { "#b13e78" });
    g.rect(4, 6, 2, 8, "#ff3e9a88");
}

fn drone(g: &mut Canvas, f: i32) {
    g.rect(5, 4, 8, 6, "#3d4554");
    g.rect(6, 5, 6, 3, "#262a36");
    g.px(8, 6, "#ff5566");
    g.px(9, 6, "#ff5566");
    g.rect(0, 2, 5, 1, "#5f6b7d");
    g.rect(13, 2, 5, 1, "#5f6b7d");
    let blade = if f != 0 { "#c7d3dc" } else { "#7a8699" };
    g.rect(if f != 0 { 0 } else { 1 }, 0, 4, 1, blade);
    g.rect(if f != 0 { 14 } else { 13 }, 0, 4, 1, blade);
    g.rect(7, 11, 4, 2, "#ff556655");
}

fn vault(g: &mut Canvas, f: i32) {
    // plinth
    g.rect(2, 40, 44, 12, "#262a36");
    g.rect(2, 40, 44, 2, "#3d4554");
    g.rect(0, 48, 48, 4, "#1a1c2c");
    // monolith
    g.rect(8, 2, 32, 40, "#1d2236");
    g.rect(8, 2, 32, 2, "#3a4466");
    g.rect(10, 6, 28, 32, "#151a2b");
    // the five locks (shares)
    for i in 0..5 {
        let lit = f == 2 || (f == 1 && i < 3);
        g.rect(13 + i * 5, 32, 3, 3, if lit { c('a') } else { "#2b3550" });
    }
    if f == 2 {
        g.rect(16, 8, 16, 22, "#0b0d16");
        g.rect(17, 9, 14, 20, "#102a33");
        g.rect(22, 14, 4, 8, c('a'));
        g.rect(20, 16, 8, 4, c('a'));
    } else {
        // engraved key glyph
        g.rect(20, 12, 8, 8, "#2b3550");
        g.rect(22, 14, 4, 4, "#151a2b");
        g.rect(23, 20, 2, 9, "#2b3550");
        g.rect(25, 24, 3, 2, "#2b3550");
        g.rect(25, 27, 2, 2, "#2b3550");
    }
    draw_text(g, "LEGACY", 13, 42, if f == 2 { c('a') } else { "#5f6b7d" }, 1);
}

fn brick_drop(g: &mut Canvas, f: i32) {
    g.rect(0, 0, 32, 18, "#6e2a1f");
    for y in (0..18).step_by(4) {
        let x0 = if (y / 4) % 2 == 1 { 0 } else { 4 };
        for x in (x0..32).step_by(8) {
            g.rect(x, y, 7, 3, "#9c4a32");
        }
    }
    g.rect(0, 0, 32, 2, "#5f6b7d");
    if f == 0 {
        // chalk mark on the loose brick
        g.rect(12, 8, 7, 3, "#a8563a");
        g.px(13, 9, "#e6e2c3");
        g.px(14, 9, "#e6e2c3");
        g.px(16, 9, "#e6e2c3");
    } else {
        g.rect(12, 8, 7, 3, "#140d0b");
    }
}

fn mailbox(g: &mut Canvas, f: i32) {
    g.rect(5, 9, 2, 11, "#3d4554");
    g.rect(0, 0, 12, 10, "#29366f");
    g.rect(0, 0, 12, 2, "#3b5dc9");
    g.rect(2, 4, 8, 2, "#0e0b16");
    if f == 0 {
        g.px(10, 6, c('e'));
    }
}

fn server_rack(g: &mut Canvas, f: i32) {
    g.rect(0, 0, 16, 30, "#1a1c2c");
    g.rect(1, 1, 14, 28, "#262a36");
    for i in 0..6 {
        g.rect(2, 2 + i * 4, 12, 3, "#1f2330");
        g.px(3, 3 + i * 4, if (i + f) % 3 != 0 { c('d') } else { c('a') });
        g.px(5, 3 + i * 4, if (i + f) % 2 != 0 { "#ff5566" } else { "#2b3550" });
    }
}

fn trash(g: &mut Canvas, _f: i32) {
    g.rect(1, 3, 10, 11, "#3d4554");
    g.rect(0, 1, 12, 3, "#5f6b7d");
    for x in (3..10).step_by(3) {
        g.rect(x, 5, 1, 7, "#262a36");
    }
}

fn hydrant(g: &mut Canvas, _f: i32) {
    g.rect(2, 3, 6, 10, "#b13e53");
    g.rect(1, 12, 8, 2, "#7a2638");
    g.rect(3, 0, 4, 3, "#b13e53");
    g.rect(0, 6, 10, 2, "#7a2638");
}

fn telescope(g: &mut Canvas, _f: i32) {
    g.rect(10, 14, 4, 10, "#3d4554");
    g.rect(6, 23, 12, 3, "#262a36");
    for i in 0..16 {
        let y = 12 - (i as f64 * 0.6).floor() as i32;
        g.rect(2 + i, y, 5, 4, if i < 4 { "#c7d3dc" } else { "#94b0c2" });
    }
    g.rect(18, 0, 5, 5, "#c7d3dc");
}

fn masks(g: &mut Canvas, _f: i32) {
    let mut face = |x: i32, col: &str, smile: bool| {
        g.rect(x, 0, 9, 11, col);
        g.rect(x + 1, 11, 7, 2, col);
        g.rect(x + 2, 3, 2, 2, "#0e0b16");
        g.rect(x + 5, 3, 2, 2, "#0e0b16");
        g.rect(x + 2, 8, 5, 1, "#0e0b16");
        let corner = if smile { 7 } else { 9 };
        g.px(x + 1, corner, "#0e0b16");
        g.px(x + 7, corner, "#0e0b16");
    };
    face(0, "#f4f4f4", true);
    face(11, "#c792ff", false);
}

fn coin(g: &mut Canvas, f: i32) {
    let r = 6.5;
    for y in 0..14 {
        for x in 0..14 {
            let dx = (x as f64 - 6.5) / if f != 0 { r * 0.6 } else { r };
            let dy = (y as f64 - 6.5) / r;
            if dx * dx + dy * dy <= 1.0 {
                g.px(x, y, if dy < -0.3 { "#ffe7a8" } else { c('e') });
            }
        }
    }
    if f == 0 {
        draw_text(g, "B", 6, 4, "#b8862e", 1);
    }
}

fn projector(g: &mut Canvas, f: i32) {
    g.rect(1, 3, 14, 5, "#262a36");
    g.rect(4, 1, 8, 3, "#3d4554");
    g.rect(6, 0, 4, 2, if f != 0 { c('a') } else { "#2b8a96" });
}

fn stall(g: &mut Canvas, _f: i32) {
    for x in (0..32).step_by(4) {
        g.rect(x, 0, 4, 8, if (x / 4) % 2 == 1 { "#e6e2c3" } else { "#b13e53" });
    }
    g.rect(0, 8, 32, 2, "#7a2638");
    g.rect(2, 10, 2, 12, "#4a2c1a");
    g.rect(28, 10, 2, 12, "#4a2c1a");
    g.rect(0, 20, 32, 10, c('l'));
    g.rect(0, 20, 32, 2, c('m'));
    let goods = ["#ef7d57", "#a7f070", "#ffcd75", "#73eff7", "#ff3e9a"];
    for (i, col) in goods.iter().enumerate() {
        g.rect(3 + i as i32 * 6, 17, 4, 3, *col);
    }
}

fn cone(g: &mut Canvas, _f: i32) {
    g.rect(0, 10, 10, 2, "#262a36");
    g.rect(4, 0, 2, 2, c('y'));
    g.rect(3, 2, 4, 3, c('y'));
    g.rect(2, 5, 6, 2, "#e6e2c3");
    g.rect(2, 7, 6, 3, c('y'));
}

fn statue(g: &mut Canvas, _f: i32) {
    g.rect(1, 26, 18, 8, "#3d4554");
    g.rect(1, 26, 18, 2, "#5f6b7d");
    g.rect(7, 2, 6, 6, "#94b0c2"); // head
    g.rect(5, 8, 10, 12, "#94b0c2"); // body
    g.rect(3, 10, 2, 8, "#7a8699");
    g.rect(15, 6, 2, 10, "#7a8699"); // raised arm
    g.rect(14, 2, 4, 4, "#c7d3dc"); // holding a key
    g.rect(6, 20, 3, 6, "#7a8699");
    g.rect(11, 20, 3, 6, "#7a8699");
}

// ---------------------------------------------------------------------------
// Signs and buildings
// ---------------------------------------------------------------------------

fn shade(hex: &str) -> String {
    let c = Rgba::hex(hex);
    let s = |v: u8| ((v as f64) * 0.75).round() as u8;
    format!("#{:02x}{:02x}{:02x}", s(c.0), s(c.1), s(c.2))
}

pub fn neon_sign(text: &'static str, color: &'static str) -> ObjectArt {
    let w = text_width(text, 1) + 8;
    let dim = shade(color);
    ObjectArt {
        w,
        h: 11,
        frames: 2,
        outline: false,
        draw: Arc::new(move |g: &mut Canvas, f: i32| {
            g.rect(0, 0, w, 11, "#0e0b16");
            g.rect(1, 1, w - 2, 9, "#151a2b");
            draw_text(g, text, 4, 3, if f == 0 { color } else { dim.as_str() }, 1);
            g.rect(0, 0, w, 1, "#262a36");
        }),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RoofStyle {
    Flat,
    Dome,
    Peak,
    Sawtooth,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Pattern {
    Plain,
    Bricks,
    Panels,
    Wood,
}

#[derive(Clone, Copy)]
pub struct BuildingStyle {
    pub w: i32,
    pub h: i32,
    pub wall: &'static str,
    pub wall_dark: &'static str,
    pub roof: &'static str,
    pub trim: &'static str,
    pub window: &'static str,
    pub window_off: &'static str,
    pub door: &'static str,
    pub seed: u32,
    pub roof_style: RoofStyle,
    pub door_at: Option<i32>,
    pub pattern: Pattern,
}

pub fn building(st: BuildingStyle) -> ObjectArt {
    ObjectArt {
        w: st.w,
        h: st.h,
        frames: 2,
        outline: true,
        draw: Arc::new(move |g: &mut Canvas, f: i32| {
            let (w, h) = (st.w, st.h);
            let roof_h = if st.roof_style == RoofStyle::Dome { 22 } else { 10 };
            let mut r = Prng::new(st.seed);
            g.rect(0, roof_h, w, h - roof_h, st.wall);
            match st.pattern {
                Pattern::Bricks => {
                    for y in (roof_h..h).step_by(4) {
                        let x0 = if ((y - roof_h) / 4) % 2 == 1 { 0 } else { 4 };
                        for x in (x0..w).step_by(8) {
                            g.rect(x, y + 3, 7, 1, st.wall_dark);
                        }
                    }
                }
                Pattern::Panels => {
                    for x in (0..w).step_by(12) {
                        g.rect(x, roof_h, 1, h - roof_h, st.wall_dark);
                    }
                }
                Pattern::Wood => {
                    for y in (roof_h..h).step_by(3) {
                        g.rect(0, y + 2, w, 1, st.wall_dark);
                    }
                }
                Pattern::Plain => {}
            }
            g.rect(0, h - 3, w, 3, st.wall_dark);
            // roof
            match st.roof_style {
                RoofStyle::Dome => {
                    let cx = w as f64 / 2.0;
                    let rad = (w as f64 / 2.0 - 2.0).min(22.0);
                    for y in 0..roof_h + 2 {
                        for x in 0..w {
                            let dx = (x as f64 + 0.5 - cx) / rad;
                            let dy = (y as f64 + 0.5 - (roof_h + 2) as f64) / (roof_h + 2) as f64;
                            if dx * dx + dy * dy <= 1.0 {
                                g.px(x, y, if dy < -0.6 { "#c7d3dc" } else { st.roof });
                            }
                        }
                    }
                    g.rect(cx.floor() as i32 - 1, 2, 3, roof_h - 2, "#0e0b16");
                    g.rect(0, roof_h, w, 3, st.trim);
                }
                RoofStyle::Peak => {
                    for y in 0..roof_h {
                        let inset = (roof_h - 1 - y).max(0) * 2;
                        g.rect(inset, y, w - inset * 2, 1, if y % 3 == 2 { st.trim } else { st.roof });
                    }
                }
                RoofStyle::Sawtooth => {
                    for x in (0..w).step_by(12) {
                        for y in 0..roof_h {
                            g.rect(x, y, 12.min(((y + 1) as f64 * 1.2).floor() as i32), 1, st.roof);
                        }
                    }
                    g.rect(0, roof_h - 2, w, 2, st.trim);
                }
                RoofStyle::Flat => {
                    g.rect(0, 0, w, roof_h, st.roof);
                    g.rect(0, roof_h - 2, w, 2, st.trim);
                    // rooftop clutter
                    let cx = 4 + r.below(10);
                    g.rect(cx, 2, 8, 5, st.wall_dark);
                    g.rect(w - 18, 3, 6, 4, st.wall_dark);
                }
            }
            // windows
            let door_x = st.door_at.unwrap_or(w / 2 - 6);
            for y in (roof_h + 5..h - 18).step_by(11) {
                for x in (5..w - 9).step_by(12) {
                    if x + 8 > door_x - 2 && x < door_x + 14 && y > h - 32 {
                        continue;
                    }
                    let lit = r.next() < 0.6;
                    let flicker = r.next() < 0.12;
                    let on = lit && !(f == 1 && flicker);
                    g.rect(x, y, 8, 7, "#0e0b16");
                    g.rect(x + 1, y + 1, 6, 5, if on { st.window } else { st.window_off });
                    if on {
                        g.rect(x + 1, y + 1, 2, 1, "#ffffff66");
                    }
                }
            }
            // door
            g.rect(door_x, h - 16, 12, 13, "#0e0b16");
            g.rect(door_x + 1, h - 15, 10, 12, st.door);
            g.rect(door_x + 1, h - 15, 10, 2, st.trim);
            g.px(door_x + 8, h - 9, "#ffffffaa");
            // awning
            g.rect(door_x - 3, h - 19, 18, 3, st.trim);
        }),
    }
}

#[allow(clippy::too_many_arguments)]
fn bstyle(w: i32, h: i32, wall: &'static str, wall_dark: &'static str, roof: &'static str, trim: &'static str, window: &'static str, window_off: &'static str, door: &'static str, seed: u32, pattern: Pattern, roof_style: RoofStyle) -> ObjectArt {
    building(BuildingStyle { w, h, wall, wall_dark, roof, trim, window, window_off, door, seed, roof_style, door_at: None, pattern })
}

/// District buildings (`bld-<key>`).
pub fn buildings() -> Vec<(&'static str, ObjectArt)> {
    use Pattern::*;
    use RoofStyle::*;
    vec![
        ("exchange", bstyle(160, 84, "#2a3f66", "#1d2c4a", "#1a1c2c", "#ef7d57", "#ffb347", "#141a2e", "#3b5dc9", 11, Panels, Sawtooth)),
        ("mint", bstyle(160, 88, "#6b5320", "#4a3914", "#2b1f0c", "#ffcd75", "#ffe7a8", "#241a0a", "#9c7a2e", 12, Bricks, Peak)),
        ("masks", bstyle(160, 84, "#3a2a5c", "#2a1e44", "#1d1430", "#ff3e9a", "#c792ff", "#1a1230", "#7d4bd6", 13, Panels, Peak)),
        ("observatory", bstyle(176, 96, "#1d2236", "#151a2b", "#94b0c2", "#41a6f6", "#73eff7", "#10162a", "#29366f", 14, Plain, Dome)),
        ("boathouse", bstyle(128, 72, "#5c3a21", "#3e2614", "#2b1810", "#73eff7", "#ffcd75", "#1f140b", "#7a4a28", 15, Wood, Peak)),
        ("radioshack", bstyle(80, 60, "#3d4554", "#2b313d", "#1f2330", "#ffcd75", "#a7f070", "#141a14", "#566c86", 16, Panels, Flat)),
        ("pavilion", bstyle(96, 56, "#1f4a33", "#173326", "#2a5e3c", "#a7f070", "#d8ffb0", "#0f2418", "#2f6b44", 17, Wood, Peak)),
        ("tower", bstyle(96, 120, "#262a36", "#1a1d26", "#12141c", "#ff5566", "#ff8a8a", "#14151c", "#3d4554", 18, Panels, Flat)),
        ("block_a", bstyle(112, 92, "#2a2440", "#1d1930", "#141020", "#73eff7", "#73eff7", "#141026", "#3b5dc9", 19, Plain, Flat)),
        ("block_b", bstyle(112, 92, "#2b3a3a", "#1d2828", "#121a1a", "#a7f070", "#ffcd75", "#121c1c", "#257179", 20, Bricks, Flat)),
    ]
}

/// Neon signs (`sign-<key>`).
pub fn signs() -> Vec<(&'static str, ObjectArt)> {
    vec![
        ("wharf", neon_sign("THE WHARF", "#73eff7")),
        ("exchange", neon_sign("EXCHANGE", "#ef7d57")),
        ("commons", neon_sign("THE COMMONS", "#a7f070")),
        ("mint", neon_sign("THE MINT", "#ffcd75")),
        ("masks", neon_sign("MASQUERADE", "#ff3e9a")),
        ("observatory", neon_sign("OBSERVATORY", "#73b7ff")),
        ("plaza", neon_sign("LEGACY PLAZA", "#73eff7")),
        ("boathouse", neon_sign("BOATHOUSE", "#ffcd75")),
        ("radio", neon_sign("RADIO", "#a7f070")),
        ("dla", neon_sign("DLA", "#ff5566")),
        ("qsafe", neon_sign("QSAFE", "#ff5566")),
    ]
}

// ---------------------------------------------------------------------------
// Set pieces
// ---------------------------------------------------------------------------

fn door(g: &mut Canvas, _f: i32) {
    g.rect(0, 0, 16, 30, c('j'));
    g.rect(2, 2, 12, 28, c('l'));
    for i in 0..3 {
        g.rect(2 + i * 4 + 1, 2, 1, 28, c('k'));
    }
    g.rect(2, 10, 12, 1, c('k'));
    g.rect(2, 20, 12, 1, c('k'));
    g.px(11, 16, c('e'));
    g.px(11, 17, "#b8862e");
}

fn gatepad(g: &mut Canvas, f: i32) {
    for y in 0..20 {
        for x in 0..40 {
            let dx = (x as f64 + 0.5 - 20.0) / 20.0;
            let dy = (y as f64 + 0.5 - 10.0) / 10.0;
            let d = dx * dx + dy * dy;
            if d > 1.0 {
                continue;
            }
            let ring = (d.sqrt() - (0.35 + f as f64 * 0.25)).abs() < 0.08;
            let col = if ring {
                "#73eff7"
            } else if d > 0.85 {
                "#3ef0ff"
            } else if d > 0.6 {
                "#1f6b78"
            } else {
                "#12303a"
            };
            g.px(x, y, col);
        }
    }
}

fn archive(g: &mut Canvas, f: i32) {
    g.rect(0, 0, 32, 40, "#3d4554");
    g.rect(2, 2, 28, 36, if f != 0 { "#0b0d16" } else { "#5f6b7d" });
    if f == 0 {
        for y in (6..36).step_by(6) {
            g.rect(4, y, 24, 1, "#4a5263");
        }
        g.rect(10, 14, 12, 12, "#b13e53");
        g.rect(11, 15, 10, 10, "#7a2638");
        draw_text(g, "DLA", 11, 18, "#ffcdd2", 1);
        g.rect(24, 18, 3, 6, "#94b0c2");
    } else {
        g.rect(6, 8, 20, 26, "#0f2a33");
        for (i, col) in ["#73eff7", "#ffcd75", "#a7f070", "#c792ff", "#ef7d57"].iter().enumerate() {
            g.rect(8, 10 + i as i32 * 5, 16, 3, *col);
        }
    }
}

fn checkpoint(g: &mut Canvas, f: i32) {
    g.rect(0, 6, 8, 28, "#262a36");
    g.rect(40, 6, 8, 28, "#262a36");
    g.rect(0, 0, 48, 8, "#1f2330");
    draw_text(g, "CHECKPOINT", 5, 2, "#ff5566", 0);
    g.rect(2, 12, 4, 6, if f != 0 { "#ff5566" } else { "#7a2638" });
    g.rect(42, 12, 4, 6, if f != 0 { "#5dff9a" } else { "#1f5a3a" });
    g.rect(8, 20, 32, 3, if f != 0 { "#ff3e9a" } else { "#b13e78" });
    for x in (10..40).step_by(6) {
        g.rect(x, 20, 3, 3, "#ffcdd2");
    }
}

fn billboard(g: &mut Canvas, f: i32) {
    g.rect(10, 30, 3, 14, "#3d4554");
    g.rect(59, 30, 3, 14, "#3d4554");
    g.rect(0, 0, 72, 32, "#1a1c2c");
    g.rect(2, 2, 68, 28, if f != 0 { "#2a0f18" } else { "#3a121f" });
    draw_text(g, "LAWFUL ACCESS", 9, 6, "#ffcdd2", 1);
    draw_text(g, "KEEPS YOU", 18, 14, "#ff5566", 1);
    draw_text(g, "SAFE", 28, 22, "#ff5566", 1);
    if f != 0 {
        g.rect(2, 17, 68, 1, "#ff556644");
    }
}

fn shard(g: &mut Canvas, f: i32) {
    let col = if f != 0 { "#a4ffe0" } else { "#73eff7" };
    for y in 0..16 {
        let w = if y < 8 { (y as f64 * 0.75).floor() as i32 + 1 } else { ((15 - y) as f64 * 0.75).floor() as i32 + 1 };
        g.rect(6 - w, y, w * 2, 1, if y % 4 == 0 { "#ffffff" } else { col });
    }
}

fn radio(g: &mut Canvas, f: i32) {
    g.rect(0, 4, 26, 16, "#566c86");
    g.rect(0, 4, 26, 2, "#94b0c2");
    g.rect(3, 8, 10, 8, "#1a1c2c");
    for i in 0..4 {
        g.rect(4, 9 + i * 2, 8, 1, "#3d4554");
    }
    g.rect(16, 8, 7, 3, if f != 0 { "#a7f070" } else { "#38b764" });
    g.px(17, 14, "#ffcd75");
    g.px(20, 14, "#ffcd75");
    g.rect(20, 0, 1, 5, "#94b0c2");
}

fn whiteboard(g: &mut Canvas, _f: i32) {
    g.rect(0, 0, 40, 24, "#c7d3dc");
    g.rect(2, 2, 36, 20, "#f4f4f4");
    let mut r = Prng::new(5);
    for i in 0..6 {
        let x = 4 + r.below(4);
        let len = 10 + r.below(18);
        g.rect(x, 4 + i * 3, len, 1, if i % 2 == 1 { "#3b5dc9" } else { "#b13e53" });
    }
    g.rect(6, 24, 2, 6, "#5f6b7d");
    g.rect(32, 24, 2, 6, "#5f6b7d");
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Props (`obj-<key>`), including the larger set pieces.
pub fn objects() -> Vec<(&'static str, ObjectArt)> {
    vec![
        ("bed", art(28, 34, 1, true, bed)),
        ("bookshelf", art(30, 34, 1, true, bookshelf)),
        ("deskcrt", art(32, 30, 2, true, desk_crt)),
        ("dresser", art(22, 24, 2, true, dresser)),
        ("fireplace", art(32, 34, 3, true, fireplace)),
        ("plant", art(14, 22, 1, true, plant)),
        ("armchair", art(18, 20, 1, true, armchair)),
        ("sidetable", art(14, 24, 1, true, side_table)),
        ("photo", art(12, 10, 1, true, photo)),
        ("rug", art(48, 30, 1, false, rug)),
        ("coatrack", art(12, 28, 1, true, coat_rack)),
        ("boxes", art(18, 18, 1, true, boxes)),
        ("note", art(10, 8, 1, false, note)),
        ("lamp", art(14, 40, 1, true, lamp)),
        ("bench", art(30, 16, 1, true, bench)),
        ("tree", art(32, 42, 2, true, tree)),
        ("bush", art(16, 14, 1, true, bush)),
        ("crate", art(16, 16, 1, true, crate_box)),
        ("barrel", art(14, 16, 1, true, barrel)),
        ("vending", art(16, 30, 2, true, vending)),
        ("kiosk", art(16, 30, 2, true, kiosk)),
        ("antenna", art(16, 48, 2, true, antenna)),
        ("dish", art(30, 30, 1, true, dish)),
        ("boat", art(46, 22, 2, true, boat)),
        ("barrier_h", art(16, 16, 2, false, barrier_h)),
        ("barrier_v", art(16, 16, 2, false, barrier_v)),
        ("pylon", art(10, 20, 2, true, pylon)),
        ("drone", art(18, 14, 2, true, drone)),
        ("vault", art(48, 52, 3, true, vault)),
        ("brickdrop", art(32, 18, 2, true, brick_drop)),
        ("mailbox", art(12, 20, 2, true, mailbox)),
        ("server", art(16, 30, 2, true, server_rack)),
        ("trash", art(12, 14, 1, true, trash)),
        ("hydrant", art(10, 14, 1, true, hydrant)),
        ("telescope", art(24, 26, 1, true, telescope)),
        ("masks", art(20, 14, 1, true, masks)),
        ("coin", art(14, 14, 2, true, coin)),
        ("projector", art(16, 8, 2, true, projector)),
        ("stall", art(32, 30, 1, true, stall)),
        ("cone", art(10, 12, 1, true, cone)),
        ("statue", art(20, 34, 1, true, statue)),
        ("door", art(16, 30, 1, true, door)),
        ("gatepad", art(40, 20, 3, false, gatepad)),
        ("archive", art(32, 40, 2, true, archive)),
        ("checkpoint", art(48, 34, 2, true, checkpoint)),
        ("billboard", art(72, 44, 2, true, billboard)),
        ("shard", art(12, 16, 2, true, shard)),
        ("radio", art(26, 20, 2, true, radio)),
        ("whiteboard", art(40, 30, 1, true, whiteboard)),
    ]
}

/// Every sprite sheet keyed the way maps refer to them: `obj-*`, `bld-*`, `sign-*`.
pub fn all_sprites() -> Vec<(String, ObjectArt)> {
    let mut v: Vec<(String, ObjectArt)> = objects().into_iter().map(|(k, a)| (format!("obj-{k}"), a)).collect();
    v.extend(buildings().into_iter().map(|(k, a)| (format!("bld-{k}"), a)));
    v.extend(signs().into_iter().map(|(k, a)| (format!("sign-{k}"), a)));
    v
}

/// Looping animation rate (frames per second) for a sprite key, if it animates on its own.
pub fn anim_rate(key: &str) -> Option<f32> {
    if key.starts_with("bld-") {
        return Some(0.4);
    }
    if key.starts_with("sign-") {
        return Some(0.7);
    }
    let name = key.trim_start_matches("obj-");
    if STATEFUL.contains(&name) {
        return None;
    }
    Some(match name {
        "fireplace" => 7.0,
        "kiosk" => 1.5,
        "vending" => 1.0,
        "antenna" => 1.2,
        "boat" => 1.0,
        "barrier_h" | "barrier_v" => 5.0,
        "drone" => 12.0,
        "server" => 2.0,
        "coin" => 2.0,
        "projector" => 2.0,
        "pylon" => 2.0,
        "gatepad" => 4.0,
        "shard" => 3.0,
        "radio" => 2.0,
        "billboard" => 1.0,
        "checkpoint" => 2.0,
        _ => return None,
    })
}

/// Objects whose frames are states (set by the story), not animation.
pub const STATEFUL: &[&str] = &["deskcrt", "dresser", "tree", "vault", "brickdrop", "mailbox", "archive"];
