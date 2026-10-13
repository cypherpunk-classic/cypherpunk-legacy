//! The camera (integer zoom, follow, pans, shake) and full-screen overlays
//! (fades between maps, flashes, the optional CRT scanlines).

use bevy::camera::ScalingMode;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

use crate::director::GameRes;
use crate::world::{Level, Player, Pos};

pub enum CamMode {
    Follow,
    Pan { from: Vec2, to: Vec2, t: f32, dur: f32, reply: u64 },
    /// Stay where a pan ended until told to follow again.
    Hold(Vec2),
    Title,
}

#[derive(Resource)]
pub struct CameraCtl {
    pub mode: CamMode,
    /// Centre of the view in map space (pixels, y down).
    pub center: Vec2,
    /// Jump straight to the target next frame.
    pub snap: bool,
    pub shake: f32,
    /// Logical pixels per texel.
    pub zoom: f32,
    /// Pan replies to send.
    pub finished: Vec<u64>,
}

impl Default for CameraCtl {
    fn default() -> Self {
        Self { mode: CamMode::Title, center: Vec2::ZERO, snap: true, shake: 0.0, zoom: 3.0, finished: Vec::new() }
    }
}

#[derive(Resource, Default)]
pub struct Screen {
    /// 0 = clear, 1 = black.
    pub fade: f32,
    pub flash: Option<(egui::Color32, f32, f32)>,
    pub time: f64,
}

impl Screen {
    pub fn flash(&mut self, color: u32, secs: f32) {
        let c = egui::Color32::from_rgb((color >> 16) as u8, (color >> 8) as u8, color as u8);
        self.flash = Some((c, 0.0, secs.max(0.05)));
    }
}

#[derive(Component)]
pub struct MainCamera;

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera2d, MainCamera, Msaa::Off, Projection::Orthographic(OrthographicProjection { scaling_mode: ScalingMode::WindowSize, scale: 1.0 / 3.0, ..OrthographicProjection::default_2d() })));
}

fn ease_in_out(t: f32) -> f32 {
    -(std::f32::consts::PI * t).cos() / 2.0 + 0.5
}

/// Follow the player (or pan, or hover over the title skyline), clamp to the
/// map, snap to whole device pixels.
pub fn update_camera(
    time: Res<Time>,
    ctl: ResMut<CameraCtl>,
    level: Res<Level>,
    windows: Query<&Window, With<PrimaryWindow>>,
    player: Query<&Pos, With<Player>>,
    mut cams: Query<(&mut Transform, &mut Projection), With<MainCamera>>,
) {
    let Ok(window) = windows.single() else { return };
    let Ok((mut tf, mut proj)) = cams.single_mut() else { return };
    let ctl = ctl.into_inner();
    let dt = time.delta_secs().min(0.1);
    let (lw, lh) = (window.width().max(1.0), window.height().max(1.0));
    let sf = window.scale_factor().max(0.5);
    let title = matches!(ctl.mode, CamMode::Title);
    // integer zoom, chosen in logical pixels and made whole in device pixels
    let zoom = if title { (lw / 320.0).min(lh / 200.0).floor().max(2.0) } else { (lw / 300.0).min(lh / 190.0).floor().clamp(2.0, 6.0) };
    let texel = (zoom * sf).round().max(1.0);
    ctl.zoom = texel / sf;
    if let Projection::Orthographic(o) = proj.as_mut() {
        o.scale = sf / texel;
    }
    let view = Vec2::new(lw, lh) / ctl.zoom;

    let prev = ctl.center;
    let target = match &mut ctl.mode {
        CamMode::Title => Vec2::new(0.0, -40.0),
        CamMode::Hold(p) => *p,
        CamMode::Follow => player.single().map(|p| p.p - Vec2::new(0.0, 8.0)).unwrap_or(prev),
        CamMode::Pan { from, to, t, dur, .. } => {
            *t += dt;
            let k = ease_in_out((*t / *dur).min(1.0));
            from.lerp(*to, k)
        }
    };
    if let CamMode::Pan { t, dur, reply, to, .. } = ctl.mode
        && t >= dur
    {
        ctl.finished.push(reply);
        ctl.center = to;
        ctl.mode = CamMode::Hold(to);
    }
    let mut center = if ctl.snap || !matches!(ctl.mode, CamMode::Follow) {
        target
    } else {
        let k = 1.0 - (1.0f32 - 0.18).powf(dt * 60.0);
        prev.lerp(target, k)
    };
    ctl.snap = false;
    // keep the view on the map (or centre a map smaller than the view)
    let size = level.size();
    if !title && size.x > 0.0 {
        let clamp_axis = |c: f32, view: f32, size: f32| if size <= view { size / 2.0 } else { c.clamp(view / 2.0, size - view / 2.0) };
        center = Vec2::new(clamp_axis(center.x, view.x, size.x), clamp_axis(center.y, view.y, size.y));
    }
    ctl.center = center;
    let mut shown = center;
    if ctl.shake > 0.0 {
        ctl.shake -= dt;
        let a = view.x * 0.006;
        let t = time.elapsed_secs() * 60.0;
        shown += Vec2::new((t * 1.7).sin(), (t * 2.3).cos()) * a;
    }
    // snap to the device pixel grid; half a pixel off when the window has an odd size
    let px = 1.0 / texel;
    let odd = |n: u32| if n % 2 == 1 { 0.5 * px } else { 0.0 };
    let sx = (shown.x / px).round() * px + odd(window.physical_width());
    let sy = (shown.y / px).round() * px + odd(window.physical_height());
    tf.translation = Vec3::new(sx, -sy, tf.translation.z);
}

/// Fades, flashes and scanlines, drawn by egui beneath every window.
pub fn draw_overlays(mut contexts: EguiContexts, time: Res<Time>, mut screen: ResMut<Screen>, game: NonSend<GameRes>) -> Result {
    let ctx = contexts.ctx_mut()?;
    let dt = time.delta_secs();
    screen.time += dt as f64;
    let rect = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());
    if let Some((c, t, dur)) = screen.flash.as_mut() {
        *t += dt;
        let a = (1.0 - *t / *dur).clamp(0.0, 1.0);
        painter.rect_filled(rect, 0.0, c.gamma_multiply(a * 0.85));
        if *t >= *dur {
            screen.flash = None;
        }
    }
    if game.ctx.state_ref().settings.crt {
        let line = egui::Color32::from_black_alpha(34);
        let mut y = rect.top();
        while y < rect.bottom() {
            painter.rect_filled(egui::Rect::from_min_size(egui::pos2(rect.left(), y), egui::vec2(rect.width(), 1.0)), 0.0, line);
            y += 3.0;
        }
    }
    if screen.fade > 0.0 {
        painter.rect_filled(rect, 0.0, egui::Color32::from_black_alpha((screen.fade.clamp(0.0, 1.0) * 255.0) as u8));
    }
    Ok(())
}

pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraCtl>().init_resource::<Screen>().add_systems(Startup, spawn_camera).add_systems(EguiPrimaryContextPass, draw_overlays.before(crate::director::draw_ui));
    }
}
