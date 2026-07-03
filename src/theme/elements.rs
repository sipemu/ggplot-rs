/// Text element styling.
#[derive(Clone, Debug)]
pub struct ElementText {
    pub family: String,
    /// Font face (R's `element_text(face = ...)`).
    pub face: crate::render::backend::FontFace,
    pub size: f64,
    pub color: (u8, u8, u8),
    pub angle: f64,
    pub hjust: f64,
    pub vjust: f64,
    pub visible: bool,
}

impl ElementText {
    /// Create an invisible (blank) text element.
    pub fn blank() -> Self {
        ElementText {
            visible: false,
            ..Default::default()
        }
    }
}

impl Default for ElementText {
    fn default() -> Self {
        ElementText {
            family: "sans-serif".to_string(),
            face: crate::render::backend::FontFace::Plain,
            size: 12.0,
            color: (0, 0, 0),
            angle: 0.0,
            hjust: 0.5,
            vjust: 0.5,
            visible: true,
        }
    }
}

/// Line element styling.
#[derive(Clone, Debug)]
pub struct ElementLine {
    pub color: (u8, u8, u8),
    pub width: f64,
    pub visible: bool,
    /// Line style (R's `element_line(linetype = ...)`).
    pub linetype: crate::render::backend::Linetype,
}

impl ElementLine {
    /// Create an invisible (blank) line element.
    pub fn blank() -> Self {
        ElementLine {
            visible: false,
            ..Default::default()
        }
    }
}

impl Default for ElementLine {
    fn default() -> Self {
        ElementLine {
            color: (0, 0, 0),
            width: 1.0,
            visible: true,
            linetype: crate::render::backend::Linetype::Solid,
        }
    }
}

/// Rectangle element styling.
#[derive(Clone, Debug)]
pub struct ElementRect {
    pub fill: Option<(u8, u8, u8)>,
    pub color: Option<(u8, u8, u8)>,
    pub width: f64,
    pub visible: bool,
}

impl ElementRect {
    /// Create an invisible (blank) rect element.
    pub fn blank() -> Self {
        ElementRect {
            visible: false,
            ..Default::default()
        }
    }
}

impl Default for ElementRect {
    fn default() -> Self {
        ElementRect {
            fill: Some((255, 255, 255)),
            color: None,
            width: 0.0,
            visible: true,
        }
    }
}
