//! The game's palette. Pixel art is written as rows of characters; each
//! character names a colour here. Upper-case letters are *slots* that a
//! character's own palette fills in (hair, skin, clothes...), so one body
//! template serves every person in Cryptopolis.

use crate::canvas::{Rgba, Slots};

pub const OUTLINE: &str = "#0e0b16";

/// Global palette characters and their colours.
pub const COLORS: &[(char, &str)] = &[
    ('0', OUTLINE),
    ('1', "#1a1c2c"),
    ('2', "#2a2440"),
    ('3', "#333c57"),
    ('4', "#566c86"),
    ('5', "#94b0c2"),
    ('6', "#f4f4f4"),
    ('7', "#29366f"),
    ('8', "#3b5dc9"),
    ('9', "#41a6f6"),
    ('a', "#73eff7"),
    ('b', "#257179"),
    ('c', "#38b764"),
    ('d', "#a7f070"),
    ('e', "#ffcd75"),
    ('f', "#ef7d57"),
    ('g', "#b13e53"),
    ('h', "#5d275d"),
    ('i', "#ff3e9a"),
    ('j', "#2b1810"),
    ('k', "#4a2c1a"),
    ('l', "#7a4a28"),
    ('m', "#a8703f"),
    ('n', "#d9b38c"),
    ('o', "#fff1d0"),
    ('p', "#6e2a1f"),
    ('q', "#9c4a32"),
    ('r', "#3d4554"),
    ('s', "#5f6b7d"),
    ('t', "#1f2330"),
    ('u', "#173326"),
    ('v', "#2a5e3c"),
    ('w', "#e6e2c3"),
    ('x', "#ffb347"),
    ('y', "#ff7a2f"),
    ('z', "#fff35c"),
    ('#', "#0b0d16"),
    ('+', "#12172a"),
    ('=', "#3a2a5c"),
    ('*', "#c792ff"),
    ('%', "#7d4bd6"),
    ('&', "#1d4f58"),
    ('@', "#a4ffe0"),
];

/// Hex string for a palette character (panics on unknown characters).
pub fn c(ch: char) -> &'static str {
    COLORS.iter().find(|(k, _)| *k == ch).map(|(_, v)| *v).unwrap_or_else(|| panic!("no palette colour {ch:?}"))
}

/// Resolve a pixel character to a colour (slot palette first, then the global palette).
pub fn color_of(ch: char, slots: Option<Slots>) -> Option<Rgba> {
    if ch == '.' || ch == ' ' {
        return None;
    }
    if let Some(s) = slots
        && let Some((_, col)) = s.iter().find(|(k, _)| *k == ch)
    {
        return Some(*col);
    }
    COLORS.iter().find(|(k, _)| *k == ch).map(|(_, v)| Rgba::hex(v))
}

/// Skin tones: (base, shade).
pub const SKIN_FAIR: (&str, &str) = ("#f6d2b0", "#d9a782");
pub const SKIN_LIGHT: (&str, &str) = ("#eebd93", "#c98f68");
pub const SKIN_TAN: (&str, &str) = ("#c98a5d", "#a26843");
pub const SKIN_BROWN: (&str, &str) = ("#9a6440", "#74472b");
pub const SKIN_DEEP: (&str, &str) = ("#6b4129", "#4b2c1b");

/// UI theme colours (shared with the egui theme).
pub mod ui {
    pub const BG: &str = "#0b0d16";
    pub const PANEL: &str = "#121827";
    pub const CYAN: &str = "#3ef0ff";
    pub const MAGENTA: &str = "#ff3e9a";
    pub const AMBER: &str = "#ffb347";
    pub const GREEN: &str = "#5dff9a";
    pub const RED: &str = "#ff5566";
    pub const TEXT: &str = "#d7e3f4";
    pub const DIM: &str = "#7f8ba3";
}
