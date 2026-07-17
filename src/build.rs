use crate::aes::{mapping::apply_after_stat, mapping::resolve_mappings, Aes, Aesthetic};
use crate::annotate::Annotation;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::facet::{Facet, FacetScales, Panel};
use crate::geom::Geom;
use crate::plot::{GGError, GGPlot, Labels, Layer};
use crate::position::PositionParams;
use crate::scale::ScaleSet;
use crate::theme::Theme;

/// A built layer ready for rendering.
pub struct BuiltLayer {
    pub data: DataFrame,
    pub geom: Box<dyn Geom>,
    pub show_legend: Option<bool>,
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
    /// Aesthetics suppressed from the legend (all layers with that aes set show_legend=false).
    pub suppressed_aes: std::collections::HashSet<Aesthetic>,
    /// Per-panel scale sets for free facets. Empty when FacetScales::Fixed.
    pub panel_scales: Vec<ScaleSet>,
}

/// The grammar pipeline: transforms a GGPlot specification into render-ready data.
pub struct PlotBuilder;

impl PlotBuilder {
    pub fn build(plot: GGPlot) -> Result<BuiltPlot, GGError> {
        let GGPlot {
            data: plot_data,
            mapping: plot_mapping,
            layers,
            scales: user_scales,
            mut coord,
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

        // Faceting variables — used to group stat computation per panel so a
        // computed stat (density/histogram) is estimated per panel, not pooled.
        let facet_vars = Self::facet_vars(&facet);

        for layer in layers {
            let built = Self::build_layer(
                layer,
                &plot_data,
                &plot_mapping,
                &mut scale_set,
                theme.primary,
                &facet_vars,
            )?;
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

        // Supply trained axis spans to the coordinate system (used by coord_trans).
        // pmin/pmax are the panel positions of the domain endpoints, so the coord
        // can invert the scale's (linearly expanded) mapping exactly.
        let axis_span = |aes: &Aesthetic| {
            scale_set.get(aes).and_then(|s| {
                s.domain().map(|(min, max)| crate::coord::AxisSpan {
                    min,
                    max,
                    pmin: s.map(&crate::data::Value::Float(min)),
                    pmax: s.map(&crate::data::Value::Float(max)),
                })
            })
        };
        let x_span = axis_span(&Aesthetic::X);
        let y_span = axis_span(&Aesthetic::Y);
        coord.set_domains(x_span, y_span);

        // Apply after_scale() color derivations: copy the source aesthetic's
        // column to the target and register a lightness-modified clone of the
        // source scale, so the target aesthetic draws the mapped source color
        // adjusted in lightness (e.g. a darker border derived from the fill).
        for spec in &plot_mapping.after_scale {
            if let Some(src_scale) = scale_set.get(&spec.source) {
                let modified = crate::scale::modified::ScaleColorModified::new(
                    src_scale.clone_box(),
                    spec.target.clone(),
                    spec.lightness,
                );
                let (src_col, tgt_col) = (spec.source.col_name(), spec.target.col_name());
                for bl in &mut built_layers {
                    if !bl.data.has_column(tgt_col) {
                        if let Some(vals) = bl.data.column(src_col) {
                            let vals = vals.to_vec();
                            bl.data.add_column(tgt_col.to_string(), vals);
                        }
                    }
                }
                scale_set.add(Box::new(modified));
            }
        }

        // Compute facet panels
        let (panels, panels_data) = Self::compute_facets(&facet, &built_layers, &plot_data);

        // Compute suppressed aesthetics from show_legend flags.
        let suppressed_aes = Self::compute_suppressed_aes(&built_layers);

        // Compute per-panel scales for free facets
        let facet_scales_mode = match &facet {
            Facet::Wrap { scales, .. } => scales.clone(),
            Facet::Grid { scales, .. } => scales.clone(),
            Facet::None => FacetScales::Fixed,
        };
        let panel_scales = Self::compute_panel_scales(&facet_scales_mode, &panels_data, &scale_set);

        Ok(BuiltPlot {
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
            suppressed_aes,
            panel_scales,
        })
    }

    /// The column name(s) a facet splits on, if any.
    fn facet_vars(facet: &Facet) -> Vec<String> {
        match facet {
            Facet::None => vec![],
            Facet::Wrap { var, .. } => vec![var.clone()],
            Facet::Grid {
                row_var, col_var, ..
            } => row_var.iter().chain(col_var.iter()).cloned().collect(),
        }
    }

    fn compute_facets(
        facet: &Facet,
        built_layers: &[BuiltLayer],
        _plot_data: &DataFrame,
    ) -> (Vec<Panel>, Vec<Vec<DataFrame>>) {
        match facet {
            Facet::None => (vec![], vec![]),
            Facet::Wrap {
                var,
                ncol,
                labeller,
                ..
            } => {
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
                    .map(|(i, value)| {
                        let ncols =
                            ncol.unwrap_or_else(|| (levels.len() as f64).sqrt().ceil() as usize);
                        let formatted = labeller.format(var, value);
                        Panel {
                            row: i / ncols.max(1),
                            col: i % ncols.max(1),
                            label: formatted.clone(),
                            row_label: None,
                            col_label: Some(formatted),
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
                row_var,
                col_var,
                labeller,
                ..
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
                        let row_fmt = if rl.is_empty() {
                            None
                        } else {
                            let rv = row_var.as_deref().unwrap_or("");
                            Some(labeller.format(rv, rl))
                        };
                        let col_fmt = if cl.is_empty() {
                            None
                        } else {
                            let cv = col_var.as_deref().unwrap_or("");
                            Some(labeller.format(cv, cl))
                        };
                        let label = match (&row_fmt, &col_fmt) {
                            (Some(r), Some(c)) => format!("{r} | {c}"),
                            (Some(r), None) => r.clone(),
                            (None, Some(c)) => c.clone(),
                            (None, None) => String::new(),
                        };
                        panels.push(Panel {
                            row: ri,
                            col: ci,
                            label,
                            row_label: row_fmt,
                            col_label: col_fmt,
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
        primary: Option<(u8, u8, u8)>,
        facet_vars: &[String],
    ) -> Result<BuiltLayer, GGError> {
        let Layer {
            data: layer_data,
            mapping: layer_mapping,
            mut geom,
            stat,
            position,
            params: _,
            show_legend,
        } = layer;

        // Step 1: Resolve data — use layer data if provided, else plot data
        let source_data = layer_data.unwrap_or_else(|| plot_data.clone());

        // Step 2: Merge mappings — layer overrides plot-level
        let merged_mapping = plot_mapping.merge(&layer_mapping);

        // Brand/primary color: apply to a single-series geom only when the layer
        // maps neither color nor fill (an explicit aesthetic always wins).
        if let Some(color) = primary {
            let has_color = merged_mapping.get_mapping(&Aesthetic::Color).is_some();
            let has_fill = merged_mapping.get_mapping(&Aesthetic::Fill).is_some();
            if !has_color && !has_fill {
                geom.set_series_color(color);
            }
        }

        // Step 3: Evaluate aes — rename columns to canonical names
        let mut working_data = resolve_mappings(&source_data, &merged_mapping);

        // Remember which columns the user actually supplied (pre-stat). A required
        // aesthetic is satisfied if it was present here OR is synthesized by the
        // stat (checked after Step 6) — e.g. boxplot maps `y` then the stat turns
        // it into ymin/ymax, while StatEcdf produces `y` that wasn't mapped.
        let pre_stat_columns: Vec<String> = working_data
            .column_names()
            .iter()
            .map(|s| s.to_string())
            .collect();

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

        // Step 5b: Filter out-of-bounds data (xlim/ylim filter before stats)
        Self::filter_oob_data(&mut working_data, scale_set);

        // Step 6: Compute statistics. Group by aesthetic groups AND the facet
        // variables, so a computed stat (density/histogram/…) is estimated per
        // panel rather than on pooled data; the facet column is then re-attached
        // to each group's output so faceting can split it back out.
        // Panelwise stats (e.g. stat_compare_means) see the whole panel at once,
        // so we group only by facet variables, not by aesthetic groups.
        let mut group_cols = if stat.panelwise() {
            Vec::new()
        } else {
            Self::detect_group_columns(&working_data)
        };
        for fv in facet_vars {
            if working_data.has_column(fv) && !group_cols.contains(fv) {
                group_cols.push(fv.clone());
            }
        }

        working_data = if !group_cols.is_empty() {
            let groups =
                working_data.group_by(&group_cols.iter().map(|s| s.as_str()).collect::<Vec<_>>());
            let mut result = DataFrame::new();
            for group in groups {
                let mut computed = stat.compute_group(&group, scale_set);
                let n = computed.nrows();
                if n > 0 {
                    for fv in facet_vars {
                        if !computed.has_column(fv) {
                            if let Some(val) = group.column(fv).and_then(|c| c.first()).cloned() {
                                computed.add_column(fv.clone(), vec![val; n]);
                            }
                        }
                    }
                }
                result.vstack(&computed);
            }
            result
        } else {
            stat.compute_group(&working_data, scale_set)
        };

        // Step 6a: Apply after_stat() mappings (rename stat-computed columns)
        apply_after_stat(&mut working_data, &merged_mapping);

        // Step 6a-validate: A required aesthetic must have been supplied by the
        // user (pre-stat) or synthesized by the stat (post-stat). This lets
        // StatEcdf produce `y` for geom_step, while boxplot — which maps `y` then
        // consumes it into ymin/ymax — still validates. Empty input has the
        // column in neither place, so genuinely-missing aesthetics still error.
        for aes in &geom.required_aes() {
            let col_name = aes.col_name();
            let supplied = pre_stat_columns.iter().any(|c| c == col_name);
            if !supplied && !working_data.has_column(col_name) {
                return Err(GGError::ValidationError(format!(
                    "geom_{} requires aesthetic '{}' but it was not provided",
                    geom.name(),
                    col_name
                )));
            }
        }

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

        // Step 6c: bars/histograms/area draw from a 0 baseline, so the Y scale
        // must include 0 — for stat-computed Y (count/bin) and for an explicitly
        // mapped Y alike (e.g. geom_col), matching ggplot2.
        let y_is_user_mapped = merged_mapping.get_mapping(&Aesthetic::Y).is_some();
        if (!y_is_user_mapped || geom.include_zero_baseline()) && working_data.has_column("y") {
            if let Some(y_scale) = scale_set.get_mut(&Aesthetic::Y) {
                y_scale.train(&[crate::data::Value::Float(0.0)]);
            }
        }

        // Step 7: Position adjustment
        let params = PositionParams::default();
        position.compute(&mut working_data, &params);

        // Step 8: Train scales on this layer's data
        scale_set.train_layer(&working_data);

        // Step 8b: Positional scales also need to see stat-computed extent columns
        // (e.g. boxplot/errorbar/pointrange emit ymin/ymax but no "y"). Without
        // this the Y (or X) scale would never train on the range and collapse.
        for (col, aes) in &stat_aes {
            if let Some(values) = working_data.column(col) {
                if let Some(scale) = scale_set.get_mut(aes) {
                    scale.train(values);
                }
            }
        }

        Ok(BuiltLayer {
            data: working_data,
            geom,
            show_legend,
        })
    }

    /// Remove rows where x or y falls outside scale limits set via xlim/ylim.
    fn filter_oob_data(data: &mut DataFrame, scale_set: &ScaleSet) {
        let x_limits = scale_set.get(&Aesthetic::X).and_then(|s| s.filter_limits());
        let y_limits = scale_set.get(&Aesthetic::Y).and_then(|s| s.filter_limits());

        if x_limits.is_none() && y_limits.is_none() {
            return;
        }

        let nrows = data.nrows();
        let mut keep = vec![true; nrows];

        if let Some((min, max)) = x_limits {
            if let Some(col) = data.column("x") {
                for (i, v) in col.iter().enumerate() {
                    if let Some(f) = v.as_f64() {
                        if f < min || f > max {
                            keep[i] = false;
                        }
                    }
                }
            }
        }

        if let Some((min, max)) = y_limits {
            if let Some(col) = data.column("y") {
                for (i, v) in col.iter().enumerate() {
                    if let Some(f) = v.as_f64() {
                        if f < min || f > max {
                            keep[i] = false;
                        }
                    }
                }
            }
        }

        // If nothing was filtered, skip the rebuild
        if keep.iter().all(|&k| k) {
            return;
        }

        let indices: Vec<usize> = keep
            .iter()
            .enumerate()
            .filter(|(_, &k)| k)
            .map(|(i, _)| i)
            .collect();

        let mut result = DataFrame::new();
        for col_name in data.column_names() {
            if let Some(src) = data.column(col_name) {
                let vals: Vec<_> = indices.iter().map(|&i| src[i].clone()).collect();
                result.add_column(col_name.to_string(), vals);
            }
        }
        *data = result;
    }

    /// Compute per-panel scale sets for free facet scales.
    /// For each panel, clones the base scale set, resets freed axes, and retrains on panel data.
    fn compute_panel_scales(
        facet_scales: &FacetScales,
        panels_data: &[Vec<DataFrame>],
        base_scales: &ScaleSet,
    ) -> Vec<ScaleSet> {
        if matches!(facet_scales, FacetScales::Fixed) || panels_data.is_empty() {
            return vec![];
        }

        let free_x = matches!(facet_scales, FacetScales::FreeX | FacetScales::Free);
        let free_y = matches!(facet_scales, FacetScales::FreeY | FacetScales::Free);

        panels_data
            .iter()
            .map(|panel_layers| {
                let mut panel_set = base_scales.clone();

                // Reset freed axis scales
                if free_x {
                    if let Some(s) = panel_set.get_mut(&Aesthetic::X) {
                        s.reset_training();
                    }
                }
                if free_y {
                    if let Some(s) = panel_set.get_mut(&Aesthetic::Y) {
                        s.reset_training();
                    }
                }

                // Retrain on this panel's data
                for layer_data in panel_layers {
                    panel_set.train_layer(layer_data);
                }

                panel_set
            })
            .collect()
    }

    /// Compute which aesthetics should be suppressed from the legend.
    /// An aesthetic is suppressed if every layer that has the corresponding column
    /// sets show_legend=Some(false), and no layer has it as None or Some(true).
    fn compute_suppressed_aes(built_layers: &[BuiltLayer]) -> std::collections::HashSet<Aesthetic> {
        use std::collections::HashSet;
        let legend_aes = [
            Aesthetic::Color,
            Aesthetic::Fill,
            Aesthetic::Shape,
            Aesthetic::Linetype,
            Aesthetic::Size,
            Aesthetic::Alpha,
        ];
        let mut suppressed = HashSet::new();
        for aes in &legend_aes {
            let col_name = aes.col_name();
            let mut any_has = false;
            let mut all_hidden = true;
            for bl in built_layers {
                if bl.data.has_column(col_name) {
                    any_has = true;
                    match bl.show_legend {
                        Some(false) => {} // still hidden
                        _ => {
                            all_hidden = false;
                            break;
                        }
                    }
                }
            }
            if any_has && all_hidden {
                suppressed.insert(aes.clone());
            }
        }
        suppressed
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
