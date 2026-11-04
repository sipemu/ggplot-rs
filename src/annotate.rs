/// Annotation types for adding static marks to a plot.
#[derive(Clone, Debug)]
pub enum Annotation {
    /// Text annotation at a data position.
    Text {
        label: String,
        x: f64,
        y: f64,
        size: f64,
        color: (u8, u8, u8),
    },
    /// Rectangle annotation between two data positions.
    Rect {
        xmin: f64,
        xmax: f64,
        ymin: f64,
        ymax: f64,
        fill: (u8, u8, u8),
        alpha: f64,
    },
    /// Line segment annotation between two data positions.
    Segment {
        x: f64,
        y: f64,
        xend: f64,
        yend: f64,
        color: (u8, u8, u8),
        width: f64,
    },
    /// Log-10 tick marks on panel edges (ggplot2's `annotation_logticks`):
    /// long ticks at each power of ten, mid at 5×, short at 2–4× and 6–9×.
    /// Positions are read in the axis' (log10-)transformed units, so it suits
    /// `scale_x_log10()` / `scale_y_log10()` or data already in log10.
    LogTicks {
        /// Panel sides to draw on: any of `b`, `l`, `t`, `r`.
        sides: String,
        /// Draw outward from the panel instead of inward.
        outside: bool,
        /// Tick lengths in px (short, mid, long).
        lengths: (f64, f64, f64),
        color: (u8, u8, u8),
        width: f64,
    },
    /// An encoded image (PNG or JPEG bytes) stretched over a data rectangle
    /// (ggplot2's `annotation_raster`). `±INFINITY` bounds mean the panel edge.
    Raster {
        image: std::sync::Arc<Vec<u8>>,
        xmin: f64,
        xmax: f64,
        ymin: f64,
        ymax: f64,
        alpha: f64,
    },
    /// A whole plot drawn into a data rectangle (ggplot2's `annotation_custom`
    /// with a ggplot grob — an inset). `±INFINITY` bounds mean the panel edge.
    Inset {
        plot: InsetPlot,
        xmin: f64,
        xmax: f64,
        ymin: f64,
        ymax: f64,
    },
}

/// A plot built for drawing as an [`Annotation::Inset`]: built once when the
/// annotation is created (a `GGPlot` itself can't be cloned), laid out for
/// its rectangle at draw time.
#[derive(Clone)]
pub struct InsetPlot(
    pub(crate)  std::sync::Arc<
        std::sync::Mutex<Result<(crate::build::BuiltPlot, crate::plot::RenderMeta), String>>,
    >,
);

impl std::fmt::Debug for InsetPlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InsetPlot")
    }
}

impl InsetPlot {
    pub fn new(plot: crate::GGPlot) -> Self {
        InsetPlot(std::sync::Arc::new(std::sync::Mutex::new(
            plot.build_for_render().map_err(|e| format!("{e:?}")),
        )))
    }
}

impl Annotation {
    /// Create a text annotation.
    pub fn text(label: &str, x: f64, y: f64) -> Self {
        Annotation::Text {
            label: label.to_string(),
            x,
            y,
            size: 12.0,
            color: (50, 50, 50),
        }
    }

    /// Create a rectangle annotation.
    pub fn rect(xmin: f64, xmax: f64, ymin: f64, ymax: f64) -> Self {
        Annotation::Rect {
            xmin,
            xmax,
            ymin,
            ymax,
            fill: (200, 200, 200),
            alpha: 0.3,
        }
    }

    /// Log-10 tick marks on `sides` (`"bl"` by default in ggplot2).
    pub fn logticks(sides: &str) -> Self {
        Annotation::LogTicks {
            sides: sides.to_string(),
            outside: false,
            // ggplot2: 0.1 / 0.2 / 0.3 cm.
            lengths: (3.8, 7.6, 11.3),
            color: (50, 50, 50),
            width: 0.5,
        }
    }

    /// `outside = TRUE` for [`Annotation::LogTicks`] (no-op otherwise).
    pub fn outside(mut self) -> Self {
        if let Annotation::LogTicks { outside, .. } = &mut self {
            *outside = true;
        }
        self
    }

    /// An encoded PNG / JPEG image over the data rectangle.
    pub fn raster(image: Vec<u8>, xmin: f64, xmax: f64, ymin: f64, ymax: f64) -> Self {
        Annotation::Raster {
            image: std::sync::Arc::new(image),
            xmin,
            xmax,
            ymin,
            ymax,
            alpha: 1.0,
        }
    }

    /// `plot` drawn as an inset into the data rectangle.
    pub fn inset(plot: crate::GGPlot, xmin: f64, xmax: f64, ymin: f64, ymax: f64) -> Self {
        Annotation::Inset {
            plot: InsetPlot::new(plot),
            xmin,
            xmax,
            ymin,
            ymax,
        }
    }

    /// Create a segment annotation.
    pub fn segment(x: f64, y: f64, xend: f64, yend: f64) -> Self {
        Annotation::Segment {
            x,
            y,
            xend,
            yend,
            color: (50, 50, 50),
            width: 1.0,
        }
    }
}
