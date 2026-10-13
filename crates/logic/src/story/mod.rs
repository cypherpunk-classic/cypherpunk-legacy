//! The story: interaction handlers, zone hooks and map-ready hooks, as async
//! scripts over [`Ctx`](crate::script::Ctx).

pub mod ch1;
pub mod ch2;
pub mod ch3;
pub mod ch4;
pub mod ch5;
pub mod ch6;
pub mod city;
pub mod epilogue;
pub mod prologue;

use crate::script::{BoxFut, Ctx};
use crate::speakers::Sp;
use crate::state::MapId;

/// Every interaction id the story knows about.
pub const INTERACTIONS: &[&str] = &[
    "cabin.dresser",
    "cabin.computer",
    "cabin.bookshelf",
    "cabin.photo",
    "cabin.fireplace",
    "cabin.armchair",
    "cabin.bed",
    "cabin.plant",
    "cabin.coat",
    "cabin.boxes",
    "cabin.window",
    "cabin.door",
    "nonce",
    "citizen.1",
    "citizen.2",
    "citizen.3",
    "citizen.4",
    "citizen.5",
    "gate.exchange",
    "gate.commons",
    "gate.mint",
    "gate.masks",
    "gate.obs",
    "wharf.gatepad",
    "wharf.billboard",
    "wharf.radio",
    "wharf.kiosk",
    "wharf.boathouse",
    "plaza.bench",
    "plaza.statue",
    "plaza.vault",
    "street.vending",
    "street.stall",
    "mint.vending",
    "mint.archive",
    "mint.kiosk",
    "warden.holo",
    "mara.talk",
    "felix.talk",
    "exchange.kiosk",
    "juno.talk",
    "commons.dropA",
    "commons.dropB",
    "commons.kiosk",
    "commons.mailbox",
    "ibrahim.talk",
    "lin.talk",
    "masks.checkpoint",
    "masks.agent",
    "masks.kiosk",
    "ada.talk",
    "obs.telescope",
    "obs.whiteboard",
    "obs.kiosk",
];

/// The script for an interaction id.
pub fn interaction(c: Ctx, id: &str) -> BoxFut {
    match id {
        "cabin.dresser" => Box::pin(prologue::dresser(c)),
        "cabin.computer" => Box::pin(prologue::computer(c)),
        "cabin.bookshelf" => Box::pin(prologue::bookshelf(c)),
        "cabin.photo" => Box::pin(prologue::photo(c)),
        "cabin.fireplace" => Box::pin(prologue::fireplace(c)),
        "cabin.armchair" => Box::pin(prologue::armchair(c)),
        "cabin.bed" => Box::pin(prologue::bed(c)),
        "cabin.plant" => Box::pin(prologue::plant(c)),
        "cabin.coat" => Box::pin(prologue::coat(c)),
        "cabin.boxes" => Box::pin(prologue::boxes(c)),
        "cabin.window" => Box::pin(prologue::window(c)),
        "cabin.door" => Box::pin(prologue::door(c)),
        "nonce" => {
            if c.current_map() == MapId::Cabin {
                Box::pin(prologue::pet_nonce_cabin(c))
            } else {
                Box::pin(city::pet_nonce_city(c))
            }
        }
        "citizen.1" => Box::pin(city::citizen(c, 1)),
        "citizen.2" => Box::pin(city::citizen(c, 2)),
        "citizen.3" => Box::pin(city::citizen(c, 3)),
        "citizen.4" => Box::pin(city::citizen(c, 4)),
        "citizen.5" => Box::pin(city::citizen(c, 5)),
        "gate.exchange" => Box::pin(city::gate(c, "exchange")),
        "gate.commons" => Box::pin(city::gate(c, "commons")),
        "gate.mint" => Box::pin(city::gate(c, "mint")),
        "gate.masks" => Box::pin(city::gate(c, "masks")),
        "gate.obs" => Box::pin(city::gate(c, "obs")),
        "wharf.gatepad" => Box::pin(city::gatepad(c)),
        "wharf.billboard" => Box::pin(city::billboard(c)),
        "wharf.radio" => Box::pin(ch1::radio(c)),
        "wharf.kiosk" => Box::pin(ch1::kiosk(c)),
        "wharf.boathouse" => Box::pin(ch1::boathouse(c)),
        "plaza.bench" => Box::pin(city::bench(c)),
        "plaza.statue" => Box::pin(city::statue(c)),
        "plaza.vault" => Box::pin(epilogue::vault(c)),
        "street.vending" => Box::pin(city::street_vending(c)),
        "street.stall" => Box::pin(city::stall(c)),
        "mint.vending" => Box::pin(ch4::vending(c)),
        "mint.archive" => Box::pin(ch4::archive(c)),
        "mint.kiosk" => Box::pin(ch4::kiosk(c)),
        "warden.holo" => Box::pin(city::warden_holo(c)),
        "mara.talk" => Box::pin(ch1::mara(c)),
        "felix.talk" => Box::pin(ch2::felix(c)),
        "exchange.kiosk" => Box::pin(ch2::kiosk(c)),
        "juno.talk" => Box::pin(ch3::juno(c)),
        "commons.dropA" => Box::pin(ch3::drop_a(c)),
        "commons.dropB" => Box::pin(ch3::drop_b(c)),
        "commons.kiosk" => Box::pin(ch3::kiosk(c)),
        "commons.mailbox" => Box::pin(ch3::mailbox(c)),
        "ibrahim.talk" => Box::pin(ch4::ibrahim(c)),
        "lin.talk" => Box::pin(ch5::lin(c)),
        "masks.checkpoint" => Box::pin(ch5::checkpoint(c)),
        "masks.agent" => Box::pin(ch5::agent(c)),
        "masks.kiosk" => Box::pin(ch5::kiosk(c)),
        "ada.talk" => Box::pin(ch6::ada(c)),
        "obs.telescope" => Box::pin(ch6::telescope(c)),
        "obs.whiteboard" => Box::pin(ch6::whiteboard(c)),
        "obs.kiosk" => Box::pin(ch6::kiosk(c)),
        _ => Box::pin(async move { c.say1(Sp::Narrator, "Nothing happens.").await }),
    }
}

/// The player entered a zone.
pub fn zone(c: Ctx, id: &str) -> BoxFut {
    Box::pin(city::city_zone(c, id.to_string()))
}

/// A map finished loading.
pub fn ready(c: Ctx, map: MapId) -> BoxFut {
    match map {
        MapId::Cabin => Box::pin(prologue::cabin_ready(c)),
        MapId::City => Box::pin(city::city_ready(c)),
    }
}
