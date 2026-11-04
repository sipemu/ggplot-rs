//! Rich text labels (#413, ggtext's `element_markdown()`): a small
//! Markdown / HTML subset in titles, axis titles, strip and legend text when
//! [`Theme::rich_text`](crate::theme::Theme::rich_text) is on.
//!
//! Supported: `**bold**`, `*italic*` / `_italic_`, `^super^` / `~sub~`,
//! `<b>`, `<i>`, `<sup>`, `<sub>`, `<span style='color:#rrggbb'>` (or a basic CSS
//! colour name: `red`, `steelblue`, …) and `\*` escapes.
//! Anything else is literal text.

use super::backend::{DrawBackend, FontFace, TextStyle};
use super::RenderError;

/// One run of uniformly styled text.
#[derive(Clone, Debug, PartialEq)]
pub struct RichRun {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    /// `1` superscript, `-1` subscript, `0` normal.
    pub shift: i8,
    pub color: Option<(u8, u8, u8)>,
}

impl RichRun {
    /// The face this run draws in, given the element's own.
    pub fn face(&self, base: FontFace) -> FontFace {
        if self.bold || base == FontFace::Bold {
            FontFace::Bold
        } else if self.italic || base == FontFace::Italic {
            FontFace::Italic
        } else {
            FontFace::Plain
        }
    }
}

#[derive(Clone, Copy, Default)]
struct State {
    bold: bool,
    italic: bool,
    shift: i8,
    color: Option<(u8, u8, u8)>,
}

fn parse_color(css: &str) -> Option<(u8, u8, u8)> {
    let v = css.split(';').find_map(|d| {
        let (k, v) = d.split_once(':')?;
        (k.trim().eq_ignore_ascii_case("color")).then(|| v.trim().to_string())
    })?;
    if let Some(hex) = v.strip_prefix('#') {
        let n = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
        return match hex.len() {
            6 => Some((n(0)?, n(2)?, n(4)?)),
            3 => {
                let d = |i: usize| {
                    u8::from_str_radix(hex.get(i..i + 1)?, 16)
                        .ok()
                        .map(|x| x * 17)
                };
                Some((d(0)?, d(1)?, d(2)?))
            }
            _ => None,
        };
    }
    let named = [
        ("black", (0, 0, 0)),
        ("white", (255, 255, 255)),
        ("red", (255, 0, 0)),
        ("green", (0, 128, 0)),
        ("blue", (0, 0, 255)),
        ("grey", (128, 128, 128)),
        ("gray", (128, 128, 128)),
        ("orange", (255, 165, 0)),
        ("purple", (128, 0, 128)),
        ("darkred", (139, 0, 0)),
        ("darkgreen", (0, 100, 0)),
        ("darkblue", (0, 0, 139)),
        ("navy", (0, 0, 128)),
        ("steelblue", (70, 130, 180)),
        ("firebrick", (178, 34, 34)),
        ("forestgreen", (34, 139, 34)),
        ("gold", (255, 215, 0)),
        ("brown", (165, 42, 42)),
    ];
    let v = v.to_ascii_lowercase();
    named.iter().find(|(n, _)| *n == v).map(|(_, c)| *c)
}

/// Does `text` contain any markup [`parse`] would interpret?
pub fn has_markup(text: &str) -> bool {
    text.contains("**")
        || text.contains('*')
        || text.contains('_')
        || text.contains('^')
        || text.contains('~')
        || text.contains('<')
        || text.contains('\\')
}

/// Split `text` into styled runs.
pub fn parse(text: &str) -> Vec<RichRun> {
    let mut runs: Vec<RichRun> = Vec::new();
    let mut stack: Vec<State> = vec![State::default()];
    let mut buf = String::new();
    let flush = |buf: &mut String, runs: &mut Vec<RichRun>, st: State| {
        if buf.is_empty() {
            return;
        }
        let run = RichRun {
            text: std::mem::take(buf),
            bold: st.bold,
            italic: st.italic,
            shift: st.shift,
            color: st.color,
        };
        match runs.last_mut() {
            Some(last)
                if last.bold == run.bold
                    && last.italic == run.italic
                    && last.shift == run.shift
                    && last.color == run.color =>
            {
                last.text.push_str(&run.text)
            }
            _ => runs.push(run),
        }
    };
    // Open markdown delimiters, so a second `**` closes the first.
    let mut open_md: Vec<&'static str> = Vec::new();
    let mut rest = text;
    while let Some(ch) = rest.chars().next() {
        let cur = *stack.last().unwrap();
        // Escapes.
        if ch == '\\' {
            if let Some(next) = rest[1..].chars().next() {
                buf.push(next);
                rest = &rest[1 + next.len_utf8()..];
            } else {
                buf.push('\\');
                rest = &rest[1..];
            }
            continue;
        }
        // Markdown delimiters.
        let md = ["**", "*", "_", "^", "~"]
            .into_iter()
            .find(|d| rest.starts_with(d));
        if let Some(d) = md {
            let closing = open_md.last() == Some(&d);
            // An opener needs a matching closer later on.
            if closing || rest[d.len()..].contains(d) {
                flush(&mut buf, &mut runs, cur);
                if closing {
                    open_md.pop();
                    stack.pop();
                } else {
                    open_md.push(d);
                    let mut s = cur;
                    match d {
                        "**" => s.bold = true,
                        "*" | "_" => s.italic = true,
                        "^" => s.shift = 1,
                        _ => s.shift = -1,
                    }
                    stack.push(s);
                }
                rest = &rest[d.len()..];
                continue;
            }
        }
        // HTML tags.
        if ch == '<' {
            if let Some(end) = rest.find('>') {
                let tag = &rest[1..end];
                let lower = tag.trim().to_ascii_lowercase();
                let (closing, name) = match lower.strip_prefix('/') {
                    Some(n) => (true, n.trim().to_string()),
                    None => (
                        false,
                        lower.split_whitespace().next().unwrap_or("").to_string(),
                    ),
                };
                if matches!(
                    name.as_str(),
                    "b" | "strong" | "i" | "em" | "sup" | "sub" | "span"
                ) {
                    flush(&mut buf, &mut runs, cur);
                    if closing {
                        if stack.len() > 1 {
                            stack.pop();
                        }
                    } else {
                        let mut s = cur;
                        match name.as_str() {
                            "b" | "strong" => s.bold = true,
                            "i" | "em" => s.italic = true,
                            "sup" => s.shift = 1,
                            "sub" => s.shift = -1,
                            _ => {
                                // style='…' or style="…"
                                if let Some(i) = tag.find("style=") {
                                    let v = &tag[i + 6..];
                                    let q = v.chars().next().unwrap_or('"');
                                    let body = v[q.len_utf8()..].split(q).next().unwrap_or("");
                                    if let Some(c) = parse_color(body) {
                                        s.color = Some(c);
                                    }
                                }
                            }
                        }
                        stack.push(s);
                    }
                    rest = &rest[end + 1..];
                    continue;
                }
            }
        }
        buf.push(ch);
        rest = &rest[ch.len_utf8()..];
    }
    let cur = *stack.last().unwrap();
    flush(&mut buf, &mut runs, cur);
    runs
}

/// The font size of a shifted (super / subscript) run.
pub fn run_size(size: f64, shift: i8) -> f64 {
    if shift == 0 {
        size
    } else {
        size * 0.7
    }
}

/// Draw `text` — as rich runs when `rich` is on and it carries markup,
/// else exactly as [`DrawBackend::draw_text`] (byte-identical).
pub fn draw(
    backend: &mut dyn DrawBackend,
    rich: bool,
    text: &str,
    pos: (f64, f64),
    style: &TextStyle,
) -> Result<(), RenderError> {
    if rich && has_markup(text) {
        let runs = parse(text);
        backend.draw_rich_text(&runs, pos, style)
    } else {
        backend.draw_text(text, pos, style)
    }
}

/// The text with all markup removed (for width estimates and fallbacks).
pub fn plain(text: &str) -> String {
    parse(text).into_iter().map(|r| r.text).collect()
}

/// Measures a run: `(text, size, face) -> width` in px.
pub type RunWidth<'a> = &'a dyn Fn(&str, f64, FontFace) -> f64;

/// Lay `runs` out left to right (along the text's angle): each run's text,
/// start point and style, measuring a run with `width(text, size, face)` —
/// what a backend without native rich text draws with
/// [`DrawBackend::draw_text`].
pub fn layout_runs(
    runs: &[RichRun],
    (x, y): (f64, f64),
    style: &TextStyle,
    width: RunWidth<'_>,
) -> Vec<(String, (f64, f64), TextStyle)> {
    use super::backend::TextAnchor;
    let widths: Vec<f64> = runs
        .iter()
        .map(|r| width(&r.text, run_size(style.size, r.shift), r.face(style.face)))
        .collect();
    let total: f64 = widths.iter().sum();
    let lead = match style.anchor {
        TextAnchor::Start => 0.0,
        TextAnchor::Middle => total / 2.0,
        TextAnchor::End => total,
    };
    let angle = if style.angle.is_finite() {
        style.angle
    } else {
        0.0
    };
    let (sin, cos) = angle.to_radians().sin_cos();
    let mut along = -lead;
    let mut out = Vec::with_capacity(runs.len());
    for (run, w) in runs.iter().zip(&widths) {
        // A superscript sits a third of an em up, a subscript a fifth down.
        let up = match run.shift {
            1 => style.size * 0.33,
            -1 => -style.size * 0.2,
            _ => 0.0,
        };
        let px = x + along * cos + up * sin;
        let py = y + along * sin - up * cos;
        out.push((
            run.text.clone(),
            (px, py),
            TextStyle {
                size: run_size(style.size, run.shift),
                face: run.face(style.face),
                color: run.color.unwrap_or(style.color),
                anchor: TextAnchor::Start,
                ..style.clone()
            },
        ));
        along += w;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    type Summary = (String, bool, bool, i8, Option<(u8, u8, u8)>);

    fn summary(t: &str) -> Vec<Summary> {
        parse(t)
            .into_iter()
            .map(|r| (r.text, r.bold, r.italic, r.shift, r.color))
            .collect()
    }

    #[test]
    fn markdown_and_html() {
        assert_eq!(
            summary("a **b** *c* x^2^ H~2~O"),
            vec![
                ("a ".into(), false, false, 0, None),
                ("b".into(), true, false, 0, None),
                (" ".into(), false, false, 0, None),
                ("c".into(), false, true, 0, None),
                (" x".into(), false, false, 0, None),
                ("2".into(), false, false, 1, None),
                (" H".into(), false, false, 0, None),
                ("2".into(), false, false, -1, None),
                ("O".into(), false, false, 0, None),
            ]
        );
        assert_eq!(
            summary("<span style='color:#ff0000'>red <b>bold</b></span>!"),
            vec![
                ("red ".into(), false, false, 0, Some((255, 0, 0))),
                ("bold".into(), true, false, 0, Some((255, 0, 0))),
                ("!".into(), false, false, 0, None),
            ]
        );
    }

    #[test]
    fn unmatched_delimiters_and_escapes_stay_literal() {
        assert_eq!(plain("3 * 4 = 12"), "3 * 4 = 12");
        assert_eq!(plain("snake_case"), "snake_case");
        assert_eq!(plain(r"\*not italic\*"), "*not italic*");
        assert_eq!(plain("a < b"), "a < b");
        assert_eq!(plain("<u>x</u>"), "<u>x</u>");
    }
}
