//! The pictures signal boards speak in: 8 x 7 lamps each, with the bottom row
//! left dark (a sealed board keeps its tag there).

use cpl_crypto::lamps::{Bits, from_rows};

macro_rules! glyphs {
    ($( $name:ident, $id:literal, $label:literal, [$($row:literal),* $(,)?]; )*) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum Glyph { $($name),* }

        impl Glyph {
            pub const ALL: &'static [Glyph] = &[$(Glyph::$name),*];

            pub fn rows(self) -> [&'static str; 7] {
                match self { $(Glyph::$name => [$($row),*]),* }
            }

            /// Stable id used in saves.
            pub fn id(self) -> &'static str {
                match self { $(Glyph::$name => $id),* }
            }

            /// What a legend calls it.
            pub fn label(self) -> &'static str {
                match self { $(Glyph::$name => $label),* }
            }
        }
    };
}

glyphs! {
    Open, "open", "OPEN", [
        "....###.",
        "...#...#",
        "...#....",
        ".######.",
        ".##..##.",
        ".##..##.",
        ".######.",
    ];
    Closed, "closed", "SHUT", [
        "..####..",
        ".#....#.",
        ".#....#.",
        ".######.",
        ".##..##.",
        ".##..##.",
        ".######.",
    ];
    Alarm, "alarm", "ALARM", [
        "...##...",
        "..####..",
        "..####..",
        ".######.",
        ".######.",
        "########",
        "...##...",
    ];
    East, "east", "EAST", [
        "....#...",
        "....##..",
        "#######.",
        "########",
        "#######.",
        "....##..",
        "....#...",
    ];
    West, "west", "WEST", [
        "...#....",
        "..##....",
        ".#######",
        "########",
        ".#######",
        "..##....",
        "...#....",
    ];
    North, "north", "NORTH", [
        "...#....",
        "..###...",
        ".#####..",
        "#######.",
        "..###...",
        "..###...",
        "..###...",
    ];
    South, "south", "SOUTH", [
        "..###...",
        "..###...",
        "..###...",
        "#######.",
        ".#####..",
        "..###...",
        "...#....",
    ];
    Eye, "eye", "WATCH", [
        "........",
        "..####..",
        ".#....#.",
        "#..##..#",
        ".#....#.",
        "..####..",
        "........",
    ];
    Shut, "shut", "SLEEP", [
        "........",
        "........",
        "#......#",
        ".#....#.",
        "..####..",
        ".#.##.#.",
        "........",
    ];
    Crest, "crest", "CREST", [
        "...##...",
        "...##...",
        "..#..#..",
        "..#..#..",
        ".#.##.#.",
        ".#....#.",
        "########",
    ];
    Fish, "fish", "FISH", [
        "........",
        "..###..#",
        ".#####.#",
        "#.######",
        ".#####.#",
        "..###..#",
        "........",
    ];
    Crab, "crab", "CRAB", [
        "#......#",
        "##....##",
        ".#.##.#.",
        ".######.",
        "#.####.#",
        ".#.##.#.",
        "#......#",
    ];
    Anchor, "anchor", "ANCHOR", [
        "...##...",
        "..#..#..",
        "...##...",
        ".######.",
        "...##...",
        "#..##..#",
        ".######.",
    ];
    Key, "key", "KEY", [
        ".###....",
        "#...#...",
        "#...####",
        "#...#.##",
        ".###..#.",
        "........",
        "........",
    ];
    Moon, "moon", "MOON", [
        "..###...",
        ".###....",
        "###.....",
        "###.....",
        "###.....",
        ".###....",
        "..###...",
    ];
    Star, "star", "STAR", [
        "...#....",
        "...#....",
        "#######.",
        ".#####..",
        "..###...",
        ".##.##..",
        ".#...#..",
    ];
    Tower, "tower", "DIRECTORATE", [
        "#..#..#.",
        ".#.#.#..",
        "...#....",
        "..###...",
        "..#.#...",
        ".#...#..",
        ".#####..",
    ];
    Sigil, "sigil", "NIGHTJAR", [
        "#......#",
        "##....##",
        ".##..##.",
        "..####..",
        "...##...",
        "..#..#..",
        "........",
    ];
    D0, "0", "0", [".###....", "#...#...", "#..##...", "#.#.#...", "##..#...", "#...#...", ".###...."];
    D1, "1", "1", ["..#.....", ".##.....", "..#.....", "..#.....", "..#.....", "..#.....", ".###...."];
    D2, "2", "2", [".###....", "#...#...", "....#...", "...#....", "..#.....", ".#......", "#####..."];
    D3, "3", "3", [".###....", "#...#...", "....#...", "..##....", "....#...", "#...#...", ".###...."];
    D4, "4", "4", ["...#....", "..##....", ".#.#....", "#..#....", "#####...", "...#....", "...#...."];
    D5, "5", "5", ["#####...", "#.......", "####....", "....#...", "....#...", "#...#...", ".###...."];
    D6, "6", "6", ["..##....", ".#......", "#.......", "####....", "#...#...", "#...#...", ".###...."];
    D7, "7", "7", ["#####...", "....#...", "...#....", "..#.....", ".#......", ".#......", ".#......"];
    D8, "8", "8", [".###....", "#...#...", "#...#...", ".###....", "#...#...", "#...#...", ".###...."];
    D9, "9", "9", [".###....", "#...#...", "#...#...", ".####...", "....#...", "...#....", ".##....."];
}

pub const DIGITS: [Glyph; 10] = [Glyph::D0, Glyph::D1, Glyph::D2, Glyph::D3, Glyph::D4, Glyph::D5, Glyph::D6, Glyph::D7, Glyph::D8, Glyph::D9];
pub const GATE: [Glyph; 3] = [Glyph::Open, Glyph::Closed, Glyph::Alarm];
pub const ORDERS: [Glyph; 4] = [Glyph::East, Glyph::West, Glyph::North, Glyph::South];
pub const CAMERA: [Glyph; 2] = [Glyph::Eye, Glyph::Shut];
pub const SIGNS: [Glyph; 6] = [Glyph::Fish, Glyph::Crab, Glyph::Anchor, Glyph::Key, Glyph::Moon, Glyph::Star];

impl Glyph {
    pub fn bits(self) -> Bits {
        let r = self.rows();
        // digits are drawn at the left edge; centre them on the board
        let shift = if self.digit().is_some() { 1 } else { 0 };
        let b = from_rows(&r);
        if shift == 0 {
            return b;
        }
        let mut out = 0u64;
        for y in 0..7 {
            let row = (b >> (y * 8)) & 0xff;
            out |= ((row << shift) & 0xff) << (y * 8);
        }
        out
    }

    pub fn from_bits(b: Bits) -> Option<Glyph> {
        Glyph::ALL.iter().copied().find(|g| g.bits() == b)
    }

    pub fn from_id(id: &str) -> Option<Glyph> {
        Glyph::ALL.iter().copied().find(|g| g.id() == id)
    }

    pub fn digit(self) -> Option<u8> {
        DIGITS.iter().position(|d| *d == self).map(|i| i as u8)
    }

    pub fn of_digit(d: u8) -> Glyph {
        DIGITS[(d % 10) as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cpl_crypto::lamps::PICTURE;
    use std::collections::HashSet;

    #[test]
    fn every_glyph_is_distinct_and_leaves_the_seal_row_dark() {
        let mut seen = HashSet::new();
        for g in Glyph::ALL {
            let b = g.bits();
            assert_eq!(b & !PICTURE, 0, "{g:?} uses the bottom row");
            assert!(b.count_ones() >= 8, "{g:?} is too faint");
            assert!(seen.insert(b), "{g:?} repeats another glyph");
            assert_eq!(Glyph::from_bits(b), Some(*g));
            assert_eq!(Glyph::from_id(g.id()), Some(*g));
        }
    }

    /// The two-time-pad puzzles peel one glyph off `a xor b` and expect to see
    /// the other. That only works as a deduction if no third glyph of the
    /// same family also leaves a glyph behind.
    #[test]
    fn peeling_is_unambiguous_within_each_family() {
        for family in [&DIGITS[..], &SIGNS[..], &GATE[..], &ORDERS[..]] {
            for a in family {
                for b in family {
                    if a == b {
                        continue;
                    }
                    let x = a.bits() ^ b.bits();
                    for c in family {
                        let left = x ^ c.bits();
                        match Glyph::from_bits(left) {
                            Some(g) => assert!((c == a && g == *b) || (c == b && g == *a), "{a:?}^{b:?}^{c:?} = {g:?}"),
                            None => assert!(c != a && c != b),
                        }
                    }
                }
            }
        }
    }

    /// A flipped gate must not land on another order by accident: changing one
    /// order into another always takes a deliberate handful of lamps. (Digits
    /// are only ever read, never forged, and a 3 is allowed to resemble an 8.)
    #[test]
    fn orders_in_a_family_differ_in_several_lamps() {
        for family in [&SIGNS[..], &GATE[..], &ORDERS[..], &CAMERA[..]] {
            for a in family {
                for b in family {
                    if a != b {
                        let d = (a.bits() ^ b.bits()).count_ones();
                        assert!(d >= 5, "{a:?} and {b:?} differ in only {d} lamps");
                    }
                }
            }
        }
        for a in DIGITS {
            for b in DIGITS {
                assert!(a == b || (a.bits() ^ b.bits()).count_ones() >= 2);
            }
        }
    }
}
