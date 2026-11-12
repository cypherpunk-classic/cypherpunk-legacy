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

// v3.1: missions from cypherpunk history, side quests, shops
pub mod blind_cash;
pub mod clipper_leaf;
pub mod key_party;
pub mod ledger_chain;
pub mod ocr_book;
pub mod remailer;
pub mod shop;
pub mod source_check;
pub mod stamps;
pub mod tracking_dots;

pub use bit_flip::BitFlip;
pub use blind_cash::BlindCash;
pub use clipper_leaf::ClipperLeaf;
pub use key_party::KeyParty;
pub use ledger_chain::LedgerChain;
pub use ocr_book::OcrBook;
pub use remailer::RemailerPuzzle;
pub use shop::Shop;
pub use source_check::SourceCheck;
pub use stamps::Stamps;
pub use tracking_dots::TrackingDots;
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

use crate::puzzle::Puzzle;
use crate::script::Ctx;

/// Every puzzle id with a constructor, in story order (used by the tests and
/// the puzzle gallery).
pub const ALL: &[&str] = &[
    "pro.seed",
    "pro.login",
    "ch1.otp",
    "ch1.ctr",
    "q.source",
    "ch2.dh",
    "ch2.mitm",
    "ch2.logjam",
    "ch2.book",
    "q.dots",
    "ch3.shamir",
    "ch3.feldman",
    "ch3.cash",
    "q.ledger",
    "party.keys",
    "ch4.nonce",
    "ch4.rogue",
    "ch4.taproot",
    "ch4.clipper",
    "ch5.sigma",
    "ch5.sim",
    "ch5.extract",
    "ch5.remail",
    "ch5.stamps",
    "ch6.gauss",
    "ch6.knapsack",
    "ch6.pq",
    "end.broadcast",
];

/// Build a puzzle by id.
pub fn make(id: &str, ctx: &Ctx) -> Option<Box<dyn Puzzle>> {
    let world = ctx.world();
    Some(match id {
        "pro.seed" => Box::new(SeedRecovery::new(world)),
        "pro.login" => Box::new(GatewayLogin::new(world)),
        "ch1.otp" => Box::new(TwoTimePad::new(world)),
        "ch1.ctr" => Box::new(BitFlip::new(world)),
        "q.source" => Box::new(SourceCheck::new(world)),
        "ch2.dh" => Box::new(DhBasics::new(world)),
        "ch2.mitm" => Box::new(MitmRelay::new(world)),
        "ch2.logjam" => Box::new(Logjam::new(world)),
        "ch2.book" => Box::new(OcrBook::new(world)),
        "q.dots" => Box::new(TrackingDots::new(world)),
        "ch3.shamir" => Box::new(ShamirReconstruct::new(world)),
        "ch3.feldman" => Box::new(FeldmanVerify::new(world)),
        "ch3.cash" => Box::new(BlindCash::new(world)),
        "q.ledger" => Box::new(LedgerChain::new(world)),
        "party.keys" => Box::new(KeyParty::new(world)),
        "ch4.nonce" => Box::new(NonceReuse::new(world)),
        "ch4.rogue" => Box::new(RogueKey::new(world)),
        "ch4.taproot" => Box::new(TaprootReveal::new(world)),
        "ch4.clipper" => Box::new(ClipperLeaf::new(world)),
        "ch5.sigma" => Box::new(SigmaCheckpoint::new(world)),
        "ch5.sim" => Box::new(SimulatorGame::new(world)),
        "ch5.extract" => Box::new(ExtractorFs::new(world)),
        "ch5.remail" => Box::new(RemailerPuzzle::new(ctx.clone())),
        "ch5.stamps" => Box::new(Stamps::new(ctx.clone())),
        "ch6.gauss" => Box::new(GaussReduce::new(world)),
        "ch6.knapsack" => Box::new(KnapsackBreak::new(world)),
        "ch6.pq" => Box::new(LweMlkem::new(world)),
        "end.broadcast" => Box::new(Broadcast::new(world)),
        _ => return None,
    })
}
