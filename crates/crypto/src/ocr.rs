//! Checksummed source code on paper: how PGP got out of the United States.
//!
//! Until 1996 the US treated strong cryptography as a munition: putting PGP on
//! a server abroad was an export, and Phil Zimmermann spent three years under
//! criminal investigation for it. Books were different; printed words are
//! speech. So in 1995 MIT Press published *PGP: Source Code and Internals*, the
//! complete source in a font chosen for OCR, every line followed by a
//! checksum. Volunteers in Europe scanned it, and the checksums told them
//! exactly which lines the OCR had got wrong. *Bernstein v. US* later held that
//! source code is speech too.
//!
//! The checksum here is CRC-16/CCITT-FALSE, printed as four hex digits.

use crate::drbg::Drbg;

/// CRC-16/CCITT-FALSE: polynomial 0x1021, initial value 0xFFFF.
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for b in data {
        crc ^= (*b as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
        }
    }
    crc
}

pub fn line_checksum(line: &str) -> String {
    format!("{:04X}", crc16(line.as_bytes()))
}

/// Characters OCR mixes up, in groups. Each character can be read as any
/// other in its group.
pub const CONFUSABLE: &[&str] = &["0Oo", "1lI|", "5S", "8B", "2Z", ";:", ",.", "({", ")}", "ec"];

/// The other readings of a character, if it has any.
pub fn alternatives(c: char) -> Vec<char> {
    for g in CONFUSABLE {
        if g.contains(c) {
            return g.chars().filter(|x| *x != c).collect();
        }
    }
    Vec::new()
}

/// Corrupt `n` characters of a line the way a scanner would (only characters
/// that have a confusable twin).
pub fn corrupt(line: &str, n: usize, rng: &mut Drbg) -> String {
    let mut chars: Vec<char> = line.chars().collect();
    let mut spots: Vec<usize> = (0..chars.len()).filter(|i| !alternatives(chars[*i]).is_empty()).collect();
    rng.shuffle(&mut spots);
    for &i in spots.iter().take(n) {
        let alts = alternatives(chars[i]);
        chars[i] = alts[rng.int(alts.len())];
    }
    chars.into_iter().collect()
}

/// Find readings of `line` (changing up to `max_changes` confusable
/// characters) whose checksum is `crc`. This is what the volunteers' tools
/// did; the game uses it for hints and autoplay.
pub fn repairs(line: &str, crc: u16, max_changes: usize) -> Vec<String> {
    let chars: Vec<char> = line.chars().collect();
    let spots: Vec<usize> = (0..chars.len()).filter(|i| !alternatives(chars[*i]).is_empty()).collect();
    let mut out = Vec::new();
    if crc16(line.as_bytes()) == crc {
        out.push(line.to_string());
    }
    fn go(chars: &mut Vec<char>, spots: &[usize], from: usize, left: usize, crc: u16, out: &mut Vec<String>) {
        if left == 0 || out.len() > 8 {
            return;
        }
        for k in from..spots.len() {
            let i = spots[k];
            let orig = chars[i];
            for alt in alternatives(orig) {
                chars[i] = alt;
                let s: String = chars.iter().collect();
                if crc16(s.as_bytes()) == crc && !out.contains(&s) {
                    out.push(s);
                }
                go(chars, spots, k + 1, left - 1, crc, out);
            }
            chars[i] = orig;
        }
    }
    let mut work = chars.clone();
    go(&mut work, &spots, 0, max_changes, crc, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc16_ccitt_false_check_value() {
        assert_eq!(crc16(b"123456789"), 0x29B1);
        assert_eq!(line_checksum(""), "FFFF");
    }

    #[test]
    fn checksums_find_the_ocr_errors() {
        let mut rng = Drbg::new(b"seed", "ocr");
        let line = "    for (i = 0; i < 16; i++) key[i] ^= pad[i];";
        let crc = crc16(line.as_bytes());
        let bad = corrupt(line, 2, &mut rng);
        assert_ne!(bad, line);
        assert_ne!(crc16(bad.as_bytes()), crc);
        let fixes = repairs(&bad, crc, 2);
        assert!(fixes.contains(&line.to_string()), "{fixes:?}");
    }
}
