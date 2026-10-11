//! Light inline markup for dialogue, codex and puzzle text:
//!   **bold**   *emphasis*   `code`   {term} (highlighted)   \n (line break)
//! Rendered to an egui `LayoutJob`, optionally truncated to the first N
//! visible characters (the typewriter effect).

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, Stroke};

use super::theme::{self, CYAN, GOLD, bold_family, italic_family, mono};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Span {
    Plain,
    Bold,
    Em,
    Code,
    Term,
}

/// Split marked-up text into styled runs.
fn parse(text: &str) -> Vec<(Span, String)> {
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<(Span, String)> = Vec::new();
    let mut buf = String::new();
    let mut i = 0;
    let flush = |buf: &mut String, out: &mut Vec<(Span, String)>| {
        if !buf.is_empty() {
            out.push((Span::Plain, std::mem::take(buf)));
        }
    };
    // find the closing delimiter starting at `from`
    let find = |from: usize, delim: &[char]| -> Option<usize> {
        let mut j = from;
        while j + delim.len() <= chars.len() {
            if chars[j..j + delim.len()] == *delim && j > from {
                return Some(j);
            }
            j += 1;
        }
        None
    };
    while i < chars.len() {
        let c = chars[i];
        if c == '*' && i + 1 < chars.len() && chars[i + 1] == '*' {
            if let Some(end) = find(i + 2, &['*', '*']) {
                flush(&mut buf, &mut out);
                out.push((Span::Bold, chars[i + 2..end].iter().collect()));
                i = end + 2;
                continue;
            }
        } else if c == '*' {
            if let Some(end) = find(i + 1, &['*']) {
                flush(&mut buf, &mut out);
                out.push((Span::Em, chars[i + 1..end].iter().collect()));
                i = end + 1;
                continue;
            }
        } else if c == '`' {
            if let Some(end) = find(i + 1, &['`']) {
                flush(&mut buf, &mut out);
                out.push((Span::Code, chars[i + 1..end].iter().collect()));
                i = end + 1;
                continue;
            }
        } else if c == '{' {
            if let Some(end) = find(i + 1, &['}']) {
                flush(&mut buf, &mut out);
                out.push((Span::Term, chars[i + 1..end].iter().collect()));
                i = end + 1;
                continue;
            }
        }
        buf.push(c);
        i += 1;
    }
    flush(&mut buf, &mut out);
    out
}

/// Number of visible characters (for the typewriter).
pub fn visible_len(text: &str) -> usize {
    parse(text).iter().map(|(_, s)| s.chars().count()).sum()
}

/// The plain text without markup.
pub fn plain(text: &str) -> String {
    parse(text).into_iter().map(|(_, s)| s).collect()
}

#[derive(Clone)]
pub struct Look {
    pub size: f32,
    pub color: Color32,
    pub family: Option<egui::FontFamily>,
}

impl Default for Look {
    fn default() -> Self {
        Self { size: 14.5, color: theme::TEXT, family: None }
    }
}

/// Build a layout job; `limit` shows only the first N visible characters.
pub fn job(text: &str, look: Look, wrap: f32, limit: Option<usize>) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap;
    let base_family = look.family.clone().unwrap_or(egui::FontFamily::Monospace);
    let mut left = limit.unwrap_or(usize::MAX);
    for (span, s) in parse(text) {
        if left == 0 {
            break;
        }
        let take: String = s.chars().take(left).collect();
        left = left.saturating_sub(take.chars().count());
        let mut fmt = TextFormat { font_id: FontId::new(look.size, base_family.clone()), color: look.color, ..Default::default() };
        match span {
            Span::Plain => {}
            Span::Bold => {
                fmt.font_id = FontId::new(look.size, bold_family());
                fmt.color = Color32::WHITE;
            }
            Span::Em => {
                fmt.font_id = FontId::new(look.size, italic_family());
                fmt.color = GOLD;
            }
            Span::Code => {
                fmt.font_id = mono(look.size * 0.95);
                fmt.color = theme::rgb(0xa4ffe0);
                fmt.background = theme::rgb(0x0a1a22);
            }
            Span::Term => {
                fmt.color = CYAN;
                fmt.underline = Stroke::new(1.0, theme::CYAN_DIM);
            }
        }
        job.append(&take, 0.0, fmt);
    }
    job
}

/// Show marked-up text that wraps to the available width.
pub fn rich(ui: &mut egui::Ui, text: &str, look: Look) -> egui::Response {
    let j = job(text, look, ui.available_width(), None);
    ui.label(j)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_markup() {
        let p = parse("a **b** *c* `d` {e} f*g");
        assert_eq!(p[0], (Span::Plain, "a ".into()));
        assert_eq!(p[1], (Span::Bold, "b".into()));
        assert_eq!(p[3], (Span::Em, "c".into()));
        assert_eq!(p[5], (Span::Code, "d".into()));
        assert_eq!(p[7], (Span::Term, "e".into()));
        assert_eq!(p.last().unwrap(), &(Span::Plain, " f*g".into()));
        assert_eq!(plain("**x** y"), "x y");
        assert_eq!(visible_len("**ab** c"), 4);
    }
}
