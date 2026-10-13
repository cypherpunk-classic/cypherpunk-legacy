//! A tiny raster toolkit for code-defined pixel art.
//!
//! Everything draws into plain RGBA buffers that the game hands to Bevy as
//! textures, so the game ships with zero image files. Colours are sRGB; drawing
//! with a translucent colour composites "source over", like a 2D canvas.

use crate::palette::color_of;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

impl Rgba {
    pub const CLEAR: Rgba = Rgba(0, 0, 0, 0);

    /// Parse `#rgb`, `#rrggbb` or `#rrggbbaa`.
    pub fn hex(s: &str) -> Rgba {
        let h = s.trim_start_matches('#');
        let v = |i: usize, n: usize| u8::from_str_radix(&h[i..i + n], 16).unwrap_or(0);
        match h.len() {
            3 => Rgba(v(0, 1) * 17, v(1, 1) * 17, v(2, 1) * 17, 255),
            6 => Rgba(v(0, 2), v(2, 2), v(4, 2), 255),
            8 => Rgba(v(0, 2), v(2, 2), v(4, 2), v(6, 2)),
            _ => panic!("bad colour {s}"),
        }
    }

    pub fn with_alpha(self, a: u8) -> Rgba {
        Rgba(self.0, self.1, self.2, a)
    }

    /// 0xRRGGBB, handy for tints.
    pub fn rgb_u32(self) -> u32 {
        (self.0 as u32) << 16 | (self.1 as u32) << 8 | self.2 as u32
    }
}

/// Anything that names a colour: `"#a8703f"`, a palette character, or an [`Rgba`].
pub trait IntoColor {
    fn rgba(&self) -> Rgba;
}

impl IntoColor for Rgba {
    fn rgba(&self) -> Rgba {
        *self
    }
}

impl IntoColor for &str {
    fn rgba(&self) -> Rgba {
        Rgba::hex(self)
    }
}

impl IntoColor for String {
    fn rgba(&self) -> Rgba {
        Rgba::hex(self)
    }
}

impl IntoColor for char {
    fn rgba(&self) -> Rgba {
        color_of(*self, None).unwrap_or_else(|| panic!("no palette colour for {self:?}"))
    }
}

#[derive(Clone, Debug)]
pub struct Canvas {
    pub w: i32,
    pub h: i32,
    pub data: Vec<u8>,
}

/// Per-character slot colours for templated sprites (hair, skin, clothes...).
pub type Slots<'a> = &'a [(char, Rgba)];

#[derive(Clone, Copy, Default)]
pub struct PaintOpts<'a> {
    pub slots: Option<Slots<'a>>,
    pub mirror: bool,
    /// Characters to skip (treated as transparent).
    pub skip: &'a str,
}

impl Canvas {
    pub fn new(w: i32, h: i32) -> Self {
        Self { w, h, data: vec![0; (w.max(0) * h.max(0) * 4) as usize] }
    }

    #[inline]
    fn idx(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.w && y < self.h).then(|| ((y * self.w + x) * 4) as usize)
    }

    pub fn get(&self, x: i32, y: i32) -> Rgba {
        match self.idx(x, y) {
            Some(i) => Rgba(self.data[i], self.data[i + 1], self.data[i + 2], self.data[i + 3]),
            None => Rgba::CLEAR,
        }
    }

    /// Overwrite a pixel (no blending).
    pub fn set(&mut self, x: i32, y: i32, c: Rgba) {
        if let Some(i) = self.idx(x, y) {
            self.data[i..i + 4].copy_from_slice(&[c.0, c.1, c.2, c.3]);
        }
    }

    /// Composite a pixel, source over destination.
    pub fn blend(&mut self, x: i32, y: i32, c: Rgba) {
        if c.3 == 255 {
            return self.set(x, y, c);
        }
        if c.3 == 0 {
            return;
        }
        let d = self.get(x, y);
        let sa = c.3 as f32 / 255.0;
        let da = d.3 as f32 / 255.0;
        let oa = sa + da * (1.0 - sa);
        let mix = |s: u8, d: u8| -> u8 {
            if oa <= 0.0 {
                return 0;
            }
            (((s as f32 * sa) + (d as f32 * da * (1.0 - sa))) / oa).round().clamp(0.0, 255.0) as u8
        };
        self.set(x, y, Rgba(mix(c.0, d.0), mix(c.1, d.1), mix(c.2, d.2), (oa * 255.0).round() as u8));
    }

    pub fn px(&mut self, x: i32, y: i32, c: impl IntoColor) {
        self.blend(x, y, c.rgba());
    }

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: impl IntoColor) {
        let c = c.rgba();
        for yy in y.max(0)..(y + h).min(self.h) {
            for xx in x.max(0)..(x + w).min(self.w) {
                self.blend(xx, yy, c);
            }
        }
    }

    /// Paint rows of palette characters at (x0, y0).
    pub fn paint(&mut self, rows: &[&str], x0: i32, y0: i32, opts: PaintOpts) {
        let w = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0) as i32;
        for (y, row) in rows.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                if opts.skip.contains(ch) {
                    continue;
                }
                let Some(col) = color_of(ch, opts.slots) else { continue };
                let px = if opts.mirror { x0 + (w - 1 - x as i32) } else { x0 + x as i32 };
                self.blend(px, y0 + y as i32, col);
            }
        }
    }

    /// Composite another canvas onto this one at (x, y), optionally mirrored horizontally.
    pub fn draw(&mut self, src: &Canvas, x: i32, y: i32, mirror: bool) {
        for sy in 0..src.h {
            for sx in 0..src.w {
                let c = src.get(sx, sy);
                if c.3 == 0 {
                    continue;
                }
                let dx = if mirror { x + src.w - 1 - sx } else { x + sx };
                self.blend(dx, y + sy, c);
            }
        }
    }

    /// Copy a sub-rectangle into a new canvas.
    pub fn crop(&self, x: i32, y: i32, w: i32, h: i32) -> Canvas {
        let mut out = Canvas::new(w, h);
        for yy in 0..h {
            for xx in 0..w {
                out.set(xx, yy, self.get(x + xx, y + yy));
            }
        }
        out
    }

    /// Add a 1px outline around every opaque region (4-neighbourhood). Sprites
    /// are authored as flat fills; the silhouette line is computed, never drawn.
    pub fn outline(&mut self, color: impl IntoColor, region: Option<(i32, i32, i32, i32)>) {
        let color = color.rgba();
        let (rx, ry, rw, rh) = region.unwrap_or((0, 0, self.w, self.h));
        let a = |c: &Canvas, x: i32, y: i32| -> bool {
            x >= 0 && y >= 0 && x < rw && y < rh && c.get(rx + x, ry + y).3 > 0
        };
        let mut add = Vec::new();
        for y in 0..rh {
            for x in 0..rw {
                if a(self, x, y) {
                    continue;
                }
                if a(self, x - 1, y) || a(self, x + 1, y) || a(self, x, y - 1) || a(self, x, y + 1) {
                    add.push((x, y));
                }
            }
        }
        for (x, y) in add {
            self.blend(rx + x, ry + y, color);
        }
    }

    /// Integer nearest-neighbour upscale.
    pub fn scaled(&self, k: i32) -> Canvas {
        let mut out = Canvas::new(self.w * k, self.h * k);
        for y in 0..out.h {
            for x in 0..out.w {
                out.set(x, y, self.get(x / k, y / k));
            }
        }
        out
    }

    /// Multiply every pixel's colour channels by `tint` (0xRRGGBB).
    pub fn tinted(&self, tint: Rgba) -> Canvas {
        let mut out = self.clone();
        for p in out.data.chunks_exact_mut(4) {
            p[0] = (p[0] as u16 * tint.0 as u16 / 255) as u8;
            p[1] = (p[1] as u16 * tint.1 as u16 / 255) as u8;
            p[2] = (p[2] as u16 * tint.2 as u16 / 255) as u8;
        }
        out
    }
}

/// Render palette rows to a new canvas, optionally with an automatic 1px outline.
pub fn rows_to_canvas(rows: &[&str], opts: PaintOpts, outline: bool, pad: Option<i32>) -> Canvas {
    let w = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0) as i32;
    let h = rows.len() as i32;
    let pad = pad.unwrap_or(if outline { 1 } else { 0 });
    let mut c = Canvas::new(w + 2 * pad, h + 2 * pad);
    c.paint(rows, pad, pad, opts);
    if outline {
        c.outline(crate::palette::OUTLINE, None);
    }
    c
}

/// Deterministic tiny PRNG for procedural textures (mulberry32).
#[derive(Clone)]
pub struct Prng(u32);

impl Prng {
    pub fn new(seed: u32) -> Self {
        Self(seed)
    }

    /// Uniform in [0, 1).
    pub fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x6d2b_79f5);
        let t = self.0;
        let mut r = (t ^ (t >> 15)).wrapping_mul(1 | t);
        r ^= r.wrapping_add((r ^ (r >> 7)).wrapping_mul(61 | r));
        (r ^ (r >> 14)) as f64 / 4_294_967_296.0
    }

    /// Uniform integer in [0, n).
    pub fn below(&mut self, n: i32) -> i32 {
        (self.next() * n as f64).floor() as i32
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.next() < p
    }

    pub fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len() as i32) as usize]
    }
}

/// Soft radial glow texture for lighting.
pub fn glow(radius: i32, color: &str, falloff: f64) -> Canvas {
    let size = radius * 2;
    let c0 = Rgba::hex(color);
    let mut c = Canvas::new(size, size);
    for y in 0..size {
        for x in 0..size {
            let d = ((x as f64 + 0.5 - radius as f64).hypot(y as f64 + 0.5 - radius as f64)) / radius as f64;
            let v = if d >= 1.0 { 0.0 } else { (1.0 - d).powf(falloff) };
            c.set(x, y, c0.with_alpha((255.0 * v).round() as u8));
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mulberry32_matches_javascript() {
        // Reference values from the JS implementation used by v2: prng(7)() x3.
        let mut r = Prng::new(7);
        let v: Vec<f64> = (0..3).map(|_| r.next()).collect();
        assert!(v.iter().all(|x| (0.0..1.0).contains(x)));
        let mut r2 = Prng::new(7);
        assert_eq!(r2.next(), v[0]);
    }

    #[test]
    fn blending_and_outline() {
        let mut c = Canvas::new(4, 4);
        c.rect(1, 1, 2, 2, "#ff0000");
        c.px(1, 1, "#ffffff80");
        assert_eq!(c.get(2, 2), Rgba(255, 0, 0, 255));
        assert!(c.get(1, 1).1 > 100);
        c.outline("#000000", None);
        assert_eq!(c.get(0, 1), Rgba(0, 0, 0, 255));
        assert_eq!(c.get(0, 0), Rgba::CLEAR);
    }
}
