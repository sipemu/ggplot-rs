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
