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
    /// Non-fatal problems found while building (ggplot2's warnings), e.g.
    /// "geom_point: removed 2 rows containing non-finite values". See
    /// [`BuiltPlot::warnings`].
    pub warnings: Vec<String>,
    /// Every discrete legend's B3-2 layout plan (explicit `nrow`/`ncol`
    /// grid, or auto-wrap), keyed by aesthetic, computed exactly once in
    /// `layout_built` and reused as-is by `draw_legend_planned` — never
    /// recomputed at draw time, so the warning (based on these same
    /// plans) always matches what's drawn (review #158 finding 2:
    /// computing it twice, once against the pre-reservation panel height
    /// and once against the post-reservation one, could warn "+7 more"
    /// while drawing "+15 more"). A guide missing from this map draws via
    /// the 0.21 single-row/single-column path unchanged. Several
    /// simultaneous discrete guides each get their own independent entry
    /// (sipemu/ggplot-rs#155) — previously this only ever held at most
    /// one plan, for a single discrete guide.
    pub(crate) legend_plans: std::collections::HashMap<Aesthetic, crate::guide::legend::LegendPlan>,
}

impl BuiltPlot {
    /// Non-fatal build warnings, in layer order: rows dropped for non-finite
    /// positions, and layers skipped because their stat produced no data
    /// (e.g. a density of a single value). Rendering still succeeds.
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }
}

/// Columns holding *position* values: rows with non-finite values here are
/// dropped before stats (and again after, for stat output).
const POSITION_COLS: &[&str] = &[
    "x",
    "y",
    "xmin",
    "xmax",
    "ymin",
    "ymax",
    "xend",
    "yend",
    "open",
    "high",
    "low",
    "close",
    "xintercept",
    "yintercept",
    "slope",
    "intercept",
    // Boxplot stat="identity" hinges (B3-5). `outliers` is a comma-list
    // string, not a position value, so it's not listed here — a non-finite
    // outlier is skipped individually with its own warning when parsed.
    "lower",
    "middle",
    "upper",
];

/// Columns a position scale's transformation applies to (pre-stat).
const X_FAMILY: &[&str] = &["x", "xmin", "xmax", "xend", "xintercept"];
const Y_FAMILY: &[&str] = &[
    "y",
    "ymin",
    "ymax",
    "yend",
    "open",
    "high",
    "low",
    "close",
    "yintercept",
    "lower",
    "middle",
    "upper",
];

/// Reference-line intercept columns and the position scale they train
/// (ggplot2: `xintercept`/`yintercept` are x/y aesthetics).
const INTERCEPT_COLS: [(&str, Aesthetic); 2] =
    [("xintercept", Aesthetic::X), ("yintercept", Aesthetic::Y)];

/// Train `scale` on reference-line intercepts. A numeric intercept on a
/// discrete axis is a position *between* categories (ggplot2), not a level.
fn train_intercepts(scale: &mut Box<dyn crate::scale::Scale>, values: &[crate::data::Value]) {
    if scale.is_discrete() && values.iter().all(|v| v.as_f64().is_some() || v.is_na()) {
        return;
    }
    scale.train(values);
}

/// Map a canonical column name to the aesthetic whose scale formats it
/// (B2-5 tooltip templates / B2-4 `data-fvalue`-style formatting). Columns
/// outside this list (group, label, or a stat output like `count`) have no
/// scale of their own: [`render_tooltip_template`] falls back to the plain
/// display format for them.
fn column_aesthetic(name: &str) -> Option<Aesthetic> {
    match name {
        "x" => Some(Aesthetic::X),
        "y" => Some(Aesthetic::Y),
        "fill" => Some(Aesthetic::Fill),
        "color" | "colour" => Some(Aesthetic::Color),
        "size" => Some(Aesthetic::Size),
        "linewidth" => Some(Aesthetic::Linewidth),
        "alpha" => Some(Aesthetic::Alpha),
        // Position columns (B3-5, D9 allowed #7): formatted through the
        // x/y scale, same as the bare `x`/`y` they're derived from.
        "xmin" | "xmax" | "xend" => Some(Aesthetic::X),
        "ymin" | "ymax" | "yend" | "lower" | "middle" | "upper" => Some(Aesthetic::Y),
        _ => None,
    }
}

/// Render one row of a `GGPlot::tooltip` template: `{column}` is filled
/// from `data`'s row `row`, formatted through that column's scale when
/// [`column_aesthetic`] names one and it has a
/// [`Scale::format_value`](crate::scale::Scale::format_value) (else the
/// plain display format); a column that doesn't exist on this layer is
/// empty, not literally `{column}`. `{{`/`}}` are literal braces.
fn render_tooltip_template(
    template: &str,
    data: &DataFrame,
    scales: &ScaleSet,
    row: usize,
) -> String {
    let mut out = String::new();
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                out.push('{');
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                out.push('}');
            }
            '{' => {
                let mut name = String::new();
                let mut closed = false;
                for nc in chars.by_ref() {
                    if nc == '}' {
                        closed = true;
                        break;
                    }
                    name.push(nc);
                }
                if closed {
                    out.push_str(&render_placeholder(&name, data, scales, row));
                } else {
                    // Unterminated `{…` to end of string: not a placeholder,
                    // kept literally.
                    out.push('{');
                    out.push_str(&name);
                }
            }
            c => out.push(c),
        }
    }
    out
}

fn render_placeholder(name: &str, data: &DataFrame, scales: &ScaleSet, row: usize) -> String {
    let Some(col) = data.column(name) else {
        return String::new();
    };
    let Some(v) = col.get(row) else {
        return String::new();
    };
    if v.is_na() {
        return String::new();
    }
    if let Some(formatted) = column_aesthetic(name)
        .and_then(|aes| scales.get(&aes))
        .and_then(|s| s.format_value(v))
    {
        return formatted;
    }
    crate::geom::tip_value(v)
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
            warnings: plot_warnings,
            default_aspect_ratio,
            locale,
            svg_root_data: _,
            root_attrs: _,
            svg_warnings_attr: _,
            svg_formatted_values: _,
        } = plot;
        let mut theme = theme;
        if theme.aspect_ratio.is_none() {
            theme.aspect_ratio = default_aspect_ratio;
        }

        // Malformed input (e.g. mismatched column lengths) is reported here as
        // a validation error instead of panicking when the frame is assembled.
        plot_data.validate()?;
        for layer in &layers {
            if let Some(d) = &layer.data {
                d.validate()?;
            }
        }

        let mut scale_set = ScaleSet::new();

        // Add user-specified scales
        for s in user_scales {
            scale_set.add(s);
        }

        let mut built_layers = Vec::new();
        let mut warnings: Vec<String> = plot_warnings;

        // Faceting variables — used to group stat computation per panel so a
        // computed stat (density/histogram) is estimated per panel, not pooled.
        let facet_vars = Self::facet_vars(&facet);

        for layer in layers {
            let built = Self::build_layer(
                layer,
                &plot_data,
                &plot_mapping,
                &mut scale_set,
                &theme,
                &facet_vars,
                &mut warnings,
            )?;
            built_layers.push(built);
        }

        // Final scale training pass across all layers
        for bl in &built_layers {
            scale_set.train_layer(&bl.data);
            for (col, aes) in &INTERCEPT_COLS {
                // No layer created this position scale: the intercepts do.
                if let (Some(values), None) = (bl.data.column(col), scale_set.get(aes)) {
                    let mut frame = DataFrame::new();
                    frame.add_column(aes.col_name().to_string(), values.to_vec());
                    scale_set.ensure_scale(aes, &frame);
                }
                if let (Some(values), Some(scale)) = (bl.data.column(col), scale_set.get_mut(aes)) {
                    train_intercepts(scale, values);
                }
            }
        }

        // Validate each scale's final configuration (e.g. a transform
        // undefined at its own domain limits) now that training is done —
        // `map`/`map_to_size` run per mark at draw time and have no
        // warnings sink of their own.
        for scale in scale_set.iter() {
            scale.validate(&mut warnings);
        }

        // An empty plot (no layers, or only layers without data) still gets a
        // panel with axes: make sure both position scales exist. They stay
        // untrained, so they draw no breaks and map everything to the centre.
        if built_layers.iter().all(|bl| bl.data.nrows() == 0) {
            for aes in [Aesthetic::X, Aesthetic::Y] {
                if scale_set.get(&aes).is_none() {
                    scale_set.add(Box::new(
                        crate::scale::continuous::ScaleContinuous::new().for_aesthetic(aes),
                    ));
                }
            }
        }

        // Apply coord zoom limits (coord_cartesian xlim/ylim)
        if let Some((min, max)) = coord.zoom_x() {
            scale_set.set_limits(&Aesthetic::X, min, max);
        }
        if let Some((min, max)) = coord.zoom_y() {
            scale_set.set_limits(&Aesthetic::Y, min, max);
        }

        // Let the coordinate system adjust/inspect the trained scales.
        coord.train_scales(&mut scale_set);

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

        // Plot-level locale for default break labels (before panel scales are
        // cloned from the base set, so free facets inherit it).
        if let Some(locale) = &locale {
            scale_set.set_default_locale(locale);
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
            warnings,
            legend_plans: std::collections::HashMap::new(),
        })
    }

    /// Keep only the rows whose `keep` flag is set.
    fn retain_rows(data: &mut DataFrame, keep: &[bool]) {
        if keep.iter().all(|&k| k) {
            return;
        }
        let mut result = DataFrame::new();
        for col_name in data.column_names() {
            if let Some(src) = data.column(col_name) {
                let vals: Vec<_> = src
                    .iter()
                    .zip(keep)
                    .filter(|(_, &k)| k)
                    .map(|(v, _)| v.clone())
                    .collect();
                result.add_column(col_name.to_string(), vals);
            }
        }
        *data = result;
    }

    /// Drop rows with a non-finite value in any position column (`NaN` always;
    /// `±Inf` unless the geom gives infinities meaning). Returns the number of
    /// rows removed.
    /// `keep_as_break`: for a connecting geom ([`Geom::keeps_na_as_break`]),
    /// a bad row is counted (for the warning) but not deleted — it stays in
    /// `data` as a non-finite pixel position, which `DrawBackend::draw_line`
    /// splits the polyline on, instead of being spliced out and silently
    /// joining the finite points across the gap.
    /// `exclude`: column names to skip entirely this call — `lower`/
    /// `middle`/`upper` present in the raw data but not mapped as that
    /// aesthetic on this layer (review #158's final check: these must
    /// never cost an unrelated geom its rows; see `raw_unmapped` at the
    /// call site).
    fn drop_non_finite(
        data: &mut DataFrame,
        allow_infinite: bool,
        keep_as_break: bool,
        exclude: &[&str],
    ) -> usize {
        let n = data.nrows();
        let mut keep = vec![true; n];
        for col in POSITION_COLS {
            if exclude.contains(col) {
                continue;
            }
            if let Some(values) = data.column(col) {
                // Several position columns are legitimately discrete
                // (`Value::Str`) in this codebase — not just `x`/`y`, e.g.
                // `geom_bracket`'s `xmin`/`xmax` hold category names for a
                // bracket spanning two discrete groups. `Value::Na` is
                // unambiguously missing everywhere, though (review #158
                // finding 4: a SQL NULL in a boxplot position column used
                // to silently map to 0 instead of being dropped with the
                // standard warning; only `Float(NaN)` was caught before).
                // The new boxplot-only `lower`/`middle`/`upper` columns
                // have no legitimate discrete use anywhere, so a
                // non-numeric, non-`Na` value there (a stray string) is
                // missing too.
                let strictly_numeric = matches!(*col, "lower" | "middle" | "upper");
                for (i, v) in values.iter().enumerate() {
                    let bad = match v {
                        crate::data::Value::Na => true,
                        crate::data::Value::Float(f) => {
                            f.is_nan() || (f.is_infinite() && !allow_infinite)
                        }
                        _ if strictly_numeric => v.as_f64().is_none(),
                        _ => false,
                    };
                    if bad {
                        keep[i] = false;
                    }
                }
            }
        }
        let removed = keep.iter().filter(|&&k| !k).count();
        if removed > 0 && !keep_as_break {
            Self::retain_rows(data, &keep);
        }
        removed
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
        theme: &Theme,
        facet_vars: &[String],
        warnings: &mut Vec<String>,
    ) -> Result<BuiltLayer, GGError> {
        let primary = theme.primary;
        let geom_theme = &theme.geom;
        let Layer {
            data: layer_data,
            mapping: layer_mapping,
            mut geom,
            stat,
            position,
            params: _,
            show_legend,
            explicit_style,
            tooltip_template,
        } = layer;

        // Step 1: Resolve data — use layer data if provided, else plot data
        // Borrow the plot data when the layer has none of its own (no per-layer
        // full-frame clone; `resolve_mappings` copies only the columns it needs).
        let source_data = layer_data.as_ref().unwrap_or(plot_data);

        // Step 2: Merge mappings — layer overrides plot-level
        // (Reference lines don't inherit the plot mapping — ggplot2's
        // `inherit.aes = FALSE`.)
        let merged_mapping = if geom.inherit_aes() {
            plot_mapping.merge(&layer_mapping)
        } else {
            layer_mapping.clone()
        };

        // B2-7: geom ink/paper/accent palette, applied before the brand
        // colour below so a primary colour still wins when both are set.
        // Skipped when the layer was configured explicitly
        // (`geom_*_with(...)`) — the same `explicit_style` gate
        // `set_series_color` uses below, and for the same reason: a
        // field-value comparison against the hard-coded default (e.g.
        // `color == (0, 0, 0)`) can't tell an explicitly-set black apart
        // from a black that was simply never touched (review PR#141
        // finding 4), but `explicit_style` can.
        if !explicit_style {
            geom.apply_element_geom(geom_theme);
        }

        // Brand/primary color: apply to a single-series geom only when the layer
        // maps neither color nor fill (an explicit aesthetic always wins) and
        // the geom wasn't configured explicitly via `geom_*_with(...)`.
        if let Some(color) = primary.filter(|_| !explicit_style) {
            let has_color = merged_mapping.get_mapping(&Aesthetic::Color).is_some();
            let has_fill = merged_mapping.get_mapping(&Aesthetic::Fill).is_some();
            if !has_color && !has_fill {
                geom.set_series_color(color);
            }
        }

        // Step 3: Evaluate aes — rename columns to canonical names
        let mut working_data = resolve_mappings(source_data, &merged_mapping, warnings);

        // D3 (B3-5): drop a "mapped-only" column (e.g. `lower`/`middle`/
        // `upper`/`outliers` for the boxplot) unless the user actually
        // mapped that aesthetic — so a raw column that merely happens to
        // share the name (a SQL alias, say) is never mistaken for one, and
        // so a raw `lower` is never drawn as a boxplot hinge on
        // `geom_boxplot` itself.
        //
        // Review #158's *final check* found that making this drop
        // universal (every geom, round 3's `MAPPED_ONLY_COLS`) was itself
        // a regression: it removed the raw `lower`/`middle`/`upper`/
        // `outliers` columns from a layer's data entirely, so a forecast
        // chart's `tooltip("{y} [{lower}, {upper}]")` on an unmapped
        // `geom_line` rendered `[, ]` instead of the actual bounds, and
        // `facet_wrap("upper")` on an unmapped column silently rendered
        // unfaceted. ggplot2 never drops a raw data column just because
        // its name matches an unused extension aesthetic — it only ever
        // special-cases `lower`/`middle`/`upper`/`outliers` for
        // `geom_boxplot`'s own hinges, so this drop goes back to being
        // scoped to `geom.mapped_only_columns()` alone (empty by default,
        // `GeomBoxplot` returns the four names). The narrower position/
        // non-finite/y-scale-training concerns from the *previous*
        // re-review (the forecast y-domain-stretch/row-drop regression)
        // are instead handled per layer below (`raw_unmapped`), without
        // ever removing the columns from `working_data`.
        for col in geom.mapped_only_columns() {
            let is_mapped = merged_mapping
                .mappings
                .iter()
                .any(|m| m.aesthetic.col_name() == *col);
            if !is_mapped {
                working_data.remove_column(col);
            }
        }

        // `lower`/`middle`/`upper` present in the raw (pre-stat) data under
        // their canonical name, but *not* actually mapped as that
        // aesthetic on this layer — e.g. a forecast's confidence-bound
        // columns, never mapped on a plain `geom_line`/`geom_point` layer.
        // These stay in `working_data` (see above), but mustn't be treated
        // as position data a geom never asked for: excluded below from the
        // non-finite/NA position drop, the `Y_FAMILY` pre-stat scale
        // transform, and the y-scale training.
        //
        // Computed once, right here (before the stat runs), so it reflects
        // only what the *raw* data actually had under these names.
        // `GeomBoxplot`'s own default (non-identity) stat produces *fresh*
        // `lower`/`middle`/`upper` columns later (not present yet, so never
        // in this set) — those stay subject to all three, unaffected; an
        // identity-stat boxplot has them mapped (required by
        // `validate_layer` below), so they're not in this set either.
        let raw_unmapped: Vec<&'static str> = ["lower", "middle", "upper"]
            .into_iter()
            .filter(|col| {
                working_data.has_column(col)
                    && !merged_mapping
                        .mappings
                        .iter()
                        .any(|m| m.aesthetic.col_name() == *col)
            })
            .collect();

        // Remember which columns the user actually supplied (pre-stat). A required
        // aesthetic is satisfied if it was present here OR is synthesized by the
        // stat (checked after Step 6) — e.g. boxplot maps `y` then the stat turns
        // it into ymin/ymax, while StatEcdf produces `y` that wasn't mapped.
        let pre_stat_columns: Vec<String> = working_data
            .column_names()
            .iter()
            .map(|s| s.to_string())
            .collect();

        // B3-5: stat-dependent validation beyond `required_aes` (e.g.
        // `geom_boxplot` requires lower/middle/upper under stat="identity",
        // else y) — before Step 4, so a bad mapping fails before any scale
        // work happens.
        if let Err(msg) = geom.validate_layer(&pre_stat_columns, stat.name()) {
            return Err(GGError::ValidationError(msg));
        }

        // Step 4: Ensure scales exist for each mapped aesthetic
        for m in &merged_mapping.mappings {
            scale_set.ensure_scale(&m.aesthetic, &working_data);
        }

        // Step 5: Scale transformation (e.g., log10 before stats). A position
        // scale transforms every column of its family (y also ymin/ymax/…).
        for scale in scale_set.iter() {
            let aes = scale.aesthetic();
            let cols: Vec<&str> = match aes {
                Aesthetic::X => X_FAMILY.to_vec(),
                // `lower`/`middle`/`upper` present but unmapped (raw
                // forecast-shaped passthrough columns, say) are not
                // position data this layer's geom reads, so a log/sqrt/…
                // y-scale transform must not touch them (review #158's
                // final check).
                Aesthetic::Y => Y_FAMILY
                    .iter()
                    .copied()
                    .filter(|c| !raw_unmapped.contains(c))
                    .collect(),
                _ => vec![aes.col_name()],
            };
            for col_name in cols {
                let Some(col) = working_data.column(col_name) else {
                    continue;
                };
                // A transform that is undefined for a value (log of 0 or a
                // negative) yields NaN, so the non-finite filter below drops
                // the row with a warning instead of drawing it at 0.
                let transformed: Vec<_> = col
                    .iter()
                    .map(|v| match (scale.transform(v), v.as_f64()) {
                        (crate::data::Value::Na, Some(_)) => crate::data::Value::Float(f64::NAN),
                        (t, _) => t,
                    })
                    .collect();
                let any_changed = transformed.iter().zip(col.iter()).any(|(t, o)| {
                    match (t.as_f64(), o.as_f64()) {
                        (Some(a), Some(b)) => a.is_nan() || (a - b).abs() > f64::EPSILON,
                        _ => false,
                    }
                });
                if any_changed {
                    if let Some(col_mut) = working_data.column_mut(col_name) {
                        *col_mut = transformed;
                    }
                }
            }
        }

        // Step 5b: Filter out-of-bounds data (xlim/ylim filter before stats)
        Self::filter_oob_data(&mut working_data, scale_set);

        // Step 5c: Drop rows whose position is NaN (or ±Inf, unless the geom
        // reads infinities as "panel edge"), like ggplot2's
        // "Removed n rows containing non-finite values".
        let geom_label = format!("geom_{}", geom.name());
        let removed = Self::drop_non_finite(
            &mut working_data,
            geom.allows_infinite(),
            geom.keeps_na_as_break(),
            &raw_unmapped,
        );
        if removed > 0 {
            warnings.push(format!(
                "{geom_label}: removed {removed} row{} containing non-finite values",
                if removed == 1 { "" } else { "s" }
            ));
        }
        let input_rows = working_data.nrows();

        // Validate the stat's own configuration once per layer (e.g.
        // `StatBinHex::binwidth` — a problem with fixed parameters, not
        // with the data), before it runs per-group below.
        stat.validate(warnings);

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
        apply_after_stat(&mut working_data, &merged_mapping, warnings);

        // Stat output can contain NaN (e.g. a fit on degenerate input): drop
        // those rows too, so nothing non-finite reaches a geom. `raw_unmapped`
        // was computed pre-stat, so a passthrough stat (e.g. `StatIdentity`
        // under `geom_line`/`geom_point`) still excludes the same raw,
        // unmapped `lower`/`middle`/`upper` here as it did above; a stat
        // that *produces* these fresh (`StatBoxplot`'s default, non-identity
        // path) was never in that set, since they didn't exist pre-stat.
        let removed = Self::drop_non_finite(
            &mut working_data,
            geom.allows_infinite(),
            geom.keeps_na_as_break(),
            &raw_unmapped,
        );
        if removed > 0 {
            warnings.push(format!(
                "{geom_label}: removed {removed} row{} containing missing values (stat_{} output)",
                if removed == 1 { "" } else { "s" },
                stat.name()
            ));
        }

        // A layer with no data draws nothing — an empty input, or a stat that
        // cannot estimate anything (a density of one value, loess on two
        // points). It must not fail the whole plot. A stat's own required
        // aesthetics are still validated when there *was* input.
        if working_data.nrows() == 0 {
            if input_rows > 0 {
                for aes in &stat.required_aes() {
                    let col_name = aes.col_name();
                    if !pre_stat_columns.iter().any(|c| c == col_name) {
                        return Err(GGError::ValidationError(format!(
                            "stat_{} requires aesthetic '{}' but it was not provided",
                            stat.name(),
                            col_name
                        )));
                    }
                }
                warnings.push(format!(
                    "{geom_label}: stat_{} produced no data from {input_rows} row{} \
                     (too few or degenerate values); layer skipped",
                    stat.name(),
                    if input_rows == 1 { "" } else { "s" }
                ));
            }
            return Ok(BuiltLayer {
                data: DataFrame::new(),
                geom,
                show_legend,
            });
        }

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
            // Boxplot stat="identity" hinges (B3-5): live on the y scale.
            ("lower", Aesthetic::Y),
            ("middle", Aesthetic::Y),
            ("upper", Aesthetic::Y),
            // Boxplot outlier extent (B3-5, allowed #6): `GeomBoxplot::
            // setup_data` adds these (under an internal-prefix name — see
            // review #158 finding 5 and `aes::BOXPLOT_YMIN_FINAL`'s doc,
            // so a raw column can never forge them) from the parsed
            // `outliers` column, so the y scale trains on them too (step
            // 8b below).
            (crate::aes::BOXPLOT_YMIN_FINAL, Aesthetic::Y),
            (crate::aes::BOXPLOT_YMAX_FINAL, Aesthetic::Y),
        ];
        // `raw_unmapped` (a raw, unmapped `lower`/`middle`/`upper` passthrough
        // column) never trains or gets a scale here — only a layer that
        // actually mapped (or whose stat computed) that name does (review
        // #158's final check).
        for (col, aes) in &stat_aes {
            if working_data.has_column(col) && !raw_unmapped.contains(col) {
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

        // Step 7: Position adjustment. `validate` runs once per layer (D2)
        // regardless of whether the position is per-panel.
        position.validate(warnings);
        let params = PositionParams::default();
        if position.per_panel() {
            Self::compute_position_per_panel(
                position.as_ref(),
                &mut working_data,
                &params,
                facet_vars,
            );
        } else {
            position.compute(&mut working_data, &params);
        }

        // Step 7b: Geom-specific setup (e.g. tile/candlestick extents), then
        // make sure position scales exist for any extent columns it added.
        geom.setup_data(&mut working_data, warnings);
        for (col, aes) in &stat_aes {
            if working_data.has_column(col) && !raw_unmapped.contains(col) {
                scale_set.ensure_scale(aes, &working_data);
            }
        }

        // Step 8: Train scales on this layer's data
        scale_set.train_layer(&working_data);
        for (col, aes) in &INTERCEPT_COLS {
            if let (Some(values), Some(scale)) = (working_data.column(col), scale_set.get_mut(aes))
            {
                train_intercepts(scale, values);
            }
        }

        // Step 8b: Positional scales also need to see stat-computed extent columns
        // (e.g. boxplot/errorbar/pointrange emit ymin/ymax but no "y"). Without
        // this the Y (or X) scale would never train on the range and collapse.
        // Same `raw_unmapped` exclusion as Step 6b/7b above: an unmapped
        // raw `lower`/`middle`/`upper` must never stretch the y domain.
        for (col, aes) in &stat_aes {
            if raw_unmapped.contains(col) {
                continue;
            }
            if let Some(values) = working_data.column(col) {
                if let Some(scale) = scale_set.get_mut(aes) {
                    scale.train(values);
                }
            }
        }

        // Step 9 (B2-5): render a `tooltip()` template against the built
        // (post-stat) data, into the same internal column `Aes::tooltip`
        // uses — so it replaces the geom's default `<title>` the same way,
        // with no further geom-level plumbing.
        if let Some(template) = &tooltip_template {
            // Review PR#141 finding 6 (LOW): a template on a geom that
            // never reads that internal column (`set_row_extras`) did
            // nothing, silently — e.g. `geom_area`/`geom_boxplot`/
            // `geom_ribbon`.
            if !geom.consumes_tooltips() {
                warnings.push(format!(
                    "{geom_label}: tooltip(\"{template}\") has no effect — this geom doesn't read tooltips, or the layer is decorative()"
                ));
            }
            let rendered: Vec<crate::data::Value> = (0..working_data.nrows())
                .map(|i| {
                    crate::data::Value::Str(render_tooltip_template(
                        template,
                        &working_data,
                        scale_set,
                        i,
                    ))
                })
                .collect();
            working_data.add_column(crate::aes::TOOLTIP_INTERNAL_NAME.to_string(), rendered);
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

                // Retrain on this panel's data, including extent columns
                // (bars/tiles/ranges), so free scales cover whole marks.
                for layer_data in panel_layers {
                    panel_set.train_layer(layer_data);
                    for (cols, aes) in [
                        (["xmin", "xmax", "xintercept"], Aesthetic::X),
                        (["ymin", "ymax", "yintercept"], Aesthetic::Y),
                    ] {
                        let freed = match aes {
                            Aesthetic::X => free_x,
                            _ => free_y,
                        };
                        if !freed {
                            continue;
                        }
                        for c in cols {
                            if let (Some(vals), Some(s)) =
                                (layer_data.column(c), panel_set.get_mut(&aes))
                            {
                                if c.ends_with("intercept") {
                                    train_intercepts(s, vals);
                                } else {
                                    s.train(vals);
                                }
                            }
                        }
                    }
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
            Aesthetic::Linewidth,
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

    /// Run a per-panel [`Position`](crate::position::Position) (B3-3, D8):
    /// partition the rows by the facet variables' joined key, in
    /// first-seen order, run `compute` on each panel separately, then
    /// scatter the rows back into their original positions — NA-filling
    /// any column a panel's `compute` added that another panel's didn't
    /// (there currently are none in practice: `PositionStack`/
    /// `PositionFill` add the same `ymin` everywhere). Falls back to the
    /// plain pooled `compute` when there's no facet column, or only one
    /// distinct panel — both byte-identical to the pooled path, since
    /// there's nothing to partition.
    fn compute_position_per_panel(
        position: &dyn crate::position::Position,
        data: &mut DataFrame,
        params: &PositionParams,
        facet_vars: &[String],
    ) {
        let active_vars: Vec<&str> = facet_vars
            .iter()
            .map(|s| s.as_str())
            .filter(|v| data.has_column(v))
            .collect();
        if active_vars.is_empty() {
            position.compute(data, params);
            return;
        }

        let n = data.nrows();
        let mut order: Vec<String> = Vec::new();
        let mut groups: indexmap::IndexMap<String, Vec<usize>> = indexmap::IndexMap::new();
        for i in 0..n {
            let key = active_vars
                .iter()
                .map(|v| data.column(v).unwrap()[i].key_str().into_owned())
                .collect::<Vec<_>>()
                .join("\u{1}");
            if !groups.contains_key(&key) {
                order.push(key.clone());
            }
            groups.entry(key).or_default().push(i);
        }
        if order.len() < 2 {
            position.compute(data, params);
            return;
        }

        // Review #167 finding 2: a per-panel position's group *rank*
        // (e.g. `PositionDodge`) must come from the whole layer, computed
        // here against the still-pooled `data` — before it's split into
        // the panels below — so every panel ranks a given group the same
        // way regardless of which group its own rows happen to list
        // first (sipemu/ggplot-rs#153; matches 0.22.2 and ggplot2 4.0.3's
        // global `group` id). Positions that don't override `group_order`
        // (stack/fill/jitter) get `None` back, unchanged from before.
        let mut panel_params = params.clone();
        panel_params.group_order = position.group_order(data);

        let col_names: Vec<String> = data.column_names().iter().map(|s| s.to_string()).collect();
        let mut panels: Vec<(Vec<usize>, DataFrame)> = Vec::with_capacity(order.len());
        for key in &order {
            let indices = &groups[key];
            let mut sub = DataFrame::new();
            for name in &col_names {
                let col = data.column(name).unwrap();
                let values: Vec<crate::data::Value> =
                    indices.iter().map(|&i| col[i].clone()).collect();
                sub.add_column(name.clone(), values);
            }
            position.compute(&mut sub, &panel_params);
            panels.push((indices.clone(), sub));
        }

        // Union of every column present after `compute` (new columns like
        // `ymin` may have been added), so a panel that for some reason
        // didn't add one still gets NA there rather than a stale value.
        let mut all_cols = col_names.clone();
        for (_, sub) in &panels {
            for name in sub.column_names() {
                if !all_cols.iter().any(|c| c == name) {
                    all_cols.push(name.to_string());
                }
            }
        }
        for name in &all_cols {
            let mut full = vec![crate::data::Value::Na; n];
            for (indices, sub) in &panels {
                if let Some(col) = sub.column(name) {
                    for (j, &i) in indices.iter().enumerate() {
                        full[i] = col[j].clone();
                    }
                }
            }
            data.add_column(name.clone(), full);
        }
    }

    /// Detect which columns to group by for statistics.
    /// Checks group/color/fill plus discrete x (like R's auto-grouping by discrete x).
    fn detect_group_columns(data: &DataFrame) -> Vec<String> {
        detect_group_columns(data)
    }
}

/// The columns whose discrete values split a layer's rows into aesthetic
/// groups (`group`, `color`, `fill`, a discrete `x`). Also used by panelwise
/// stats that regroup inside a panel ([`crate::stat::align::StatAlign`]).
pub(crate) fn detect_group_columns(data: &DataFrame) -> Vec<String> {
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
