use crate::render::Rect;

use super::Panel;

/// Compute panel layout for facet_grid.
pub fn compute_grid_panels(
    row_levels: &[String],
    col_levels: &[String],
    total_area: &Rect,
    strip_height: f64,
    strip_width: f64,
) -> Vec<Panel> {
    let nrow = row_levels.len().max(1);
    let ncol = col_levels.len().max(1);

    let panel_width = (total_area.width - strip_width) / ncol as f64;
    let panel_height = (total_area.height - strip_height) / nrow as f64;

    let mut panels = Vec::new();

    for (ri, rl) in row_levels.iter().enumerate() {
        for (ci, cl) in col_levels.iter().enumerate() {
            panels.push(Panel {
                row: ri,
                col: ci,
                label: format!("{rl} | {cl}"),
                row_label: Some(rl.clone()),
                col_label: Some(cl.clone()),
                rect: Rect {
                    x: total_area.x + ci as f64 * panel_width,
                    y: total_area.y + strip_height + ri as f64 * panel_height,
                    width: panel_width,
                    height: panel_height,
                },
            });
        }
    }

    panels
}
