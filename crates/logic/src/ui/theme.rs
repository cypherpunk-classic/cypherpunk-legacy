//! Colours, fonts and egui styling: a dark terminal look with VT323 for
//! headings and IBM Plex Mono for everything you read.

use std::sync::Arc;

use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Stroke, Visuals};

pub const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

pub fn color(hex: &str) -> Color32 {
    let h = hex.trim_start_matches('#');
    let v = u32::from_str_radix(&h[..6.min(h.len())], 16).unwrap_or(0xffffff);
    if h.len() == 8 {
        let a = u8::from_str_radix(&h[6..8], 16).unwrap_or(255);
        Color32::from_rgba_unmultiplied((v >> 16) as u8, (v >> 8) as u8, v as u8, a)
    } else {
        rgb(v)
    }
}

pub const BG: Color32 = rgb(0x0b0d16);
pub const BG2: Color32 = rgb(0x0f1322);
pub const PANEL: Color32 = Color32::from_rgb(12, 16, 28);
pub const PANEL2: Color32 = Color32::from_rgb(19, 25, 42);
pub const LINE: Color32 = rgb(0x2a3452);
pub const LINE2: Color32 = rgb(0x3a4870);
pub const CYAN: Color32 = rgb(0x3ef0ff);
pub const CYAN_DIM: Color32 = rgb(0x1c7f8a);
pub const MAGENTA: Color32 = rgb(0xff3e9a);
pub const AMBER: Color32 = rgb(0xffb347);
pub const GREEN: Color32 = rgb(0x5dff9a);
pub const RED: Color32 = rgb(0xff5566);
pub const VIOLET: Color32 = rgb(0xc792ff);
pub const TEXT: Color32 = rgb(0xd7e3f4);
pub const DIM: Color32 = rgb(0x8a96b0);
pub const FAINT: Color32 = rgb(0x56627d);
pub const GOLD: Color32 = rgb(0xffcd75);

/// The retro display face (VT323).
pub fn retro_family() -> FontFamily {
    FontFamily::Name("retro".into())
}

pub fn retro(size: f32) -> FontId {
    FontId::new(size, retro_family())
}

pub fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

pub fn bold_family() -> FontFamily {
    FontFamily::Name("bold".into())
}

pub fn italic_family() -> FontFamily {
    FontFamily::Name("italic".into())
}

static VT323: &[u8] = include_bytes!("../../assets/fonts/VT323-Regular.ttf");
static PLEX: &[u8] = include_bytes!("../../assets/fonts/IBMPlexMono-Regular.ttf");
static PLEX_BOLD: &[u8] = include_bytes!("../../assets/fonts/IBMPlexMono-Bold.ttf");
static PLEX_ITALIC: &[u8] = include_bytes!("../../assets/fonts/IBMPlexMono-Italic.ttf");
/// Greek, sub/superscripts, arrows and maths operators that Plex Mono lacks
/// (a subset of DejaVu Sans Mono).
static SYMBOLS: &[u8] = include_bytes!("../../assets/fonts/DejaVuSansMono-Symbols.ttf");

/// Register the game's fonts. egui's bundled fonts stay as fallbacks for
/// symbols (arrows, check marks, block shades) that the subsets lack.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert("vt323".into(), Arc::new(FontData::from_static(VT323)));
    fonts.font_data.insert("plex".into(), Arc::new(FontData::from_static(PLEX)));
    fonts.font_data.insert("plex-bold".into(), Arc::new(FontData::from_static(PLEX_BOLD)));
    fonts.font_data.insert("plex-italic".into(), Arc::new(FontData::from_static(PLEX_ITALIC)));
    fonts.font_data.insert("symbols".into(), Arc::new(FontData::from_static(SYMBOLS)));
    let mut fallbacks: Vec<String> = vec!["symbols".into()];
    fallbacks.extend(fonts.families.get(&FontFamily::Monospace).cloned().unwrap_or_default());
    let with = |first: &str| -> Vec<String> {
        let mut v = vec![first.to_string()];
        v.extend(fallbacks.iter().cloned());
        v
    };
    fonts.families.insert(FontFamily::Monospace, with("plex"));
    fonts.families.insert(FontFamily::Proportional, with("plex"));
    fonts.families.insert(retro_family(), with("vt323"));
    fonts.families.insert(bold_family(), with("plex-bold"));
    fonts.families.insert(italic_family(), with("plex-italic"));
    ctx.set_fonts(fonts);
}

/// Dark, square, terminal-flavoured egui style.
pub fn apply_style(ctx: &egui::Context, scale: f32) {
    ctx.set_visuals(visuals());
    ctx.all_styles_mut(|style| {
        use egui::TextStyle::*;
        let s = scale.clamp(0.75, 1.6);
        style.text_styles = [
            (Heading, retro(26.0 * s)),
            (Body, mono(14.5 * s)),
            (Monospace, mono(14.0 * s)),
            (Button, mono(14.0 * s)),
            (Small, mono(12.0 * s)),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 4.0);
        style.spacing.interact_size.y = 26.0;
        style.spacing.window_margin = Margin::same(12);
        style.interaction.selectable_labels = false;
    });
}

fn visuals() -> Visuals {
    let mut v = Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = PANEL;
    v.extreme_bg_color = BG;
    v.faint_bg_color = BG2;
    v.code_bg_color = rgb(0x070910);
    v.window_stroke = Stroke::new(1.0, LINE2);
    v.window_corner_radius = CornerRadius::same(4);
    v.menu_corner_radius = CornerRadius::same(3);
    v.override_text_color = Some(TEXT);
    v.hyperlink_color = CYAN;
    v.selection.bg_fill = CYAN_DIM;
    v.selection.stroke = Stroke::new(1.0, CYAN);
    v.text_cursor.stroke = Stroke::new(2.0, CYAN);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = PANEL2;
    w.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
    w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    w.noninteractive.corner_radius = CornerRadius::same(3);
    w.inactive.bg_fill = rgb(0x151c30);
    w.inactive.weak_bg_fill = rgb(0x151c30);
    w.inactive.bg_stroke = Stroke::new(1.0, LINE2);
    w.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    w.inactive.corner_radius = CornerRadius::same(3);
    w.hovered.bg_fill = rgb(0x1b2644);
    w.hovered.weak_bg_fill = rgb(0x1b2644);
    w.hovered.bg_stroke = Stroke::new(1.0, CYAN);
    w.hovered.fg_stroke = Stroke::new(1.0, CYAN);
    w.hovered.corner_radius = CornerRadius::same(3);
    w.active.bg_fill = rgb(0x0f3b44);
    w.active.weak_bg_fill = rgb(0x0f3b44);
    w.active.bg_stroke = Stroke::new(1.0, CYAN);
    w.active.fg_stroke = Stroke::new(1.0, CYAN);
    w.active.corner_radius = CornerRadius::same(3);
    w.open = w.hovered;
    v
}

/// Translucent dark panel frame.
pub fn panel_frame() -> egui::Frame {
    egui::Frame::NONE.fill(PANEL).stroke(Stroke::new(1.0, LINE2)).corner_radius(CornerRadius::same(4)).inner_margin(Margin::same(12))
}

pub fn card_frame(accent: Option<Color32>) -> egui::Frame {
    egui::Frame::NONE
        .fill(rgb(0x0d1220))
        .stroke(Stroke::new(1.0, accent.unwrap_or(LINE)))
        .corner_radius(CornerRadius::same(3))
        .inner_margin(Margin::symmetric(12, 10))
}
