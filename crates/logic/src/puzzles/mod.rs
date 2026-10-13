//! The puzzles, one module each, in story order.

// prologue
pub mod gateway_login;
pub mod seed_recovery;
// chapter 1: one-time pads and malleability
pub mod bit_flip;
pub mod two_time_pad;
// chapter 2: Diffie–Hellman
pub mod dh_basics;
pub mod logjam;
pub mod mitm_relay;
// chapter 3: secret sharing
pub mod feldman_verify;
pub mod shamir_reconstruct;
// chapter 4: Schnorr signatures, MuSig2, Taproot
pub mod nonce_reuse;
pub mod rogue_key;
pub mod taproot_reveal;
// chapter 5: zero knowledge
pub mod extractor_fs;
pub mod sigma_checkpoint;
pub mod simulator_game;
// chapter 6: lattices and post-quantum
pub mod gauss_reduce;
pub mod knapsack_break;
pub mod lwe_mlkem;
// epilogue
pub mod broadcast;

pub use bit_flip::BitFlip;
pub use broadcast::Broadcast;
pub use dh_basics::DhBasics;
pub use extractor_fs::ExtractorFs;
pub use feldman_verify::FeldmanVerify;
pub use gateway_login::GatewayLogin;
pub use gauss_reduce::GaussReduce;
pub use knapsack_break::KnapsackBreak;
pub use logjam::Logjam;
pub use lwe_mlkem::LweMlkem;
pub use mitm_relay::MitmRelay;
pub use nonce_reuse::NonceReuse;
pub use rogue_key::RogueKey;
pub use seed_recovery::SeedRecovery;
pub use shamir_reconstruct::ShamirReconstruct;
pub use sigma_checkpoint::SigmaCheckpoint;
pub use simulator_game::SimulatorGame;
pub use taproot_reveal::TaprootReveal;
pub use two_time_pad::TwoTimePad;

use std::rc::Rc;

use crate::puzzle::Puzzle;
use crate::world::World;

/// Every puzzle id with a constructor, in story order (used by the tests and
/// the puzzle gallery).
pub const ALL: &[&str] = &[
    "pro.seed",
    "pro.login",
    "ch1.otp",
    "ch1.ctr",
    "ch2.dh",
    "ch2.mitm",
    "ch2.logjam",
    "ch3.shamir",
    "ch3.feldman",
    "ch4.nonce",
    "ch4.rogue",
    "ch4.taproot",
    "ch5.sigma",
    "ch5.sim",
    "ch5.extract",
    "ch6.gauss",
    "ch6.knapsack",
    "ch6.pq",
    "end.broadcast",
];

/// Build a puzzle by id.
pub fn make(id: &str, world: Rc<World>) -> Option<Box<dyn Puzzle>> {
    Some(match id {
        "pro.seed" => Box::new(SeedRecovery::new(world)),
        "pro.login" => Box::new(GatewayLogin::new(world)),
        "ch1.otp" => Box::new(TwoTimePad::new(world)),
        "ch1.ctr" => Box::new(BitFlip::new(world)),
        "ch2.dh" => Box::new(DhBasics::new(world)),
        "ch2.mitm" => Box::new(MitmRelay::new(world)),
        "ch2.logjam" => Box::new(Logjam::new(world)),
        "ch3.shamir" => Box::new(ShamirReconstruct::new(world)),
        "ch3.feldman" => Box::new(FeldmanVerify::new(world)),
        "ch4.nonce" => Box::new(NonceReuse::new(world)),
        "ch4.rogue" => Box::new(RogueKey::new(world)),
        "ch4.taproot" => Box::new(TaprootReveal::new(world)),
        "ch5.sigma" => Box::new(SigmaCheckpoint::new(world)),
        "ch5.sim" => Box::new(SimulatorGame::new(world)),
        "ch5.extract" => Box::new(ExtractorFs::new(world)),
        "ch6.gauss" => Box::new(GaussReduce::new(world)),
        "ch6.knapsack" => Box::new(KnapsackBreak::new(world)),
        "ch6.pq" => Box::new(LweMlkem::new(world)),
        "end.broadcast" => Box::new(Broadcast::new(world)),
        _ => return None,
    })
}
