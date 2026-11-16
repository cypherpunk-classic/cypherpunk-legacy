//! Signal boards: 64 lamps in an 8 x 8 grid, used as a one-time pad channel.
//!
//! A board shows `C = M xor K`: the message `M` is a picture (a glyph in the
//! top seven rows), the pad `K` is 64 random bits that sender and lock share.
//! That is Shannon's perfect secrecy over GF(2)^64, and everything that goes
//! wrong with it in the field goes wrong here too:
//!
//! * flip a lamp of `C` and the same lamp of `M` flips at the lock, key or no
//!   key (malleability);
//! * two boards on one pad give `C1 xor C2 = M1 xor M2` (the two-time pad);
//! * one known message gives the pad: `K = C xor M` (known plaintext);
//! * an old `C` is as valid as a new one (replay).
//!
//! A sealed lock also checks an 8-bit tag in the bottom row, the first byte of
//! HMAC-SHA256 over the picture. Eight bits is a toy size (one blind guess in
//! 256 passes); the construction is the real one.

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

use crate::drbg::Drbg;

/// A board, a card or a pad: bit `y * 8 + x`, row 0 at the top.
pub type Bits = u64;

/// The seven rows a glyph may use.
pub const PICTURE: Bits = (1 << 56) - 1;
/// The bottom row, where a sealed board carries its tag.
pub const SEAL_ROW: Bits = !PICTURE;

/// 64 fresh pad bits.
pub fn pad(rng: &mut Drbg) -> Bits {
    u64::from_le_bytes(rng.array::<8>())
}

/// The 8-bit seal of a picture under `key`.
pub fn tag(key: &[u8; 16], picture: Bits) -> u8 {
    let mut m = <Hmac<Sha256> as KeyInit>::new_from_slice(key).expect("any key length");
    m.update(&(picture & PICTURE).to_le_bytes());
    m.finalize().into_bytes()[0]
}

/// A picture with its seal in the bottom row.
pub fn sealed(key: &[u8; 16], picture: Bits) -> Bits {
    (picture & PICTURE) | ((tag(key, picture) as u64) << 56)
}

/// Does the bottom row carry the right seal for the rows above it?
pub fn seal_ok(key: &[u8; 16], plain: Bits) -> bool {
    (plain >> 56) as u8 == tag(key, plain)
}

/// Read eight strings of `#` and `.` as a board.
pub fn from_rows(rows: &[&str]) -> Bits {
    let mut b = 0u64;
    for (y, row) in rows.iter().enumerate().take(8) {
        for (x, ch) in row.chars().enumerate().take(8) {
            if ch == '#' {
                b |= 1 << (y * 8 + x);
            }
        }
    }
    b
}

pub fn lamp(b: Bits, x: usize, y: usize) -> bool {
    b >> (y * 8 + x) & 1 == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pad_cancels_and_a_flip_goes_straight_through() {
        let mut rng = Drbg::new(b"lamps", "t");
        let k = pad(&mut rng);
        let (m1, m2) = (from_rows(&["##......", "#......."]), from_rows(&["....##..", ".....#.."]));
        let (c1, c2) = (m1 ^ k, m2 ^ k);
        assert_eq!(c1 ^ c2, m1 ^ m2, "two-time pad");
        assert_eq!(c1 ^ m1, k, "known plaintext gives the pad");
        let delta = m1 ^ m2;
        assert_eq!((c1 ^ delta) ^ k, m2, "flipping the cipher rewrites the message");
    }

    #[test]
    fn a_seal_notices_any_change_to_the_picture() {
        let key = [7u8; 16];
        let m = from_rows(&["..####..", ".#....#."]);
        let s = sealed(&key, m);
        assert!(seal_ok(&key, s));
        assert_eq!(s & PICTURE, m);
        let mut caught = 0;
        for bit in 0..56 {
            if !seal_ok(&key, s ^ (1 << bit)) {
                caught += 1;
            }
        }
        // an 8-bit tag misses about one change in 256
        assert!(caught >= 53, "{caught} of 56 single flips caught");
        assert!(!seal_ok(&[8u8; 16], s) || tag(&[8u8; 16], m) == tag(&key, m));
    }

    #[test]
    fn rows_read_left_to_right_top_to_bottom() {
        let b = from_rows(&["#......."]);
        assert!(lamp(b, 0, 0) && !lamp(b, 1, 0));
        assert_eq!(from_rows(&["........", ".#......"]), 1 << 9);
    }
}
