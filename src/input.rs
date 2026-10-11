//! One control state for the world, fed by the keyboard, a touch stick, or
//! the autopilot. egui gets first claim on keys and touches.

use bevy::input::touch::Touches;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::input::EguiWantsInput;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

#[derive(Resource, Default)]
pub struct Control {
    /// Movement in [-1, 1]², y down.
    pub axis: Vec2,
    pub run: bool,
    /// Interact pressed this frame.
    pub act: bool,
    /// Set by the autopilot: ignore the keyboard.
    pub override_axis: Option<Vec2>,
    pub override_act: bool,
}

/// The on-screen stick for touch screens.
#[derive(Resource, Default)]
pub struct TouchStick {
    pub active: bool,
    /// Has the player ever touched the screen? (Then show the action button.)
    pub seen: bool,
    pub id: Option<u64>,
    pub origin: Vec2,
    pub at: Vec2,
    /// Action button centre and radius, in logical pixels (set while drawn).
    pub button: Option<(Vec2, f32)>,
    pub tapped: bool,
}

pub fn gather(keys: Res<ButtonInput<KeyCode>>, touches: Res<Touches>, wants: Option<Res<EguiWantsInput>>, windows: Query<&Window, With<PrimaryWindow>>, mut stick: ResMut<TouchStick>, mut c: ResMut<Control>) {
    let typing = wants.as_ref().is_some_and(|w| w.wants_keyboard_input());
    let pointer_ui = wants.as_ref().is_some_and(|w| w.wants_pointer_input());
    let mut axis = Vec2::ZERO;
    if !typing {
        if keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
            axis.x -= 1.0;
        }
        if keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
            axis.x += 1.0;
        }
        if keys.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
            axis.y -= 1.0;
        }
        if keys.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
            axis.y += 1.0;
        }
    }
    let mut act = !typing && keys.any_just_pressed([KeyCode::KeyE, KeyCode::Space, KeyCode::Enter]);
    let run = !typing && keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

    // touch: drag on the left of the screen to walk, tap the button (or the right side) to act
    let width = windows.single().map(|w| w.width()).unwrap_or(1280.0);
    for t in touches.iter_just_pressed() {
        stick.seen = true;
        let p = t.position();
        let on_button = stick.button.is_some_and(|(c, r)| p.distance(c) <= r * 1.3);
        if on_button {
            stick.tapped = true;
        } else if pointer_ui {
            continue;
        } else if p.x < width * 0.5 && !stick.active {
            stick.active = true;
            stick.id = Some(t.id());
            stick.origin = p;
            stick.at = p;
        } else {
            stick.tapped = true;
        }
    }
    if let Some(id) = stick.id {
        match touches.get_pressed(id) {
            Some(t) => stick.at = t.position(),
            None => {
                stick.active = false;
                stick.id = None;
            }
        }
    }
    if stick.active {
        let d = (stick.at - stick.origin) / 40.0;
        let d = if d.length() > 1.0 { d.normalize() } else { d };
        if d.length() > 0.2 {
            axis += d;
        }
    }
    if stick.tapped {
        act = true;
        stick.tapped = false;
    }
    c.axis = axis.clamp(Vec2::splat(-1.0), Vec2::splat(1.0));
    c.run = run || (stick.active && (stick.at - stick.origin).length() > 52.0);
    c.act = act;
    if let Some(a) = c.override_axis {
        c.axis = a;
        c.run = true;
    }
    if c.override_act {
        c.act = true;
        c.override_act = false;
    }
}

/// Draw the touch stick and action button (only once the screen has been touched).
pub fn draw_touch(mut contexts: EguiContexts, mut stick: ResMut<TouchStick>, game: NonSend<crate::director::GameRes>) -> Result {
    if !stick.seen {
        return Ok(());
    }
    let ctx = contexts.ctx_mut()?;
    if game.mode != cpl_logic::game::Mode::World || game.busy() {
        stick.button = None;
        return Ok(());
    }
    let rect = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("touch")));
    let c = egui::pos2(rect.right() - 64.0, rect.bottom() - 84.0);
    let r = 30.0;
    painter.circle(c, r, egui::Color32::from_rgba_unmultiplied(14, 20, 36, 170), egui::Stroke::new(1.5, egui::Color32::from_rgb(62, 240, 255)));
    painter.text(c, egui::Align2::CENTER_CENTER, "E", egui::FontId::new(24.0, egui::FontFamily::Name("retro".into())), egui::Color32::from_rgb(230, 246, 255));
    stick.button = Some((Vec2::new(c.x, c.y), r));
    if stick.active {
        let o = egui::pos2(stick.origin.x, stick.origin.y);
        let d = (stick.at - stick.origin).clamp_length_max(40.0);
        painter.circle_stroke(o, 40.0, egui::Stroke::new(1.5, egui::Color32::from_white_alpha(70)));
        painter.circle_filled(o + egui::vec2(d.x, d.y), 16.0, egui::Color32::from_white_alpha(60));
    }
    Ok(())
}

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Control>().init_resource::<TouchStick>().add_systems(EguiPrimaryContextPass, draw_touch.after(crate::director::draw_ui));
    }
}
