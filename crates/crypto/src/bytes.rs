//! Byte, bit and modular-arithmetic helpers shared by every module.

use num_bigint::{BigInt, Sign};
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};

/// Arbitrary-precision signed integer, the Rust stand-in for JavaScript's `bigint`.
pub type Big = BigInt;

/// Shorthand for small constants: `big(31)`.
pub fn big(x: i64) -> Big {
    Big::from(x)
}

const HEX: &[u8; 16] = b"0123456789abcdef";

pub fn to_hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for &x in b {
        s.push(HEX[(x >> 4) as usize] as char);
        s.push(HEX[(x & 15) as usize] as char);
    }
    s
}

/// Parse hex, tolerating a `0x` prefix and whitespace.
pub fn from_hex(h: &str) -> Result<Vec<u8>, String> {
    let lower = h.trim().to_lowercase();
    let stripped = lower.strip_prefix("0x").unwrap_or(&lower);
    let clean: String = stripped.chars().filter(|c| !c.is_whitespace()).collect();
    if clean.len() % 2 != 0 || !clean.chars().all(|c| c.is_ascii_hexdigit()) {
        let head: String = h.chars().take(16).collect();
        return Err(format!("invalid hex: {head}"));
    }
    (0..clean.len() / 2)
        .map(|i| u8::from_str_radix(&clean[2 * i..2 * i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

pub fn is_hex(h: &str) -> bool {
    let lower = h.trim().to_lowercase();
    let clean = lower.strip_prefix("0x").unwrap_or(&lower);
    clean.len() % 2 == 0 && clean.chars().all(|c| c.is_ascii_hexdigit())
}

/// Fixed-size version of [`from_hex`].
pub fn hex32(h: &str) -> Result<[u8; 32], String> {
    let v = from_hex(h)?;
    v.try_into().map_err(|_| "expected 32 bytes".to_string())
}

/// One character per byte, used for XOR puzzles over ASCII.
pub fn bytes_to_ascii(b: &[u8], unprintable: char) -> String {
    b.iter().map(|&x| if (0x20..0x7f).contains(&x) { x as char } else { unprintable }).collect()
}

/// Latin-1 style encoding: each character's code point, truncated to a byte.
pub fn ascii_to_bytes(s: &str) -> Vec<u8> {
    s.chars().map(|c| (c as u32 & 0xff) as u8).collect()
}

pub fn concat(parts: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::with_capacity(parts.iter().map(|p| p.len()).sum());
    for p in parts {
        out.extend_from_slice(p);
    }
    out
}

pub fn xor(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter().zip(b).map(|(x, y)| x ^ y).collect()
}

/// Constant-time comparison of equal-length byte strings.
pub fn equal_bytes(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}

pub fn bytes_to_bits(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:08b}")).collect()
}

pub fn bits_to_bytes(bits: &str) -> Result<Vec<u8>, String> {
    if bits.len() % 8 != 0 {
        return Err("bit length must be a multiple of 8".into());
    }
    (0..bits.len() / 8)
        .map(|i| u8::from_str_radix(&bits[8 * i..8 * i + 8], 2).map_err(|e| e.to_string()))
        .collect()
}

// ---------------------------------------------------------------------------
// Big <-> bytes
// ---------------------------------------------------------------------------

pub fn bytes_to_big(b: &[u8]) -> Big {
    Big::from_bytes_be(Sign::Plus, b)
}

/// Big-endian encoding padded to `len` bytes. Panics on negative or oversized input.
pub fn big_to_bytes(x: &Big, len: usize) -> Vec<u8> {
    assert!(!x.is_negative(), "negative");
    let (_, raw) = x.to_bytes_be();
    let raw = if x.is_zero() { Vec::new() } else { raw };
    assert!(raw.len() <= len, "integer too large for length");
    let mut out = vec![0u8; len - raw.len()];
    out.extend_from_slice(&raw);
    out
}

pub fn big_to_hex(x: &Big, len: usize) -> String {
    to_hex(&big_to_bytes(x, len))
}

pub fn bit_length(x: &Big) -> u64 {
    x.abs().bits()
}

/// log2 of a positive integer with ~50 bits of precision.
pub fn log2_big(x: &Big) -> f64 {
    if !x.is_positive() {
        return f64::NEG_INFINITY;
    }
    let bl = bit_length(x);
    if bl <= 52 {
        return x.to_f64().unwrap_or(0.0).log2();
    }
    let shift = bl - 52;
    let top: Big = x >> shift;
    top.to_f64().unwrap_or(0.0).log2() + shift as f64
}

// ---------------------------------------------------------------------------
// Modular arithmetic
// ---------------------------------------------------------------------------

/// `a mod m` in `[0, m)`, also for negative `a`.
pub fn modulo(a: &Big, m: &Big) -> Big {
    a.mod_floor(m)
}

/// Floor division, correct for negative numerators.
pub fn floor_div(a: &Big, b: &Big) -> Big {
    a.div_floor(b)
}

/// Nearest integer to a/b (ties round up), exact.
pub fn round_div(a: &Big, b: &Big) -> Big {
    let (a, b) = if b.is_negative() { (-a, -b) } else { (a.clone(), b.clone()) };
    floor_div(&(big(2) * a + &b), &(big(2) * b))
}

pub fn egcd(a: &Big, b: &Big) -> (Big, Big, Big) {
    let (mut old_r, mut r) = (a.clone(), b.clone());
    let (mut old_s, mut s) = (Big::one(), Big::zero());
    let (mut old_t, mut t) = (Big::zero(), Big::one());
    while !r.is_zero() {
        let q = floor_div(&old_r, &r);
        let nr = &old_r - &q * &r;
        old_r = std::mem::replace(&mut r, nr);
        let ns = &old_s - &q * &s;
        old_s = std::mem::replace(&mut s, ns);
        let nt = &old_t - &q * &t;
        old_t = std::mem::replace(&mut t, nt);
    }
    (old_r, old_s, old_t)
}

pub fn mod_inv(a: &Big, m: &Big) -> Option<Big> {
    let (g, x, _) = egcd(&modulo(a, m), m);
    if g == Big::one() || g == -Big::one() { Some(modulo(&(x * g), m)) } else { None }
}

pub fn mod_pow(base: &Big, exp: &Big, m: &Big) -> Big {
    if m.is_one() {
        return Big::zero();
    }
    if exp.is_negative() {
        let inv = mod_inv(base, m).expect("not invertible");
        return inv.modpow(&-exp, m);
    }
    modulo(base, m).modpow(exp, m)
}

pub fn gcd(a: &Big, b: &Big) -> Big {
    a.gcd(b)
}

/// Deterministic Miller–Rabin, adequate for the parameters used in puzzles.
pub fn is_probable_prime(n: &Big) -> bool {
    if n < &big(2) {
        return false;
    }
    let small = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];
    for p in small {
        let p = big(p);
        if n == &p {
            return true;
        }
        if (n % &p).is_zero() {
            return false;
        }
    }
    let n1: Big = n - 1;
    let mut d = n1.clone();
    let mut r = 0;
    while d.is_even() {
        d >>= 1;
        r += 1;
    }
    'outer: for a in small {
        let mut x = big(a).modpow(&d, n);
        if x.is_one() || x == n1 {
            continue;
        }
        for _ in 1..r {
            x = (&x * &x) % n;
            if x == n1 {
                continue 'outer;
            }
        }
        return false;
    }
    true
}

/// Abbreviate long hex for display: `0a1b2c3d…7e8f9a`.
pub fn short_hex(h: &str, head: usize, tail: usize) -> String {
    if h.len() <= head + tail + 1 {
        h.to_string()
    } else {
        format!("{}…{}", &h[..head], &h[h.len() - tail..])
    }
}

/// u64 modular multiplication without overflow.
pub fn mul_mod_u64(a: u64, b: u64, m: u64) -> u64 {
    ((a as u128 * b as u128) % m as u128) as u64
}

pub fn pow_mod_u64(mut base: u64, mut exp: u64, m: u64) -> u64 {
    if m == 1 {
        return 0;
    }
    let mut acc = 1u64;
    base %= m;
    while exp > 0 {
        if exp & 1 == 1 {
            acc = mul_mod_u64(acc, base, m);
        }
        base = mul_mod_u64(base, base, m);
        exp >>= 1;
    }
    acc
}

pub fn inv_mod_i64(a: i64, m: i64) -> Option<i64> {
    let (g, x, _) = egcd(&big(a.rem_euclid(m)), &big(m));
    if g.is_one() { Some(modulo(&x, &big(m)).to_i64().unwrap()) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        assert_eq!(to_hex(&from_hex("0x00ff10").unwrap()), "00ff10");
        assert!(from_hex("abc").is_err());
        assert_eq!(big_to_hex(&big(255), 2), "00ff");
    }

    #[test]
    fn modular_helpers() {
        assert_eq!(modulo(&big(-7), &big(5)), big(3));
        assert_eq!(floor_div(&big(-7), &big(2)), big(-4));
        assert_eq!(round_div(&big(5), &big(2)), big(3));
        assert_eq!(round_div(&big(-5), &big(2)), big(-2));
        assert_eq!(mod_inv(&big(3), &big(31)), Some(big(21)));
        assert_eq!(mod_pow(&big(4), &big(-1), &big(1019)), mod_inv(&big(4), &big(1019)).unwrap());
        assert!(is_probable_prime(&big(2147496341)));
        assert!(!is_probable_prime(&big(1019 * 1021)));
        assert_eq!(pow_mod_u64(4, 509, 1019), 1);
        assert!((log2_big(&(big(1) << 200u32)) - 200.0).abs() < 1e-9);
    }
}
