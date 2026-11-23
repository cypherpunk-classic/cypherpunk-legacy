//! The codex page lays out as two columns that stack top to bottom, inside
//! a window that stays the size it is. Two bugs once showed it as an empty
//! dark panel: the columns inherited their row's left-to-right direction (the
//! list ran off sideways), and the divider between them took all the height
//! below it, so the centred window grew every frame until its header and
//! footer were far off screen.

use cpl_logic::codex::CODEX;
use cpl_logic::state::GameState;
use cpl_logic::ui::{menu, theme};

/// Every piece of text the menu drew, with where it starts.
fn texts(shapes: &[egui::epaint::ClippedShape]) -> Vec<(egui::Pos2, String)> {
    fn walk(s: &egui::Shape, out: &mut Vec<(egui::Pos2, String)>) {
        match s {
            egui::Shape::Vec(v) => v.iter().for_each(|x| walk(x, out)),
            egui::Shape::Text(t) => out.push((t.pos, t.galley.text().to_string())),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for cs in shapes {
        walk(&cs.shape, &mut out);
    }
    out
}

/// Draw the codex tab for `frames` frames on a `w` x `h` screen; returns
/// the text of frame 10 and of the last frame.
fn draw(w: f32, h: f32, unlocked: usize, frames: usize) -> (Vec<(egui::Pos2, String)>, Vec<(egui::Pos2, String)>) {
    let ctx = egui::Context::default();
    theme::install_fonts(&ctx);
    let mut st = GameState::new();
    st.new_game("layout", 0, Some("layout"));
    for e in CODEX.iter().take(unlocked) {
        st.unlock_codex(e.id);
    }
    let mut m = menu::MenuState::new(menu::Tab::Codex, None, &st);
    let mut sfx = Vec::new();
    let (mut early, mut last) = (Vec::new(), Vec::new());
    for frame in 0..frames {
        if frame == 1 {
            theme::apply_style(&ctx, 1.0);
        }
        let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, h))), time: Some(frame as f64 / 60.0), ..Default::default() };
        ctx.begin_pass(input);
        menu::show(&ctx, &mut m, &st, &mut sfx);
        last = texts(&ctx.end_pass().shapes);
        if frame == 10 {
            early = last.clone();
        }
    }
    (early, last)
}

fn at(texts: &[(egui::Pos2, String)], t: &str) -> egui::Pos2 {
    texts.iter().find(|(_, s)| s == t).map(|(p, _)| *p).unwrap_or_else(|| panic!("{t:?} was not drawn"))
}

#[test]
fn the_codex_window_keeps_its_size_and_stays_on_screen() {
    for (w, h, unlocked) in [(1280.0, 720.0, CODEX.len()), (1567.0, 878.0, 3), (1024.0, 640.0, 0)] {
        let (early, last) = draw(w, h, unlocked, 300);
        let count = format!("{unlocked} / {} entries", CODEX.len());
        for t in ["MENU", "CODEX", count.as_str()] {
            let (a, b) = (at(&early, t), at(&last, t));
            assert!((a.y - b.y).abs() < 0.5, "{t:?} moved from {a:?} to {b:?} on {w}x{h}: the window is growing");
            assert!(b.y >= 0.0 && b.y < h, "{t:?} is off a {w}x{h} screen at {b:?}");
        }
    }
}

#[test]
fn the_codex_lists_top_to_bottom_and_stays_on_screen() {
    let (w, h) = (1280.0, 720.0);
    let ctx = egui::Context::default();
    theme::install_fonts(&ctx);
    let mut st = GameState::new();
    st.new_game("layout", 0, Some("layout"));
    for e in CODEX {
        st.unlock_codex(e.id);
    }
    let mut m = menu::MenuState::new(menu::Tab::Codex, None, &st);
    let mut sfx = Vec::new();
    let mut last = Vec::new();
    for frame in 0..4 {
        if frame == 1 {
            theme::apply_style(&ctx, 1.0);
        }
        let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, h))), time: Some(frame as f64 / 60.0), ..Default::default() };
        ctx.begin_pass(input);
        menu::show(&ctx, &mut m, &st, &mut sfx);
        last = texts(&ctx.end_pass().shapes);
    }
    let at = |t: &str| last.iter().find(|(_, s)| s == t).map(|(p, _)| *p).unwrap_or_else(|| panic!("{t:?} was not drawn"));
    // the list: the first entries of the first group, one under the other, in one column
    let (a, b, c) = (at(CODEX[0].title), at(CODEX[1].title), at(CODEX[2].title));
    assert!(a.y < b.y && b.y < c.y, "the list stacks downwards: {a:?} {b:?} {c:?}");
    assert!((a.x - b.x).abs() < 1.0 && (b.x - c.x).abs() < 1.0, "in one column: {a:?} {b:?} {c:?}");
    // the selected entry, to the right of the list, and the count under both
    let body = last.iter().find(|(_, s)| s.starts_with(&CODEX[0].body[..30])).map(|(p, _)| *p).expect("the entry's text is drawn");
    assert!(body.x > a.x + 150.0, "the entry is in its own column: {body:?}");
    let count = at(&format!("{} / {} entries", CODEX.len(), CODEX.len()));
    assert!(count.y > c.y);
    // nothing escapes the screen
    for (p, s) in &last {
        assert!(p.x >= 0.0 && p.x < w && p.y >= 0.0 && p.y < h, "{s:?} drawn off screen at {p:?}");
    }
}
