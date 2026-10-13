//! Write every sprite sheet to PNG files for eyeballing:
//! `cargo run -p cpl-art --example preview -- <out-dir>`

use std::fs::File;
use std::io::BufWriter;

use cpl_art::canvas::Canvas;
use cpl_art::{characters, objects, tiles};

fn save(c: &Canvas, scale: i32, path: &str) {
    let c = c.scaled(scale);
    let file = File::create(path).unwrap();
    let mut enc = png::Encoder::new(BufWriter::new(file), c.w as u32, c.h as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(&c.data).unwrap();
}

/// Stack canvases vertically on a dark background with a gap.
fn stack(items: &[Canvas], bg: &str) -> Canvas {
    let w = items.iter().map(|c| c.w).max().unwrap_or(1) + 4;
    let h: i32 = items.iter().map(|c| c.h + 4).sum::<i32>() + 4;
    let mut out = Canvas::new(w, h);
    out.rect(0, 0, w, h, bg);
    let mut y = 4;
    for c in items {
        out.draw(c, 2, y, false);
        y += c.h + 4;
    }
    out
}

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "art-preview".into());
    std::fs::create_dir_all(&dir).unwrap();
    save(&tiles::tileset(), 3, &format!("{dir}/tiles.png"));
    let chars: Vec<Canvas> = characters::all_characters().iter().map(characters::character_sheet).chain(std::iter::once(characters::cat_sheet())).collect();
    save(&stack(&chars, "#3a3f55"), 3, &format!("{dir}/characters.png"));
    let all = objects::all_sprites();
    let props: Vec<Canvas> = all.iter().filter(|(k, _)| k.starts_with("obj-")).map(|(_, a)| a.sheet()).collect();
    let (a, b) = props.split_at(props.len() / 2);
    save(&stack(a, "#3a3f55"), 3, &format!("{dir}/objects-1.png"));
    save(&stack(b, "#3a3f55"), 3, &format!("{dir}/objects-2.png"));
    let blds: Vec<Canvas> = all.iter().filter(|(k, _)| !k.starts_with("obj-")).map(|(_, a)| a.sheet()).collect();
    let (a, b) = blds.split_at(blds.len() / 2);
    save(&stack(a, "#3a3f55"), 2, &format!("{dir}/buildings-1.png"));
    save(&stack(b, "#3a3f55"), 2, &format!("{dir}/buildings-2.png"));
    println!("wrote previews to {dir}");
}
