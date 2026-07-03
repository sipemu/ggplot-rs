pub mod backend;
pub mod layout;
pub mod plotters_backend;
pub mod renderer;
pub mod svg_backend;

/// A rectangle in pixel coordinates.
#[derive(Clone, Debug)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Rendering error type.
#[derive(Debug)]
pub enum RenderError {
    MissingAesthetic(String),
    BackendError(String),
    IoError(std::io::Error),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::MissingAesthetic(a) => write!(f, "Missing required aesthetic: {a}"),
            RenderError::BackendError(e) => write!(f, "Backend error: {e}"),
            RenderError::IoError(e) => write!(f, "IO error: {e}"),
        }
    }
}

impl std::error::Error for RenderError {}

impl From<std::io::Error> for RenderError {
    fn from(e: std::io::Error) -> Self {
        RenderError::IoError(e)
    }
}
