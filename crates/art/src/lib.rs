//! Code-defined pixel art for *Cypherpunk Legacy*.
//!
//! Nothing here loads an image file: tiles, people, props, buildings and neon
//! signs are painted by code into RGBA buffers. The Bevy game uploads them as
//! textures at startup; the `preview` example writes them out as PNG sheets.

pub mod canvas;
pub mod characters;
pub mod font;
pub mod objects;
pub mod palette;
pub mod tiles;

pub use canvas::{Canvas, Rgba};

use canvas::glow;
use font::draw_text;
use palette::OUTLINE;

/// Light sprites: (key, radius, colour).
pub const GLOWS: &[(&str, i32, &str)] = &[
    ("glow-warm", 48, "#ffb347"),
    ("glow-fire", 64, "#ff7a2f"),
    ("glow-cyan", 48, "#3ef0ff"),
    ("glow-magenta", 48, "#ff3e9a"),
    ("glow-green", 40, "#5dff9a"),
    ("glow-gold", 48, "#ffcd75"),
    ("glow-blue", 48, "#73b7ff"),
    ("glow-violet", 48, "#c792ff"),
    ("glow-red", 40, "#ff5566"),
    ("glow-white", 64, "#dfe8ff"),
];

pub fn glow_texture(radius: i32, color: &str) -> Canvas {
    glow(radius, color, 2.2)
}

/// The interaction prompt: a tiny "E" key cap, two bobbing frames of 11x12.
pub fn prompt_sheet() -> Canvas {
    let mut c = Canvas::new(22, 12);
    for f in 0..2 {
        let x0 = f * 11;
        c.rect(x0 + 1, 1 + f, 9, 8, "#e6f6ff");
        c.rect(x0 + 1, 8 + f, 9, 1, "#9ab8d0");
        draw_text(&mut c, "E", x0 + 4, 2 + f, "#0e0b16", 1);
    }
    c.outline(OUTLINE, None);
    c
}

/// Soft elliptical shadow under characters.
pub fn shadow() -> Canvas {
    let mut c = Canvas::new(12, 5);
    let col = Rgba(0, 0, 0, 89);
    for (x, y, w, h) in [(2, 0, 8, 5), (0, 1, 12, 3)] {
        for yy in y..y + h {
            for xx in x..x + w {
                c.set(xx, yy, col);
            }
        }
    }
    c
}

/// A 2x2 white particle.
pub fn dot() -> Canvas {
    let mut c = Canvas::new(2, 2);
    c.rect(0, 0, 2, 2, "#ffffff");
    c
}

/// A falling rain streak.
pub fn raindrop() -> Canvas {
    let mut c = Canvas::new(1, 4);
    c.rect(0, 0, 1, 4, Rgba(160, 200, 255, 140));
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everything_renders() {
        assert_eq!(tiles::tileset().w, 256);
        for def in characters::all_characters() {
            let s = characters::character_sheet(&def);
            assert_eq!((s.w, s.h), (16 * 12, 24));
            assert!(s.data.chunks_exact(4).any(|p| p[3] > 0), "{} is blank", def.id);
        }
        assert_eq!(characters::cat_sheet().w, 16 * 8);
        for (key, a) in objects::all_sprites() {
            let s = a.sheet();
            let (fw, fh) = a.frame_size();
            assert_eq!((s.w, s.h), (fw * a.frames, fh), "{key}");
            assert!(s.data.chunks_exact(4).any(|p| p[3] > 0), "{key} is blank");
        }
        assert_eq!(prompt_sheet().w, 22);
    }
}
