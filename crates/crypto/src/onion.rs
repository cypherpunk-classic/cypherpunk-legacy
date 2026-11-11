//! Layered encryption for a chain of remailers, and the traffic analysis that
//! mixing defeats.
//!
//! Cypherpunk ("Type I") remailers in 1992 already let you nest encrypted
//! messages: each remailer strips one layer and learns only the next hop.
//! Mixmaster ("Type II", 1995) added what layers alone cannot give you: fixed
//! packet sizes and batching, so that an observer who sees every message go
//! in and come out still cannot tell which is which.
//!
//! Each layer here is ECIES on secp256k1: a fresh ephemeral key E = e·G, a
//! shared secret x(e·R) with the remailer's public key R, HKDF to an AES-256
//! key, and AES-GCM over `header ‖ inner`. Only the remailer's private key
//! opens a layer, and any tampering fails the tag.

use crate::aead::{GcmBox, gcm_decrypt, gcm_encrypt, kdf};
use crate::bytes::Big;
use crate::drbg::Drbg;
use crate::secp::{N, Pt, compressed, mul, mul_pt, point_from_bytes, x_only};

/// A remailer's keypair.
#[derive(Clone, Debug)]
pub struct Remailer {
    pub name: String,
    pub secret: Big,
    pub public: Pt,
}

impl Remailer {
    pub fn new(name: &str, rng: &mut Drbg) -> Self {
        let secret = rng.scalar(&N);
        Self { name: name.to_string(), public: mul(&secret), secret }
    }
}

/// Where a peeled layer says to send the rest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hop {
    /// forward the inner packet to this remailer
    Forward(String),
    /// deliver the inner bytes (the plaintext) to this address
    Deliver(String),
}

fn encode_hop(h: &Hop) -> Vec<u8> {
    let (tag, name) = match h {
        Hop::Forward(n) => (b'F', n),
        Hop::Deliver(n) => (b'D', n),
    };
    let mut out = vec![tag, name.len() as u8];
    out.extend_from_slice(name.as_bytes());
    out
}

fn decode_hop(b: &[u8]) -> Option<(Hop, &[u8])> {
    let (&tag, rest) = b.split_first()?;
    let (&len, rest) = rest.split_first()?;
    let len = len as usize;
    if rest.len() < len {
        return None;
    }
    let name = String::from_utf8(rest[..len].to_vec()).ok()?;
    let hop = match tag {
        b'F' => Hop::Forward(name),
        b'D' => Hop::Deliver(name),
        _ => return None,
    };
    Some((hop, &rest[len..]))
}

/// Bytes a layer adds: ephemeral key (33), GCM nonce (12), tag (16).
pub const LAYER_OVERHEAD: usize = 33 + 12 + 16;

/// Encrypt one layer for `to`.
pub fn seal_layer(to: &Pt, hop: &Hop, inner: &[u8], rng: &mut Drbg) -> Vec<u8> {
    let e = rng.scalar(&N);
    let eph = mul(&e);
    let key = kdf(&x_only(&mul_pt(&e, to)), "onion layer");
    let nonce: [u8; 12] = rng.array();
    let mut pt = encode_hop(hop);
    pt.extend_from_slice(inner);
    let b = gcm_encrypt(&key, &nonce, &pt, &compressed(&eph));
    let mut out = compressed(&eph).to_vec();
    out.extend_from_slice(&b.nonce);
    out.extend_from_slice(&b.ct);
    out
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeelError {
    Malformed,
    /// wrong key, or the packet was modified
    Tag,
}

/// Strip one layer with a remailer's private key.
pub fn peel_layer(secret: &Big, packet: &[u8]) -> Result<(Hop, Vec<u8>), PeelError> {
    if packet.len() < LAYER_OVERHEAD {
        return Err(PeelError::Malformed);
    }
    let eph = point_from_bytes(&packet[..33]).ok_or(PeelError::Malformed)?;
    let nonce: [u8; 12] = packet[33..45].try_into().unwrap();
    let key = kdf(&x_only(&mul_pt(secret, &eph)), "onion layer");
    let pt = gcm_decrypt(&key, &GcmBox { nonce, ct: packet[45..].to_vec() }, &packet[..33]).map_err(|_| PeelError::Tag)?;
    let (hop, rest) = decode_hop(&pt).ok_or(PeelError::Malformed)?;
    Ok((hop, rest.to_vec()))
}

/// Build an onion along `route` (first hop first) that delivers `message` to `recipient`.
/// Layers are sealed from the inside out: the last remailer's layer first.
pub fn wrap(route: &[&Remailer], recipient: &str, message: &[u8], rng: &mut Drbg) -> Vec<u8> {
    let mut packet = message.to_vec();
    for (i, r) in route.iter().enumerate().rev() {
        let hop = match route.get(i + 1) {
            Some(next) => Hop::Forward(next.name.clone()),
            None => Hop::Deliver(recipient.to_string()),
        };
        packet = seal_layer(&r.public, &hop, &packet, rng);
    }
    packet
}

/// What one remailer saw while relaying.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trace {
    pub remailer: String,
    pub size_in: usize,
    pub hop: Hop,
}

/// Run a packet through the network starting at `first`. Returns the delivered
/// recipient and message plus what each remailer learned.
pub fn route_packet(net: &[Remailer], first: &str, packet: &[u8]) -> Result<(String, Vec<u8>, Vec<Trace>), PeelError> {
    let mut at = first.to_string();
    let mut packet = packet.to_vec();
    let mut traces = Vec::new();
    for _ in 0..16 {
        let r = net.iter().find(|r| r.name == at).ok_or(PeelError::Malformed)?;
        let (hop, inner) = peel_layer(&r.secret, &packet)?;
        traces.push(Trace { remailer: r.name.clone(), size_in: packet.len(), hop: hop.clone() });
        match hop {
            Hop::Forward(next) => {
                at = next;
                packet = inner;
            }
            Hop::Deliver(to) => return Ok((to, inner, traces)),
        }
    }
    Err(PeelError::Malformed)
}

/// Pad a message to a fixed size (so every packet at a given hop looks alike).
pub fn pad(message: &[u8], size: usize) -> Vec<u8> {
    let mut out = (message.len() as u16).to_be_bytes().to_vec();
    out.extend_from_slice(message);
    out.resize(size.max(out.len()), 0);
    out
}

pub fn unpad(padded: &[u8]) -> Option<Vec<u8>> {
    let len = u16::from_be_bytes(padded.get(..2)?.try_into().ok()?) as usize;
    padded.get(2..2 + len).map(<[u8]>::to_vec)
}

// ---------------------------------------------------------------------------
// Traffic analysis
// ---------------------------------------------------------------------------

/// A message seen entering (`t_in`) and leaving (`t_out`) a remailer, as an
/// observer on the wire sees it: times only, the contents are encrypted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sighting {
    pub sender: usize,
    pub t_in: f64,
    pub t_out: f64,
    /// position in the output stream
    pub out_slot: usize,
}

/// How the remailer releases messages.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Policy {
    /// forward each message as soon as it is decrypted (Type I)
    Immediate { delay: f64 },
    /// hold messages until `batch` have arrived, then send them all in random order (a threshold mix)
    Batch { batch: usize },
}

/// Simulate senders 0..n posting at the given times through one remailer.
pub fn simulate(arrivals: &[f64], policy: Policy, rng: &mut Drbg) -> Vec<Sighting> {
    let mut order: Vec<usize> = (0..arrivals.len()).collect();
    order.sort_by(|a, b| arrivals[*a].partial_cmp(&arrivals[*b]).unwrap());
    let mut out: Vec<Sighting> = Vec::new();
    match policy {
        Policy::Immediate { delay } => {
            for &s in &order {
                out.push(Sighting { sender: s, t_in: arrivals[s], t_out: arrivals[s] + delay, out_slot: 0 });
            }
        }
        Policy::Batch { batch } => {
            for chunk in order.chunks(batch.max(1)) {
                let flush = arrivals[*chunk.last().unwrap()] + 0.5;
                let mut shuffled = chunk.to_vec();
                rng.shuffle(&mut shuffled);
                for (i, &s) in shuffled.iter().enumerate() {
                    out.push(Sighting { sender: s, t_in: arrivals[s], t_out: flush + i as f64 * 0.05, out_slot: 0 });
                }
            }
        }
    }
    let mut by_out: Vec<usize> = (0..out.len()).collect();
    by_out.sort_by(|a, b| out[*a].t_out.partial_cmp(&out[*b].t_out).unwrap());
    for (slot, i) in by_out.into_iter().enumerate() {
        out[i].out_slot = slot;
    }
    out
}

/// The analyst's best guess: the k-th message in is the k-th message out.
/// Returns how many senders that links correctly.
pub fn fifo_attack(sightings: &[Sighting]) -> usize {
    let mut by_in: Vec<&Sighting> = sightings.iter().collect();
    by_in.sort_by(|a, b| a.t_in.partial_cmp(&b.t_in).unwrap());
    by_in.iter().enumerate().filter(|(k, s)| s.out_slot == *k).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn net(rng: &mut Drbg) -> Vec<Remailer> {
        ["alpha", "bravo", "charlie"].iter().map(|n| Remailer::new(n, rng)).collect()
    }

    #[test]
    fn three_hops_deliver_and_each_learns_one_step() {
        let mut rng = Drbg::new(b"seed", "onion");
        let n = net(&mut rng);
        let route: Vec<&Remailer> = n.iter().collect();
        let packet = wrap(&route, "press@cryptopolis", b"the documents are real", &mut rng);
        let (to, msg, traces) = route_packet(&n, "alpha", &packet).unwrap();
        assert_eq!(to, "press@cryptopolis");
        assert_eq!(msg, b"the documents are real");
        assert_eq!(traces.len(), 3);
        assert_eq!(traces[0].hop, Hop::Forward("bravo".into()));
        assert_eq!(traces[2].hop, Hop::Deliver("press@cryptopolis".into()));
        // packets shrink by exactly one layer per hop: size reveals position
        assert_eq!(traces[0].size_in - traces[1].size_in, LAYER_OVERHEAD + 2 + "bravo".len());
    }

    #[test]
    fn wrong_key_or_tamper_fails() {
        let mut rng = Drbg::new(b"seed", "onion2");
        let n = net(&mut rng);
        let packet = wrap(&[&n[0], &n[1]], "x", b"hi", &mut rng);
        assert_eq!(peel_layer(&n[1].secret, &packet), Err(PeelError::Tag));
        let mut bad = packet.clone();
        let last = bad.len() - 1;
        bad[last] ^= 1;
        assert_eq!(peel_layer(&n[0].secret, &bad), Err(PeelError::Tag));
        // layers in the wrong order cannot be opened by the first remailer
        let backwards = wrap(&[&n[1], &n[0]], "x", b"hi", &mut rng);
        assert!(route_packet(&n, "alpha", &backwards).is_err());
    }

    #[test]
    fn padding_roundtrips() {
        let p = pad(b"short", 64);
        assert_eq!(p.len(), 64);
        assert_eq!(unpad(&p).unwrap(), b"short");
    }

    #[test]
    fn batching_defeats_fifo_linking() {
        let mut rng = Drbg::new(b"seed", "mix");
        let arrivals: Vec<f64> = (0..24).map(|i| i as f64 * 0.7 + (i % 3) as f64 * 0.1).collect();
        let open = simulate(&arrivals, Policy::Immediate { delay: 0.3 }, &mut rng);
        assert_eq!(fifo_attack(&open), 24, "an immediate remailer leaks every link");
        let mixed = simulate(&arrivals, Policy::Batch { batch: 8 }, &mut rng);
        assert!(fifo_attack(&mixed) < 10, "a threshold mix hides most links");
    }
}
