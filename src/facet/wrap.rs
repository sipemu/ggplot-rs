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
