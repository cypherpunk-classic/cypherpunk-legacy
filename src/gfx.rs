//! Every texture in the game, painted by `cpl-art` at startup and uploaded
//! once: character sheets, props, buildings, signs, lights, particles, and
//! the baked tile layers of each map. Also the additive material used for
//! lights, so glows brighten what is under them instead of covering it.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::material::descriptor::RenderPipelineDescriptor;
use bevy::material::specialize::SpecializedMeshPipelineError;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, Extent3d, TextureDimension, TextureFormat};
use bevy::sprite_render::{AlphaMode2d, ColorMaterial, ExtendedMaterial2d, Material2dKey, Material2dPipeline, Material2dPlugin, MaterialExtension2d};
use cpl_art::Canvas;
use cpl_art::characters::{FRAME_H, FRAME_W, all_characters, cat_sheet, character_sheet};
use cpl_art::objects::{all_sprites, anim_rate};
use cpl_logic::maps::MapDef;

/// A sprite sheet: one row of equally sized frames.
#[derive(Clone)]
pub struct Sheet {
    pub image: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub frames: usize,
    pub size: UVec2,
    /// Frames per second when the sheet loops on its own.
    pub rate: Option<f32>,
}

impl Sheet {
    pub fn sprite(&self, index: usize) -> Sprite {
        Sprite::from_atlas_image(self.image.clone(), TextureAtlas { layout: self.layout.clone(), index: index.min(self.frames.saturating_sub(1)) })
    }
}

#[derive(Resource)]
pub struct Gfx {
    pub chars: HashMap<String, Sheet>,
    pub cat: Sheet,
    pub objects: HashMap<String, Sheet>,
    pub glows: HashMap<String, Handle<Image>>,
    pub prompt: Sheet,
    pub shadow: Handle<Image>,
    pub dot: Handle<Image>,
    pub raindrop: Handle<Image>,
    /// A unit square for lights (scaled per light).
    pub quad: Handle<Mesh>,
    /// Shared additive materials, by (texture, alpha in thousandths).
    pub additive: HashMap<(String, u32), Handle<GlowMaterial>>,
}

pub fn image_from_canvas(c: &Canvas) -> Image {
    Image::new(
        Extent3d { width: c.w.max(1) as u32, height: c.h.max(1) as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        c.data.clone(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

fn sheet(images: &mut Assets<Image>, layouts: &mut Assets<TextureAtlasLayout>, canvas: &Canvas, fw: u32, fh: u32, rate: Option<f32>) -> Sheet {
    let frames = (canvas.w as u32 / fw).max(1) as usize;
    let layout = TextureAtlasLayout::from_grid(UVec2::new(fw, fh), frames as u32, 1, None, None);
    Sheet { image: images.add(image_from_canvas(canvas)), layout: layouts.add(layout), frames, size: UVec2::new(fw, fh), rate }
}

pub fn build(images: &mut Assets<Image>, layouts: &mut Assets<TextureAtlasLayout>, meshes: &mut Assets<Mesh>) -> Gfx {
    let mut chars = HashMap::new();
    for def in all_characters() {
        let c = character_sheet(&def);
        chars.insert(def.id.to_string(), sheet(images, layouts, &c, FRAME_W as u32, FRAME_H as u32, None));
    }
    let cat = sheet(images, layouts, &cat_sheet(), 16, 16, None);
    let mut objects = HashMap::new();
    for (key, art) in all_sprites() {
        let (fw, fh) = art.frame_size();
        let rate = if art.frames > 1 { anim_rate(&key) } else { None };
        objects.insert(key, sheet(images, layouts, &art.sheet(), fw as u32, fh as u32, rate));
    }
    let mut glows = HashMap::new();
    for (key, r, color) in cpl_art::GLOWS {
        glows.insert(key.to_string(), images.add(image_from_canvas(&cpl_art::glow_texture(*r, color))));
    }
    Gfx {
        chars,
        cat,
        objects,
        glows,
        prompt: sheet(images, layouts, &cpl_art::prompt_sheet(), 11, 12, Some(2.5)),
        shadow: images.add(image_from_canvas(&cpl_art::shadow())),
        dot: images.add(image_from_canvas(&cpl_art::dot())),
        raindrop: images.add(image_from_canvas(&cpl_art::raindrop())),
        quad: meshes.add(Rectangle::new(1.0, 1.0)),
        additive: HashMap::new(),
    }
}

impl Gfx {
    pub fn char_sheet(&self, id: &str) -> &Sheet {
        self.chars.get(id).or_else(|| self.chars.get("player-a")).expect("player sheet")
    }

    pub fn object(&self, key: &str) -> Option<&Sheet> {
        self.objects.get(key)
    }

    /// The additive material for a light texture at a given strength.
    pub fn glow_material(&mut self, materials: &mut Assets<GlowMaterial>, tex: &str, alpha: f32) -> Option<Handle<GlowMaterial>> {
        let image = self.glows.get(tex)?.clone();
        let key = (tex.to_string(), (alpha.clamp(0.0, 4.0) * 1000.0) as u32);
        Some(
            self.additive
                .entry(key)
                .or_insert_with(|| {
                    materials.add(ExtendedMaterial2d {
                        base: ColorMaterial { color: Color::srgba(1.0, 1.0, 1.0, alpha.clamp(0.0, 1.0)), texture: Some(image), alpha_mode: AlphaMode2d::Blend, ..default() },
                        extension: Additive {},
                    })
                })
                .clone(),
        )
    }

    /// The pixel size of a light texture.
    pub fn glow_size(&self, images: &Assets<Image>, tex: &str) -> Vec2 {
        self.glows.get(tex).and_then(|h| images.get(h)).map(|i| i.size_f32()).unwrap_or(Vec2::splat(96.0))
    }
}

/// Bake a map's tile grid into one image (and a second with the water's
/// other animation frame, if the map has water).
pub fn bake_tiles(map: &MapDef) -> (Canvas, Option<Canvas>) {
    let t = cpl_art::tiles::T;
    let mut cache: HashMap<&str, Canvas> = HashMap::new();
    let mut a = Canvas::new(map.w as i32 * t, map.h as i32 * t);
    let mut b: Option<Canvas> = None;
    let has_water = map.tiles.iter().flatten().any(|n| *n == "water");
    if has_water {
        b = Some(Canvas::new(map.w as i32 * t, map.h as i32 * t));
    }
    for (y, row) in map.tiles.iter().enumerate() {
        for (x, name) in row.iter().enumerate() {
            let (px, py) = (x as i32 * t, y as i32 * t);
            let tile = cache.entry(name).or_insert_with(|| cpl_art::tiles::tile(name));
            a.draw(tile, px, py, false);
            if let Some(b) = b.as_mut() {
                let alt = if *name == "water" { "water_b" } else { name };
                let tile = cache.entry(alt).or_insert_with(|| cpl_art::tiles::tile(alt));
                b.draw(tile, px, py, false);
            }
        }
    }
    (a, b)
}

// ---------------------------------------------------------------------------
// Additive blending for lights
// ---------------------------------------------------------------------------

/// `ColorMaterial`, drawn with additive blending: dst + src·alpha.
pub type GlowMaterial = ExtendedMaterial2d<ColorMaterial, Additive>;

#[derive(Asset, AsBindGroup, Reflect, Clone, Debug, Default)]
pub struct Additive {}

impl MaterialExtension2d for Additive {
    fn specialize(_pipeline: &Material2dPipeline, descriptor: &mut RenderPipelineDescriptor, _layout: &MeshVertexBufferLayoutRef, _key: Material2dKey<Self>) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(fragment) = descriptor.fragment.as_mut() {
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = Some(BlendState {
                    color: BlendComponent { src_factor: BlendFactor::SrcAlpha, dst_factor: BlendFactor::One, operation: BlendOperation::Add },
                    alpha: BlendComponent { src_factor: BlendFactor::Zero, dst_factor: BlendFactor::One, operation: BlendOperation::Add },
                });
            }
        }
        Ok(())
    }
}

pub struct GfxPlugin;

impl Plugin for GfxPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<GlowMaterial>::default()).add_systems(PreStartup, setup);
    }
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, mut layouts: ResMut<Assets<TextureAtlasLayout>>, mut meshes: ResMut<Assets<Mesh>>) {
    let gfx = build(&mut images, &mut layouts, &mut meshes);
    commands.insert_resource(gfx);
}
