/// A secondary axis specification — defines a transformation from the primary axis domain.
///
/// # Example
/// ```
/// use ggplot_rs::prelude::*;
///
/// // Temperature in Celsius on primary, Fahrenheit on secondary
/// let sec = SecAxis::new(|c| c * 9.0 / 5.0 + 32.0).with_name("Fahrenheit");
/// ```
#[derive(Clone)]
pub struct SecAxis {
    /// Transformation from primary axis values to secondary axis values.
    pub transform: fn(f64) -> f64,
    /// Name / title for the secondary axis.
    pub name: String,
    /// Custom break positions in primary axis coordinates.
    pub breaks: Option<Vec<f64>>,
}

impl SecAxis {
    /// Create a secondary axis with the given transformation function.
    pub fn new(transform: fn(f64) -> f64) -> Self {
        SecAxis {
            transform,
            name: String::new(),
            breaks: None,
        }
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn with_breaks(mut self, breaks: Vec<f64>) -> Self {
        self.breaks = Some(breaks);
        self
    }

    /// Transform a primary axis value to the secondary axis value.
    pub fn transform_value(&self, v: f64) -> f64 {
        (self.transform)(v)
    }
}

impl std::fmt::Debug for SecAxis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecAxis")
            .field("name", &self.name)
            .field("breaks", &self.breaks)
            .finish()
    }
}
