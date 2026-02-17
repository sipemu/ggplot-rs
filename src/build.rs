use crate::aes::{mapping::apply_after_stat, mapping::resolve_mappings, Aes, Aesthetic};
use crate::annotate::Annotation;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::facet::{Facet, Panel};
use crate::geom::Geom;
use crate::plot::{GGPlot, Labels, Layer};
use crate::position::PositionParams;
use crate::scale::ScaleSet;
use crate::theme::Theme;

/// A built layer ready for rendering.
pub struct BuiltLayer {
    pub data: DataFrame,
    pub geom: Box<dyn Geom>,
}

/// A fully built plot ready for rendering.
pub struct BuiltPlot {
    pub layers: Vec<BuiltLayer>,
    pub scales: ScaleSet,
    pub coord: Box<dyn Coord>,
    pub theme: Theme,
    pub labels: Labels,
    pub facet: Facet,
    pub panels: Vec<Panel>,
    /// Per-panel layer data. panels_data[panel_idx][layer_idx] = data for that panel+layer.
    pub panels_data: Vec<Vec<DataFrame>>,
    pub annotations: Vec<Annotation>,
    pub guide_legend: crate::guide::config::GuideLegend,
}

/// The grammar pipeline: transforms a GGPlot specification into render-ready data.
pub struct PlotBuilder;

impl PlotBuilder {
    pub fn build(plot: GGPlot) -> BuiltPlot {
        let GGPlot {
            data: plot_data,
            mapping: plot_mapping,
            layers,
            scales: user_scales,
            coord,
            theme,
            labels,
            facet,
            annotations,
            guide_legend,
        } = plot;

        let mut scale_set = ScaleSet::new();

        // Add user-specified scales
        for s in user_scales {
            scale_set.add(s);
        }

        let mut built_layers = Vec::new();

        for layer in layers {
            let built = Self::build_layer(layer, &plot_data, &plot_mapping, &mut scale_set);
            built_layers.push(built);
        }

        // Final scale training pass across all layers
        for bl in &built_layers {
            scale_set.train_layer(&bl.data);
        }

        // Apply coord zoom limits (coord_cartesian xlim/ylim)
        if let Some((min, max)) = coord.zoom_x() {
            scale_set.set_limits(&Aesthetic::X, min, max);
        }
        if let Some((min, max)) = coord.zoom_y() {
            scale_set.set_limits(&Aesthetic::Y, min, max);
        }

        // Compute facet panels
        let (panels, panels_data) = Self::compute_facets(&facet, &built_layers, &plot_data);

        BuiltPlot {
            layers: built_layers,
            scales: scale_set,
            coord,
            theme,
            labels,
            facet,
            panels,
            panels_data,
            annotations,
            guide_legend,
        }
    }

    fn compute_facets(
        facet: &Facet,
        built_layers: &[BuiltLayer],
        _plot_data: &DataFrame,
    ) -> (Vec<Panel>, Vec<Vec<DataFrame>>) {
        match facet {
            Facet::None => (vec![], vec![]),
            Facet::Wrap { var, ncol, .. } => {
                // Collect unique levels from all layers' data
                let mut levels: Vec<String> = Vec::new();
                for bl in built_layers {
                    if let Some(col) = bl.data.column(var) {
                        for v in col {
                            let key = v.to_group_key();
                            if !levels.contains(&key) {
                                levels.push(key);
                            }
                        }
                    }
                }

                // Panels will be positioned during rendering (depends on layout)
                let panels: Vec<Panel> = levels
                    .iter()
                    .enumerate()
                    .map(|(i, label)| {
                        let ncols =
                            ncol.unwrap_or_else(|| (levels.len() as f64).sqrt().ceil() as usize);
                        Panel {
                            row: i / ncols.max(1),
                            col: i % ncols.max(1),
                            label: label.clone(),
                            row_label: None,
                            col_label: Some(label.clone()),
                            rect: crate::render::Rect {
                                x: 0.0,
                                y: 0.0,
                                width: 0.0,
                                height: 0.0,
                            },
                        }
                    })
                    .collect();

                // Split data per panel per layer
                let panels_data: Vec<Vec<DataFrame>> = levels
                    .iter()
                    .map(|level| {
                        built_layers
                            .iter()
                            .map(|bl| Self::filter_data_by_var(&bl.data, var, level))
                            .collect()
                    })
                    .collect();

                (panels, panels_data)
            }
            Facet::Grid {
                row_var, col_var, ..
            } => {
                let mut row_levels: Vec<String> = Vec::new();
                let mut col_levels: Vec<String> = Vec::new();

                for bl in built_layers {
                    if let Some(rv) = row_var {
                        if let Some(col) = bl.data.column(rv) {
                            for v in col {
                                let key = v.to_group_key();
                                if !row_levels.contains(&key) {
                                    row_levels.push(key);
                                }
                            }
                        }
                    }
                    if let Some(cv) = col_var {
                        if let Some(col) = bl.data.column(cv) {
                            for v in col {
                                let key = v.to_group_key();
                                if !col_levels.contains(&key) {
                                    col_levels.push(key);
                                }
                            }
                        }
                    }
                }

                if row_levels.is_empty() {
                    row_levels.push("".to_string());
                }
                if col_levels.is_empty() {
                    col_levels.push("".to_string());
                }

                let mut panels = Vec::new();
                let mut panels_data = Vec::new();

                for (ri, rl) in row_levels.iter().enumerate() {
                    for (ci, cl) in col_levels.iter().enumerate() {
                        panels.push(Panel {
                            row: ri,
                            col: ci,
                            label: format!("{rl} | {cl}"),
                            row_label: if rl.is_empty() {
                                None
                            } else {
                                Some(rl.clone())
                            },
                            col_label: if cl.is_empty() {
                                None
                            } else {
                                Some(cl.clone())
                            },
                            rect: crate::render::Rect {
                                x: 0.0,
                                y: 0.0,
                                width: 0.0,
                                height: 0.0,
                            },
                        });

                        let layer_data: Vec<DataFrame> = built_layers
                            .iter()
                            .map(|bl| {
                                let mut data = bl.data.clone();
                                if let Some(rv) = row_var {
                                    if !rl.is_empty() {
                                        data = Self::filter_data_by_var(&data, rv, rl);
                                    }
                                }
                                if let Some(cv) = col_var {
                                    if !cl.is_empty() {
                                        data = Self::filter_data_by_var(&data, cv, cl);
                                    }
                                }
                                data
                            })
                            .collect();
                        panels_data.push(layer_data);
                    }
                }

                (panels, panels_data)
            }
        }
    }

    fn filter_data_by_var(data: &DataFrame, var: &str, level: &str) -> DataFrame {
        if let Some(col) = data.column(var) {
            let indices: Vec<usize> = col
                .iter()
                .enumerate()
                .filter(|(_, v)| v.to_group_key() == level)
                .map(|(i, _)| i)
                .collect();

            let mut result = DataFrame::new();
            for col_name in data.column_names() {
                if let Some(src) = data.column(col_name) {
                    let vals: Vec<_> = indices.iter().map(|&i| src[i].clone()).collect();
                    result.add_column(col_name.to_string(), vals);
                }
            }
            result
        } else {
            data.clone()
        }
    }

    fn build_layer(
        layer: Layer,
        plot_data: &DataFrame,
        plot_mapping: &Aes,
        scale_set: &mut ScaleSet,
    ) -> BuiltLayer {
        let Layer {
            data: layer_data,
            mapping: layer_mapping,
            geom,
            stat,
            position,
            params: _,
        } = layer;

        // Step 1: Resolve data — use layer data if provided, else plot data
        let source_data = layer_data.unwrap_or_else(|| plot_data.clone());

        // Step 2: Merge mappings — layer overrides plot-level
        let merged_mapping = plot_mapping.merge(&layer_mapping);

        // Step 3: Evaluate aes — rename columns to canonical names
        let mut working_data = resolve_mappings(&source_data, &merged_mapping);

        // Step 4: Ensure scales exist for each mapped aesthetic
        for m in &merged_mapping.mappings {
            scale_set.ensure_scale(&m.aesthetic, &working_data);
        }

        // Step 5: Scale transformation (e.g., log10 before stats)
        for scale in scale_set.iter() {
            let col_name = scale.aesthetic().col_name().to_string();
            if let Some(col) = working_data.column(&col_name) {
                let transformed: Vec<_> = col.iter().map(|v| scale.transform(v)).collect();
                let any_changed = transformed.iter().zip(col.iter()).any(|(t, o)| {
                    match (t.as_f64(), o.as_f64()) {
                        (Some(a), Some(b)) => (a - b).abs() > f64::EPSILON,
                        _ => false,
                    }
                });
                if any_changed {
                    if let Some(col_mut) = working_data.column_mut(&col_name) {
                        *col_mut = transformed;
                    }
                }
            }
        }

        // Step 6: Compute statistics
        let group_cols = Self::detect_group_columns(&working_data);

        working_data = if !group_cols.is_empty() {
            let groups =
                working_data.group_by(&group_cols.iter().map(|s| s.as_str()).collect::<Vec<_>>());
            let mut result = DataFrame::new();
            for group in groups {
                let computed = stat.compute_group(&group, scale_set);
                result.vstack(&computed);
            }
            result
        } else {
            stat.compute_group(&working_data, scale_set)
        };

        // Step 6a: Apply after_stat() mappings (rename stat-computed columns)
        apply_after_stat(&mut working_data, &merged_mapping);

        // Step 6b: Ensure scales for stat-computed aesthetics (e.g. y from StatCount/StatBin)
        let stat_aes = [
            ("x", Aesthetic::X),
            ("y", Aesthetic::Y),
            ("xmin", Aesthetic::X),
            ("xmax", Aesthetic::X),
            ("ymin", Aesthetic::Y),
            ("ymax", Aesthetic::Y),
        ];
        for (col, aes) in &stat_aes {
            if working_data.has_column(col) {
                scale_set.ensure_scale(aes, &working_data);
            }
        }

        // Step 6c: For stat-computed Y (bars, histograms), ensure Y scale includes zero baseline
        let y_is_user_mapped = merged_mapping.get_mapping(&Aesthetic::Y).is_some();
        if !y_is_user_mapped && working_data.has_column("y") {
            if let Some(y_scale) = scale_set.get_mut(&Aesthetic::Y) {
                y_scale.train(&[crate::data::Value::Float(0.0)]);
            }
        }

        // Step 7: Position adjustment
        let params = PositionParams::default();
        position.compute(&mut working_data, &params);

        // Step 8: Train scales on this layer's data
        scale_set.train_layer(&working_data);

        BuiltLayer {
            data: working_data,
            geom,
        }
    }

    /// Detect which columns to group by for statistics.
    /// Checks group/color/fill plus discrete x (like R's auto-grouping by discrete x).
    fn detect_group_columns(data: &DataFrame) -> Vec<String> {
        let candidates = ["group", "color", "fill", "x"];
        let mut group_cols = Vec::new();
        for &col in &candidates {
            if data.has_column(col) {
                if let Some(values) = data.column(col) {
                    let is_discrete = values
                        .iter()
                        .any(|v| matches!(v, crate::data::Value::Str(_)));
                    if is_discrete {
                        group_cols.push(col.to_string());
                    }
                }
            }
        }
        group_cols
    }
}
