//! Pixel-art portraits for the dialogue box and the title screen, painted by
//! cpl-art and uploaded to egui as nearest-neighbour textures.

use std::collections::HashMap;

use cpl_art::canvas::Canvas;
use cpl_art::characters::{all_characters, cat_sheet, character_sheet};
use egui::{ColorImage, TextureHandle, TextureOptions};

#[derive(Default)]
pub struct Portraits {
    cache: HashMap<String, TextureHandle>,
}

fn to_image(c: &Canvas) -> ColorImage {
    ColorImage::from_rgba_unmultiplied([c.w as usize, c.h as usize], &c.data)
}

impl Portraits {
    /// Head-and-shoulders portrait for a character id ("cat" for Nonce).
    pub fn portrait(&mut self, ctx: &egui::Context, id: &str) -> Option<TextureHandle> {
        let key = format!("portrait-{id}");
        if let Some(t) = self.cache.get(&key) {
            return Some(t.clone());
        }
        let canvas = if id == "cat" {
            cat_sheet().crop(0, 0, 16, 16)
        } else {
            let def = all_characters().into_iter().find(|d| d.id == id)?;
            character_sheet(&def).crop(0, 0, 16, 15)
        };
        let tex = ctx.load_texture(&key, to_image(&canvas), TextureOptions::NEAREST);
        self.cache.insert(key, tex.clone());
        Some(tex)
    }

    /// Full-body idle frame (for picking a look).
    pub fn full_body(&mut self, ctx: &egui::Context, id: &str) -> Option<TextureHandle> {
        let key = format!("body-{id}");
        if let Some(t) = self.cache.get(&key) {
            return Some(t.clone());
        }
        let def = all_characters().into_iter().find(|d| d.id == id)?;
        let canvas = character_sheet(&def).crop(0, 0, 16, 24);
        let tex = ctx.load_texture(&key, to_image(&canvas), TextureOptions::NEAREST);
        self.cache.insert(key, tex.clone());
        Some(tex)
    }
}
