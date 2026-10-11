//! BIP-341 Taproot outputs.
//!
//! An output key Q commits to an internal key P and (optionally) a Merkle tree
//! of scripts:   t = H_TapTweak(P.x || root),   Q = P + t*G.
//! Spending by key path needs only a signature for Q; spending by script path
//! reveals one leaf plus its Merkle path, and nothing about the other leaves.

use crate::bech32::{decode_segwit_v1, segwit_v1_address};
use crate::bytes::{Big, bytes_to_big, concat};
use crate::secp::{N, Pt, has_even_y, lift_x, mul, tagged_hash, x_only};

pub const LEAF_VERSION_TAPSCRIPT: u8 = 0xc0;

pub mod op {
    pub const CHECKSIG: u8 = 0xac;
    pub const CHECKSIGVERIFY: u8 = 0xad;
    pub const CHECKSIGADD: u8 = 0xba;
    pub const CHECKLOCKTIMEVERIFY: u8 = 0xb1;
    pub const CHECKSEQUENCEVERIFY: u8 = 0xb2;
    pub const DROP: u8 = 0x75;
    pub const NUMEQUAL: u8 = 0x9c;
    pub const OP_2: u8 = 0x52;
    pub const OP_3: u8 = 0x53;
}

pub fn compact_size(n: usize) -> Vec<u8> {
    if n < 0xfd {
        vec![n as u8]
    } else if n <= 0xffff {
        vec![0xfd, (n & 0xff) as u8, (n >> 8) as u8]
    } else {
        panic!("script too long for this toy encoder")
    }
}

/// Minimal CScriptNum push (for lock times).
pub fn push_number(v: i64) -> Vec<u8> {
    if v == 0 {
        return vec![0x00];
    }
    if (1..=16).contains(&v) {
        return vec![0x50 + v as u8];
    }
    let mut bytes = Vec::new();
    let mut x = v.unsigned_abs();
    while x > 0 {
        bytes.push((x & 0xff) as u8);
        x >>= 8;
    }
    if bytes.last().unwrap() & 0x80 != 0 {
        bytes.push(if v < 0 { 0x80 } else { 0x00 });
    } else if v < 0 {
        *bytes.last_mut().unwrap() |= 0x80;
    }
    let mut out = vec![bytes.len() as u8];
    out.extend(bytes);
    out
}

pub fn push_key(xonly: &[u8; 32]) -> Vec<u8> {
    concat(&[&[0x20], xonly])
}

#[derive(Clone, Debug)]
pub struct Leaf {
    pub id: String,
    pub label: String,
    pub asm: String,
    pub script: Vec<u8>,
}

pub fn leaf_single_sig(id: &str, label: &str, key_x: &[u8; 32], key_name: &str) -> Leaf {
    Leaf { id: id.into(), label: label.into(), asm: format!("<{key_name}> OP_CHECKSIG"), script: concat(&[&push_key(key_x), &[op::CHECKSIG]]) }
}

pub fn leaf_two_of_two(id: &str, label: &str, a_x: &[u8; 32], a_name: &str, b_x: &[u8; 32], b_name: &str) -> Leaf {
    Leaf {
        id: id.into(),
        label: label.into(),
        asm: format!("<{a_name}> OP_CHECKSIGVERIFY <{b_name}> OP_CHECKSIG"),
        script: concat(&[&push_key(a_x), &[op::CHECKSIGVERIFY], &push_key(b_x), &[op::CHECKSIG]]),
    }
}

pub fn leaf_timelocked(id: &str, label: &str, lock_time: i64, key_x: &[u8; 32], key_name: &str) -> Leaf {
    Leaf {
        id: id.into(),
        label: label.into(),
        asm: format!("{lock_time} OP_CHECKLOCKTIMEVERIFY OP_DROP <{key_name}> OP_CHECKSIG"),
        script: concat(&[&push_number(lock_time), &[op::CHECKLOCKTIMEVERIFY, op::DROP], &push_key(key_x), &[op::CHECKSIG]]),
    }
}

pub fn tap_leaf_hash(script: &[u8], leaf_version: u8) -> [u8; 32] {
    tagged_hash("TapLeaf", &[&[leaf_version], &compact_size(script.len()), script])
}

pub fn tap_branch_hash(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    if a <= b { tagged_hash("TapBranch", &[a, b]) } else { tagged_hash("TapBranch", &[b, a]) }
}

/// A script tree as in the BIP-341 test vectors: a leaf, or a pair of subtrees.
pub enum ScriptTree {
    Leaf { script: Vec<u8>, leaf_version: u8 },
    Branch(Box<ScriptTree>, Box<ScriptTree>),
}

pub fn tree_hash(tree: &ScriptTree) -> [u8; 32] {
    match tree {
        ScriptTree::Leaf { script, leaf_version } => tap_leaf_hash(script, *leaf_version),
        ScriptTree::Branch(a, b) => tap_branch_hash(&tree_hash(a), &tree_hash(b)),
    }
}

#[derive(Clone, Debug)]
pub struct TweakResult {
    pub internal: Pt,
    pub merkle_root: Option<[u8; 32]>,
    pub tweak: Big,
    pub output: Pt,
    pub output_x: [u8; 32],
    pub parity: u8,
    pub address: String,
    pub script_pub_key: Vec<u8>,
}

pub fn tweak_key(internal_x: &[u8; 32], merkle_root: Option<&[u8; 32]>, hrp: &str) -> Result<TweakResult, String> {
    let internal = lift_x(internal_x).ok_or("internal key is not on the curve")?;
    let t_hash = match merkle_root {
        Some(root) => tagged_hash("TapTweak", &[internal_x, root]),
        None => tagged_hash("TapTweak", &[internal_x]),
    };
    let tweak = bytes_to_big(&t_hash);
    if tweak >= *N {
        return Err("tweak out of range".into());
    }
    let output = internal + mul(&tweak);
    let output_x = x_only(&output);
    Ok(TweakResult {
        internal,
        merkle_root: merkle_root.copied(),
        tweak,
        output,
        output_x,
        parity: if has_even_y(&output) { 0 } else { 1 },
        address: taproot_address(&output_x, hrp),
        script_pub_key: concat(&[&[0x51, 0x20], &output_x]),
    })
}

pub fn taproot_address(output_x: &[u8; 32], hrp: &str) -> String {
    segwit_v1_address(hrp, output_x)
}

pub fn decode_taproot_address(addr: &str) -> Result<[u8; 32], String> {
    decode_segwit_v1(addr)
}

pub struct PairAddress<'a> {
    pub a: &'a Leaf,
    pub b: &'a Leaf,
    pub root: [u8; 32],
    pub address: String,
}

/// All the leaf pairs a player could try, with the address each pair would produce.
pub fn pair_addresses<'a>(internal_x: &[u8; 32], leaves: &'a [Leaf]) -> Vec<PairAddress<'a>> {
    let mut out = Vec::new();
    for i in 0..leaves.len() {
        for j in i + 1..leaves.len() {
            let root = tap_branch_hash(&tap_leaf_hash(&leaves[i].script, LEAF_VERSION_TAPSCRIPT), &tap_leaf_hash(&leaves[j].script, LEAF_VERSION_TAPSCRIPT));
            let address = tweak_key(internal_x, Some(&root), "bc").expect("valid key").address;
            out.push(PairAddress { a: &leaves[i], b: &leaves[j], root, address });
        }
    }
    out
}
