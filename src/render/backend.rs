use super::{Rect, RenderError};

/// Point shape types.
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum PointShape {
    Circle,
    Triangle,
    Square,
    Diamond,
    Cross,
    Plus,
}

impl PointShape {
    /// All shapes in order (for discrete scale mapping).
    pub const ALL: &[PointShape] = &[
        PointShape::Circle,
        PointShape::Triangle,
        PointShape::Square,
        PointShape::Diamond,
        PointShape::Cross,
        PointShape::Plus,
    ];
}

/// Line type patterns.
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum Linetype {
    Solid,
    Dashed,
    Dotted,
    DashDot,
    LongDash,
    TwoDash,
}

impl Linetype {
    /// All linetypes in order (for discrete scale mapping).
    pub const ALL: &[Linetype] = &[
        Linetype::Solid,
        Linetype::Dashed,
        Linetype::Dotted,
        Linetype::DashDot,
        Linetype::LongDash,
        Linetype::TwoDash,
    ];

    /// Get dash pattern as (draw_len, gap_len) pairs in pixels.
    pub fn pattern(&self) -> &[(f64, f64)] {
        match self {
            Linetype::Solid => &[],
            Linetype::Dashed => &[(6.0, 3.0)],
            Linetype::Dotted => &[(2.0, 2.0)],
            Linetype::DashDot => &[(6.0, 2.0), (2.0, 2.0)],
            Linetype::LongDash => &[(10.0, 4.0)],
            Linetype::TwoDash => &[(8.0, 3.0), (3.0, 3.0)],
        }
    }
}

/// Style for drawing points/circles.
#[derive(Clone, Debug)]
pub struct PointStyle {
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub filled: bool,
    pub shape: PointShape,
}

impl Default for PointStyle {
    fn default() -> Self {
        PointStyle {
            color: (0, 0, 0),
            alpha: 1.0,
            filled: true,
            shape: PointShape::Circle,
        }
    }
}

/// Style for drawing lines.
#[derive(Clone, Debug)]
pub struct LineStyle {
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub width: f64,
    pub linetype: Linetype,
}

impl Default for LineStyle {
    fn default() -> Self {
        LineStyle {
            color: (0, 0, 0),
            alpha: 1.0,
            width: 1.0,
            linetype: Linetype::Solid,
        }
    }
}

/// Style for drawing rectangles/polygons.
#[derive(Clone, Debug)]
pub struct RectStyle {
    pub fill: Option<(u8, u8, u8)>,
    pub stroke: Option<(u8, u8, u8)>,
    pub stroke_width: f64,
    pub alpha: f64,
    /// Whether to clip this rect to the plot area. Default `true` for data elements.
    /// Set to `false` for non-data elements (backgrounds, strips, legends).
    pub clip: bool,
}

impl Default for RectStyle {
    fn default() -> Self {
        RectStyle {
            fill: Some((128, 128, 128)),
            stroke: None,
            stroke_width: 1.0,
            alpha: 1.0,
            clip: true,
        }
    }
}

/// Style for drawing text.
#[derive(Clone, Debug)]
pub struct TextStyle {
    pub color: (u8, u8, u8),
    pub size: f64,
    pub anchor: TextAnchor,
    pub angle: f64,
    /// Font family (e.g., "serif", "monospace"). None defaults to "sans-serif".
    pub family: Option<String>,
    /// Font face (R's `element_text(face = ...)`).
    pub face: FontFace,
}

/// Font face / weight (R's `element_text(face = ...)`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FontFace {
    #[default]
    Plain,
    Bold,
    Italic,
}

impl Default for TextStyle {
    fn default() -> Self {
        TextStyle {
            color: (50, 50, 50),
            size: 12.0,
            anchor: TextAnchor::Middle,
            angle: 0.0,
            family: None,
            face: crate::render::backend::FontFace::Plain,
        }
    }
}

#[derive(Clone, Debug)]
pub enum TextAnchor {
    Start,
    Middle,
    End,
}

/// Our rendering abstraction, independent of plotters details.
pub trait DrawBackend {
    fn draw_circle(
        &mut self,
        center: (f64, f64),
        radius: f64,
        style: &PointStyle,
    ) -> Result<(), RenderError>;
    fn draw_line(&mut self, points: &[(f64, f64)], style: &LineStyle) -> Result<(), RenderError>;
    fn draw_rect(
        &mut self,
        top_left: (f64, f64),
        bottom_right: (f64, f64),
        style: &RectStyle,
    ) -> Result<(), RenderError>;
    fn draw_text(
        &mut self,
        text: &str,
        pos: (f64, f64),
        style: &TextStyle,
    ) -> Result<(), RenderError>;
    fn draw_polygon(&mut self, points: &[(f64, f64)], style: &RectStyle)
        -> Result<(), RenderError>;
    fn plot_area(&self) -> Rect;
    fn total_area(&self) -> Rect;

    /// Draw a point with a specific shape. Default delegates to draw_circle for Circle.
    fn draw_shape(
        &mut self,
        center: (f64, f64),
        radius: f64,
        style: &PointStyle,
    ) -> Result<(), RenderError> {
        match style.shape {
            PointShape::Circle => self.draw_circle(center, radius, style),
            _ => {
                // Default: fall back to circle for unsupported backends
                self.draw_circle(center, radius, style)
            }
        }
    }
}
