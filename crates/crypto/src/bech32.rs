//! Bech32m (BIP-350) for segwit v1 (Taproot) addresses, written out.
//!
//! The checksum is a BCH code over GF(32): `polymod` of the expanded HRP, the
//! data and six zero symbols must equal the constant 0x2bc830a3 (Bech32m; the
//! original Bech32 used 1).

const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
const BECH32M_CONST: u32 = 0x2bc8_30a3;

fn polymod(values: &[u8]) -> u32 {
    const GEN: [u32; 5] = [0x3b6a_57b2, 0x2650_8e6d, 0x1ea1_19fa, 0x3d42_33dd, 0x2a14_62b3];
    let mut chk: u32 = 1;
    for &v in values {
        let top = chk >> 25;
        chk = ((chk & 0x01ff_ffff) << 5) ^ v as u32;
        for (i, g) in GEN.iter().enumerate() {
            if (top >> i) & 1 == 1 {
                chk ^= g;
            }
        }
    }
    chk
}

fn hrp_expand(hrp: &str) -> Vec<u8> {
    let b = hrp.as_bytes();
    let mut v: Vec<u8> = b.iter().map(|c| c >> 5).collect();
    v.push(0);
    v.extend(b.iter().map(|c| c & 31));
    v
}

/// Regroup bits: 8-bit bytes to 5-bit words (or back with `pad = false`).
pub fn convert_bits(data: &[u8], from: u32, to: u32, pad: bool) -> Option<Vec<u8>> {
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    let maxv: u32 = (1 << to) - 1;
    let mut out = Vec::new();
    for &value in data {
        let v = value as u32;
        if v >> from != 0 {
            return None;
        }
        acc = (acc << from) | v;
        bits += from;
        while bits >= to {
            bits -= to;
            out.push(((acc >> bits) & maxv) as u8);
        }
    }
    if pad {
        if bits > 0 {
            out.push(((acc << (to - bits)) & maxv) as u8);
        }
    } else if bits >= from || ((acc << (to - bits)) & maxv) != 0 {
        return None;
    }
    Some(out)
}

pub fn encode(hrp: &str, words: &[u8]) -> String {
    let mut values = hrp_expand(hrp);
    values.extend_from_slice(words);
    values.extend_from_slice(&[0; 6]);
    let pm = polymod(&values) ^ BECH32M_CONST;
    let mut s = String::from(hrp);
    s.push('1');
    for &w in words {
        s.push(CHARSET[w as usize] as char);
    }
    for i in 0..6 {
        s.push(CHARSET[((pm >> (5 * (5 - i))) & 31) as usize] as char);
    }
    s
}

pub fn decode(addr: &str) -> Result<(String, Vec<u8>), String> {
    let lower = addr.to_lowercase();
    if lower != addr && addr.to_uppercase() != addr {
        return Err("mixed case".into());
    }
    let pos = lower.rfind('1').ok_or("no separator")?;
    if pos == 0 || pos + 7 > lower.len() {
        return Err("bad separator position".into());
    }
    let hrp = &lower[..pos];
    let mut words = Vec::new();
    for c in lower[pos + 1..].bytes() {
        words.push(CHARSET.iter().position(|&x| x == c).ok_or("invalid character")? as u8);
    }
    let mut values = hrp_expand(hrp);
    values.extend_from_slice(&words);
    if polymod(&values) != BECH32M_CONST {
        return Err("bad checksum".into());
    }
    words.truncate(words.len() - 6);
    Ok((hrp.to_string(), words))
}

/// A segwit v1 address for a 32-byte witness program.
pub fn segwit_v1_address(hrp: &str, program: &[u8; 32]) -> String {
    let mut words = vec![1u8];
    words.extend(convert_bits(program, 8, 5, true).unwrap());
    encode(hrp, &words)
}

pub fn decode_segwit_v1(addr: &str) -> Result<[u8; 32], String> {
    let (_, words) = decode(addr)?;
    if words.first() != Some(&1) {
        return Err("not a segwit v1 address".into());
    }
    let prog = convert_bits(&words[1..], 5, 8, false).ok_or("bad padding")?;
    prog.try_into().map_err(|_| "program must be 32 bytes".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bip350_valid_strings() {
        for s in ["A1LQFN3A", "a1lqfn3a", "abcdef1l7aum6echk45nj3s0wdvt2fg8x9yrzpqzd3ryx", "split1checkupstagehandshakeupstreamerranterredcaperredlc445v"] {
            assert!(decode(s).is_ok(), "{s}");
        }
        assert!(decode("a1lqfn3q").is_err());
    }
}
