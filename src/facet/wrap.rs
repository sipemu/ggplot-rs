use crate::render::Rect;

use super::Panel;

/// Compute panel layout for facet_wrap.
pub fn compute_wrap_panels(
    levels: &[String],
    ncol: Option<usize>,
    total_area: &Rect,
    strip_height: f64,
) -> Vec<Panel> {
    let n = levels.len();
    if n == 0 {
        return vec![];
    }

    let ncol = ncol.unwrap_or_else(|| {
        let sqrt = (n as f64).sqrt().ceil() as usize;
        sqrt.max(1)
    });
    let nrow = n.div_ceil(ncol);

    let panel_width = total_area.width / ncol as f64;
    let panel_height = total_area.height / nrow as f64;

    levels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let row = i / ncol;
            let col = i % ncol;

            Panel {
                row,
                col,
                label: label.clone(),
                row_label: None,
                col_label: Some(label.clone()),
                rect: Rect {
                    x: total_area.x + col as f64 * panel_width,
                    y: total_area.y + row as f64 * panel_height + strip_height,
                    width: panel_width,
                    height: panel_height - strip_height,
                },
            }
        })
        .collect()
}

/// A `facet_wrap` grid of equal, fixed-aspect panels (e.g. maps under
/// `coord_sf`): the column/row count and each panel's size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FacetGrid {
    pub ncol: usize,
    pub nrow: usize,
    pub panel_w: f64,
    pub panel_h: f64,
}

/// Lay `n` panels of aspect `aspect` (height / width) into an
/// `area_w × area_h` facet area, each panel topped by a `strip_h` strip and
/// separated by `gap_x` / `gap_y`. With `ncol` = `None` the column count is
/// the one that gives the largest panels (ties: fewer rows); a given `ncol`
/// is clamped to `1..=n`. Every panel is `panel_w × panel_h`, with
/// `panel_h = panel_w × aspect`.
#[allow(clippy::too_many_arguments)]
pub fn fit_grid(
    n: usize,
    ncol: Option<usize>,
    aspect: f64,
    area_w: f64,
    area_h: f64,
    strip_h: f64,
    gap_x: f64,
    gap_y: f64,
) -> FacetGrid {
    let n = n.max(1);
    let aspect = if aspect.is_finite() && aspect > 0.0 {
        aspect
    } else {
        1.0
    };
    let size = |c: usize| {
        let r = n.div_ceil(c);
        let pw = (area_w - gap_x * (c as f64 - 1.0)) / c as f64;
        let ph = (area_h - r as f64 * strip_h - gap_y * (r as f64 - 1.0)) / r as f64;
        (pw.min(ph / aspect)).max(0.0)
    };
    let c = match ncol {
        Some(c) => c.clamp(1, n),
        None => {
            let mut best = (1, size(1));
            for c in 2..=n {
                let s = size(c);
                // on a tie, the grid with fewer rows wins
                if s > best.1 + 1e-9
                    || ((s - best.1).abs() <= 1e-9 && n.div_ceil(c) < n.div_ceil(best.0))
                {
                    best = (c, s);
                }
            }
            best.0
        }
    };
    let s = size(c);
    FacetGrid {
        ncol: c,
        nrow: n.div_ceil(c),
        panel_w: s,
        panel_h: s * aspect,
    }
}

/// The natural height of a `facet_wrap` area of fixed-aspect panels at
/// `width`: the column count (a given `ncol`, else as many columns as keep
/// each panel at least `min_panel_w` wide, at most `n`) and
/// `nrow × (panel_h + strip_h) + (nrow − 1) × gap_y` for panels filling the
/// width. Lets a host size a faceted map before rendering it.
#[allow(clippy::too_many_arguments)]
pub fn natural_height(
    n: usize,
    ncol: Option<usize>,
    aspect: f64,
    width: f64,
    strip_h: f64,
    gap_x: f64,
    gap_y: f64,
    min_panel_w: f64,
) -> (usize, f64) {
    let n = n.max(1);
    let aspect = if aspect.is_finite() && aspect > 0.0 {
        aspect
    } else {
        1.0
    };
    let c = match ncol {
        Some(c) => c.clamp(1, n),
        None => (((width + gap_x) / (min_panel_w + gap_x)).floor() as usize).clamp(1, n),
    };
    let r = n.div_ceil(c);
    let pw = ((width - gap_x * (c as f64 - 1.0)) / c as f64).max(0.0);
    (
        c,
        r as f64 * (pw * aspect + strip_h) + gap_y * (r as f64 - 1.0),
    )
}

#[cfg(test)]
mod grid_tests {
    use super::*;

    #[test]
    fn fit_grid_picks_the_largest_panels() {
        // wide area, landscape-ish maps (aspect 1.3 = taller than wide): one row
        let g = fit_grid(4, None, 1.3, 1200.0, 400.0, 0.0, 0.0, 0.0);
        assert_eq!((g.ncol, g.nrow), (4, 1));
        // square area: 2×2
        let g = fit_grid(4, None, 1.0, 600.0, 600.0, 0.0, 0.0, 0.0);
        assert_eq!((g.ncol, g.nrow), (2, 2));
        assert!((g.panel_w - 300.0).abs() < 1e-9 && (g.panel_h - 300.0).abs() < 1e-9);
    }

    #[test]
    fn fit_grid_respects_strips_gaps_and_a_given_ncol() {
        let g = fit_grid(3, Some(3), 0.5, 620.0, 400.0, 20.0, 10.0, 10.0);
        assert_eq!((g.ncol, g.nrow), (3, 1));
        assert!((g.panel_w - 200.0).abs() < 1e-9);
        assert!((g.panel_h - 100.0).abs() < 1e-9);
        // ncol is clamped to n
        assert_eq!(
            fit_grid(2, Some(9), 1.0, 100.0, 100.0, 0.0, 0.0, 0.0).ncol,
            2
        );
        // height-bound: panels shrink to fit the rows
        let g = fit_grid(2, Some(1), 1.0, 1000.0, 220.0, 10.0, 0.0, 0.0);
        assert!((g.panel_h - 100.0).abs() < 1e-9 && (g.panel_w - 100.0).abs() < 1e-9);
    }

    #[test]
    fn natural_height_fills_the_width() {
        // 4 panels, min 160 px, 700 px wide, gaps 10: 4 columns fit (4×160+30=670)
        let (c, h) = natural_height(4, None, 1.0, 700.0, 20.0, 10.0, 10.0, 160.0);
        assert_eq!(c, 4);
        assert!((h - (167.5 + 20.0)).abs() < 1e-9);
        // 390 px phone: 2 columns, 2 rows
        let (c, h) = natural_height(4, None, 1.0, 390.0, 20.0, 10.0, 10.0, 160.0);
        assert_eq!(c, 2);
        assert!((h - (2.0 * (190.0 + 20.0) + 10.0)).abs() < 1e-9);
        // a fixed ncol wins
        assert_eq!(
            natural_height(4, Some(1), 1.0, 700.0, 0.0, 0.0, 0.0, 160.0).0,
            1
        );
    }
}
