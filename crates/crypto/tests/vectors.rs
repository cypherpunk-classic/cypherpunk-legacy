//! The crypto layer against official test vectors (BIP-39, BIP-32, BIP-86,
//! BIP-340, BIP-327, BIP-341) plus the attack demonstrations the game relies on.

use cpl_crypto::aead::{ctr_decrypt, ctr_encrypt, flip_bit, forge_ctr, gcm_decrypt_text, gcm_encrypt_text, kdf, GcmBox};
use cpl_crypto::bip32::master_fingerprint;
use cpl_crypto::bip340::{even_key, explain_verify, public_key, recover_from_nonce_reuse, sign, sign_with_nonce, verify, verify_bytes, Sig};
use cpl_crypto::bip39::*;
use cpl_crypto::bytes::{ascii_to_bytes, big, bytes_to_ascii, bytes_to_big, from_hex, to_hex, xor};
use cpl_crypto::drbg::Drbg;
use cpl_crypto::musig::{key_agg, naive_aggregate, rogue_attempt_against_key_agg, rogue_key};
use cpl_crypto::otp::{crib_drag, make_two_time_pad, partner};
use cpl_crypto::secp::{mul, x_only};
use cpl_crypto::taproot::*;
use serde_json::Value;

const ABANDON: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn json(name: &str) -> Value {
    let path = format!("{}/tests/vectors/{name}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn bip39_reference_vectors() {
    let v = json("vectors.json");
    for case in v["english"].as_array().unwrap().iter().take(12) {
        let entropy = case[0].as_str().unwrap();
        let phrase = case[1].as_str().unwrap();
        let seed = case[2].as_str().unwrap();
        assert_eq!(entropy_to_mnemonic(&from_hex(entropy).unwrap()), phrase);
        let words: Vec<&str> = phrase.split(' ').collect();
        assert!(checksum_ok(&words));
        assert_eq!(to_hex(&seed_from_mnemonic(phrase, "TREZOR")), seed);
        let xprv = case[3].as_str().unwrap();
        let m = cpl_crypto::bip32::ExtendedKey::master(&seed_from_mnemonic(phrase, "TREZOR"));
        assert_eq!(m.xprv(), xprv);
    }
}

#[test]
fn bip39_checksum_is_honest() {
    let words: Vec<&str> = ABANDON.split(' ').collect();
    let r = checksum_report(&words).unwrap();
    assert_eq!(r.entropy_bits, "0".repeat(128));
    assert_eq!(r.checksum_bits, r.expected_bits);
    assert_eq!(r.checksum_bits, "0011"); // SHA-256(0^128) starts with 0x37
    let mut broken = words.clone();
    broken[11] = "abandon";
    assert!(!checksum_ok(&broken));

    let mut holes: Vec<Option<&str>> = words.iter().map(|w| Some(*w)).collect();
    holes[5] = None;
    let c = checksum_candidates(&holes, 5);
    assert!(c.contains(&"abandon"));
    assert!(c.len() > 90 && c.len() < 170, "about one word in sixteen fits: {}", c.len());
}

#[test]
fn bip86_vector_and_master_fingerprint() {
    let seed = seed_from_mnemonic(ABANDON, "");
    assert_eq!(master_fingerprint(&seed), "73c5da0a");
    let acct = derive_key(&seed, BIP86_ACCOUNT);
    assert_eq!(acct.xpub, "xpub6BgBgsespWvERF3LHQu6CnqdvfEvtMcQjYrcRzx53QJjSxarj2afYWcLteoGVky7D3UKDP9QyrLprQ3VCECoY49yfdDEHGCtMMj92pReUsQ");
    let k = derive_key(&seed, BIP86_FIRST);
    assert_eq!(to_hex(&k.x_only), "cc8a4bc64d897bddc5fbc2f670f7a8ba0b386779106cf1223c6fc5d7cd6fc115");
    let t = tweak_key(&k.x_only, None, "bc").unwrap();
    assert_eq!(to_hex(&t.output_x), "a60869f0dbcf1dc659c9cecbaf8050135ea9e8cdc487053f1dc6880949dc684c");
    assert_eq!(t.address, "bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr");
    assert!(descriptor(&seed).starts_with("tr([73c5da0a/86h/0h/0h]xpub6BgBgsesp"));
}

#[test]
fn grandpa_seed_has_unique_fingerprint_match() {
    for i in 0..3u8 {
        let g = generate_grandpa_seed(&mut Drbg::new(&[i + 3; 32], "grandpa"));
        let masked: Vec<Option<&str>> = g.words.iter().enumerate().map(|(j, w)| (j != g.missing_index).then_some(*w)).collect();
        let f = recovery_funnel(&masked, g.missing_index, g.first_letter);
        assert!(f.letter_matches.contains(&g.words[g.missing_index]));
        assert!(f.letter_matches.len() >= 3);
        let matching: Vec<&str> = f
            .letter_matches
            .iter()
            .copied()
            .filter(|w| {
                let mut trial = g.words.clone();
                trial[g.missing_index] = w;
                master_fingerprint(&seed_from_mnemonic(&trial.join(" "), "")) == g.fingerprint
            })
            .collect();
        assert_eq!(matching, vec![g.words[g.missing_index]]);
    }
}

#[test]
fn bip340_official_vectors() {
    let csv = std::fs::read_to_string(format!("{}/tests/vectors/test-vectors.csv", env!("CARGO_MANIFEST_DIR"))).unwrap();
    for line in csv.lines().skip(1) {
        let f: Vec<&str> = line.split(',').collect();
        let (sk, pk, aux, msg, sig, result) = (f[1], f[2], f[3], f[4], f[5], f[6]);
        if !sk.is_empty() {
            let aux: [u8; 32] = from_hex(aux).unwrap().try_into().unwrap();
            let s = sign(&from_hex(msg).unwrap(), &bytes_to_big(&from_hex(sk).unwrap()), Some(aux));
            assert_eq!(s.hex().to_uppercase(), sig, "vector {}", f[0]);
        }
        assert_eq!(verify_bytes(&from_hex(sig).unwrap(), &from_hex(msg).unwrap(), &from_hex(pk).unwrap()), result == "TRUE", "vector {}", f[0]);
    }
}

#[test]
fn reused_nonce_gives_the_key_away() {
    let d = bytes_to_big(&from_hex("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef").unwrap());
    let p = public_key(&d);
    let k = bytes_to_big(&from_hex("deadbeefcafebabedeadbeefcafebabedeadbeefcafebabedeadbeefcafebabe").unwrap());
    let (m1, m2) = (b"ORDER 1: SEIZE THE ARCHIVE".as_slice(), b"ORDER 2: MONITOR THE WHARF".as_slice());
    let s1 = sign_with_nonce(m1, &d, &k);
    let s2 = sign_with_nonce(m2, &d, &k);
    assert!(verify(&s1, m1, &p));
    assert!(verify(&s2, m2, &p));
    assert_eq!(s1.r, s2.r);
    let sol = recover_from_nonce_reuse(&p, m1, &s1, m2, &s2).unwrap();
    assert_eq!(sol.d, even_key(&d));
    let forged = b"RELEASE THE ARCHIVE";
    assert!(verify(&sign(forged, &sol.d, None), forged, &p));
    assert!(explain_verify(&s1, m1, &p).unwrap().ok);
    assert!(Sig::parse(&s1.bytes()).unwrap() == s1);
}

#[test]
fn bip327_keyagg_vectors_and_rogue_keys() {
    let v = json("key_agg_vectors.json");
    let pubkeys: Vec<&str> = v["pubkeys"].as_array().unwrap().iter().map(|x| x.as_str().unwrap()).collect();
    for tc in v["valid_test_cases"].as_array().unwrap() {
        let pks: Vec<[u8; 33]> = tc["key_indices"].as_array().unwrap().iter().map(|i| from_hex(pubkeys[i.as_u64().unwrap() as usize]).unwrap().try_into().unwrap()).collect();
        let q = key_agg(&pks).unwrap().q;
        assert_eq!(to_hex(&x_only(&q)).to_uppercase(), tc["expected"].as_str().unwrap());
    }

    let victim = mul(&big(0x1111));
    let attacker = big(0x2222);
    let announced = rogue_key(&mul(&attacker), &[victim]);
    let q = naive_aggregate(&[victim, announced]);
    assert_eq!(q, mul(&attacker));
    let m = b"spend the joint coins";
    assert!(verify(&sign(m, &attacker, None), m, &x_only(&q)));
    for step in rogue_attempt_against_key_agg(&victim, &mul(&big(0x2222)), 5) {
        assert!(!step.hits_target);
    }
}

fn to_tree(t: &Value) -> ScriptTree {
    match t {
        Value::Array(a) => ScriptTree::Branch(Box::new(to_tree(&a[0])), Box::new(to_tree(&a[1]))),
        _ => ScriptTree::Leaf { script: from_hex(t["script"].as_str().unwrap()).unwrap(), leaf_version: t["leafVersion"].as_u64().unwrap() as u8 },
    }
}

fn leaves(t: &Value, out: &mut Vec<(String, u8)>) {
    match t {
        Value::Array(a) => a.iter().for_each(|x| leaves(x, out)),
        _ => out.push((t["script"].as_str().unwrap().to_string(), t["leafVersion"].as_u64().unwrap() as u8)),
    }
}

#[test]
fn bip341_script_pubkey_vectors() {
    let v = json("wallet-test-vectors.json");
    for tc in v["scriptPubKey"].as_array().unwrap() {
        let internal: [u8; 32] = from_hex(tc["given"]["internalPubkey"].as_str().unwrap()).unwrap().try_into().unwrap();
        let tree = &tc["given"]["scriptTree"];
        let root = if tree.is_null() {
            None
        } else {
            let mut ls = Vec::new();
            leaves(tree, &mut ls);
            let hashes: Vec<String> = ls.iter().map(|(s, lv)| to_hex(&tap_leaf_hash(&from_hex(s).unwrap(), *lv))).collect();
            let expected: Vec<String> = tc["intermediary"]["leafHashes"].as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_string()).collect();
            assert_eq!(hashes, expected);
            Some(tree_hash(&to_tree(tree)))
        };
        assert_eq!(root.map(|r| to_hex(&r)), tc["intermediary"]["merkleRoot"].as_str().map(String::from));
        let t = tweak_key(&internal, root.as_ref(), "bc").unwrap();
        assert_eq!(format!("{:0>64}", t.tweak.to_str_radix(16)), tc["intermediary"]["tweak"].as_str().unwrap());
        assert_eq!(to_hex(&t.output_x), tc["intermediary"]["tweakedPubkey"].as_str().unwrap());
        assert_eq!(to_hex(&t.script_pub_key), tc["expected"]["scriptPubKey"].as_str().unwrap());
        assert_eq!(t.address, tc["expected"]["bip350Address"].as_str().unwrap());
        assert_eq!(to_hex(&decode_taproot_address(&t.address).unwrap()), tc["intermediary"]["tweakedPubkey"].as_str().unwrap());
    }
}

#[test]
fn taproot_helpers() {
    assert_eq!(to_hex(&push_number(1_893_456_000)), "0480d8db70");
    assert_eq!(to_hex(&push_number(16)), "60");
    assert_eq!(to_hex(&push_number(128)), "028000");
    let k = |i: i64| x_only(&mul(&big(i)));
    let ls = vec![
        leaf_two_of_two("a", "A", &k(2), "A", &k(3), "B"),
        leaf_timelocked("b", "B", 1_893_456_000, &k(4), "C"),
        leaf_single_sig("c", "C", &k(5), "D"),
        leaf_timelocked("d", "D", 1_924_992_000, &k(6), "E"),
    ];
    let pairs = pair_addresses(&k(7), &ls);
    assert_eq!(pairs.len(), 6);
    let unique: std::collections::HashSet<&String> = pairs.iter().map(|p| &p.address).collect();
    assert_eq!(unique.len(), 6);
}

#[test]
fn one_time_pad_reuse() {
    const P1: &str = "MEET ME AT THE WHARF AT MIDNIGHT";
    const P2: &str = "THE VAULT KEY IS SPLIT FIVE WAYS";
    let t = make_two_time_pad(P1, P2, &mut Drbg::new(&[7; 32], "test"));
    assert_eq!(t.x, xor(&ascii_to_bytes(P1), &ascii_to_bytes(P2)));
    assert_eq!(bytes_to_ascii(&xor(&t.c1, &t.pad), '·'), P1);
    assert_eq!(partner(&t.x, "WHARF", 15), &P2[15..20]);
    let best = crib_drag(&t.x, " THE ").into_iter().max_by(|a, b| a.score.partial_cmp(&b.score).unwrap()).unwrap();
    assert_eq!(best.score, 1.0);
}

#[test]
fn malleability_vs_authenticated_encryption() {
    let key: [u8; 32] = Drbg::new(&[0; 32], "k").array();
    let nonce: [u8; 16] = Drbg::new(&[0; 32], "n").array();
    let ticket = ctr_encrypt(&key, &nonce, "ROLE=GUEST;NAME=ANON");
    let forged = forge_ctr(&ticket, "ROLE=GUEST;NAME=ANON", "ROLE=ADMIN;NAME=ANON").unwrap();
    assert_eq!(ctr_decrypt(&key, &forged), "ROLE=ADMIN;NAME=ANON");

    let n12: [u8; 12] = nonce[..12].try_into().unwrap();
    let b = gcm_encrypt_text(&key, &n12, "ROLE=GUEST;NAME=ANON");
    assert_eq!(gcm_decrypt_text(&key, &b).unwrap(), "ROLE=GUEST;NAME=ANON");
    for bit in [0, 7, 45, 100, b.ct.len() * 8 - 1] {
        assert!(gcm_decrypt_text(&key, &GcmBox { nonce: n12, ct: flip_bit(&b.ct, bit) }).is_err());
    }
    assert_eq!(kdf(b"x", "a"), kdf(b"x", "a"));
    assert_ne!(kdf(b"x", "a"), kdf(b"x", "b"));
}
