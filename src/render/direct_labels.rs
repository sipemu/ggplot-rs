//! Direct labels (#543, R's `directlabels` / a `ggrepel` label at the last
//! point): instead of a colour legend, each line's colour group is named at
//! its end, in its own colour, in the band right of the panel that a right
//! legend would take. Labels never overlap: they are pushed apart vertically
//! (in order of their line's end), then kept inside the panel's height; a
//! label moved away from its line's end gets a short leader to it.

use crate::aes::Aesthetic;
use crate::build::BuiltPlot;
use crate::render::backend::{DrawBackend, LineStyle, TextAnchor, TextStyle};
use crate::render::Rect;
use crate::render::RenderError;

/// One label: its line's end (px), its text and colour.
struct End {
    x: f64,
    y: f64,
    text: String,
    color: (u8, u8, u8),
}

/// The line layers' group ends, or `None` when nothing can be labelled this
/// way (no line layer with a mapped colour) — the caller then draws the usual
/// legend.
fn ends(built: &BuiltPlot, plot_area: &Rect) -> Option<Vec<End>> {
    let (xs, ys) = (
        built.scales.get(&Aesthetic::X)?,
        built.scales.get(&Aesthetic::Y)?,
    );
    let mut out: Vec<End> = Vec::new();
    for layer in &built.layers {
        if !matches!(layer.geom.name(), "line" | "path" | "step") {
            continue;
        }
        let data = &layer.data;
        let (Some(color), Some(x), Some(y)) =
            (data.column("color"), data.column("x"), data.column("y"))
        else {
            continue;
        };
        // the row with the largest x per colour level
        let mut last: Vec<(String, usize, f64)> = Vec::new();
        for i in 0..data.nrows() {
            let (nx, ny) = (xs.map(&x[i]), ys.map(&y[i]));
            if !nx.is_finite() || !ny.is_finite() {
                continue;
            }
            let key = crate::geom::tip_value(&color[i]);
            match last.iter_mut().find(|(k, ..)| *k == key) {
                Some(e) if nx > e.2 => (e.1, e.2) = (i, nx),
                Some(_) => {}
                None => last.push((key, i, nx)),
            }
        }
        for (text, i, _) in last {
            if out.iter().any(|e| e.text == text) {
                continue;
            }
            let (px, py) = built
                .coord
                .transform((xs.map(&x[i]), ys.map(&y[i])), plot_area);
            let color = built
                .scales
                .map_color(&Aesthetic::Color, &color[i])
                .unwrap_or((60, 60, 60));
            out.push(End {
                x: px,
                y: py,
                text,
                color,
            });
        }
    }
    (!out.is_empty()).then_some(out)
}

/// Spread label centres `ys` (sorted ascending) at least `gap` apart and keep
/// them within `[lo, hi]`, moving each as little as possible.
fn spread(ys: &mut [f64], gap: f64, lo: f64, hi: f64) {
    if ys.is_empty() {
        return;
    }
    let n = ys.len();
    // if they can't all fit, squeeze the gap
    let gap = gap
        .min(if n > 1 {
            (hi - lo) / (n - 1) as f64
        } else {
            gap
        })
        .max(0.0);
    for _ in 0..50 {
        let mut moved = false;
        for i in 1..n {
            let d = ys[i] - ys[i - 1];
            if d < gap - 1e-9 {
                let push = (gap - d) / 2.0;
                ys[i - 1] -= push;
                ys[i] += push;
                moved = true;
            }
        }
        if ys[0] < lo {
            let s = lo - ys[0];
            ys.iter_mut().for_each(|y| *y += s);
            moved = true;
        }
        if ys[n - 1] > hi {
            let s = ys[n - 1] - hi;
            ys.iter_mut().for_each(|y| *y -= s);
            moved = true;
        }
        if !moved {
            break;
        }
    }
}

/// Draw the direct labels; `Ok(false)` when there is nothing to label this
/// way (the caller draws the legend instead).
pub fn draw(
    built: &BuiltPlot,
    plot_area: &Rect,
    backend: &mut dyn DrawBackend,
) -> Result<bool, RenderError> {
    let Some(mut ends) = ends(built, plot_area) else {
        return Ok(false);
    };
    let theme = &built.theme;
    let size = theme.legend_text.size;
    ends.sort_by(|a, b| a.y.total_cmp(&b.y));
    let mut ys: Vec<f64> = ends.iter().map(|e| e.y).collect();
    spread(
        &mut ys,
        size * 1.35,
        plot_area.y + size * 0.6,
        plot_area.y + plot_area.height - size * 0.6,
    );
    let x0 = plot_area.x + plot_area.width;
    let tx = x0 + theme.legend_spacing.max(6.0);
    backend.set_element_class(Some("gg-direct-label"));
    for (e, y) in ends.iter().zip(&ys) {
        // a label away from its line's end gets a leader to it
        if (y - e.y).abs() > size * 0.3 || e.x < x0 - 1.0 {
            backend.draw_line(
                &[(e.x + 2.0, e.y), (tx - 2.0, *y)],
                &LineStyle {
                    color: e.color,
                    width: 0.6,
                    alpha: 0.6,
                    ..Default::default()
                },
            )?;
        }
        backend.set_mark_series(Some(e.text.clone()));
        backend.draw_text(
            &e.text,
            (tx, *y),
            &TextStyle {
                color: e.color,
                size,
                anchor: TextAnchor::Start,
                ..Default::default()
            },
        )?;
        backend.set_mark_series(None);
    }
    backend.set_element_class(None);
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::spread;

    #[test]
    fn labels_are_pushed_apart_and_kept_in_range() {
        let mut ys = vec![10.0, 11.0, 12.0, 50.0];
        spread(&mut ys, 5.0, 0.0, 100.0);
        assert!(ys.windows(2).all(|w| w[1] - w[0] >= 5.0 - 1e-6), "{ys:?}");
        assert!(
            (ys[3] - 50.0).abs() < 1e-9,
            "a label with room stays put: {ys:?}"
        );
        let mut low = vec![0.0, 0.5];
        spread(&mut low, 5.0, 2.0, 100.0);
        assert!(
            low[0] >= 2.0 - 1e-9 && low[1] - low[0] >= 5.0 - 1e-6,
            "{low:?}"
        );
    }
}
