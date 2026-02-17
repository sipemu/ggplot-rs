pub mod mapping;

pub use mapping::resolve_mappings;

/// All supported aesthetic channels.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Aesthetic {
    X,
    Y,
    Color,
    Fill,
    Size,
    Shape,
    Alpha,
    Linetype,
    Group,
    Ymin,
    Ymax,
    Xmin,
    Xmax,
    Label,
    Weight,
    Xend,
    Yend,
}

impl Aesthetic {
    /// The canonical column name used in the working DataFrame after aes evaluation.
    pub fn col_name(&self) -> &str {
        match self {
            Aesthetic::X => "x",
            Aesthetic::Y => "y",
            Aesthetic::Color => "color",
            Aesthetic::Fill => "fill",
            Aesthetic::Size => "size",
            Aesthetic::Shape => "shape",
            Aesthetic::Alpha => "alpha",
            Aesthetic::Linetype => "linetype",
            Aesthetic::Group => "group",
            Aesthetic::Ymin => "ymin",
            Aesthetic::Ymax => "ymax",
            Aesthetic::Xmin => "xmin",
            Aesthetic::Xmax => "xmax",
            Aesthetic::Label => "label",
            Aesthetic::Weight => "weight",
            Aesthetic::Xend => "xend",
            Aesthetic::Yend => "yend",
        }
    }
}

/// Maps a source column to an aesthetic channel.
#[derive(Clone, Debug)]
pub struct AesMapping {
    pub column: String,
    pub aesthetic: Aesthetic,
}

/// Builder for aesthetic mappings.
#[derive(Clone, Debug, Default)]
pub struct Aes {
    pub mappings: Vec<AesMapping>,
}

impl Aes {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn x(mut self, col: &str) -> Self {
        self.mappings.push(AesMapping {
            column: col.to_string(),
            aesthetic: Aesthetic::X,
        });
        self
    }

    pub fn y(mut self, col: &str) -> Self {
        self.mappings.push(AesMapping {
            column: col.to_string(),
            aesthetic: Aesthetic::Y,
        });
        self
    }

    pub fn color(mut self, col: &str) -> Self {
        self.mappings.push(AesMapping {
            column: col.to_string(),
            aesthetic: Aesthetic::Color,
        });
        self
    }

    pub fn fill(mut self, col: &str) -> Self {
        self.mappings.push(AesMapping {
            column: col.to_string(),
            aesthetic: Aesthetic::Fill,
        });
        self
    }

    pub fn size(mut self, col: &str) -> Self {
        self.mappings.push(AesMapping {
            column: col.to_string(),
            aesthetic: Aesthetic::Size,
        });
        self
    }

    pub fn shape(mut self, col: &str) -> Self {
        self.mappings.push(AesMapping {
            column: col.to_string(),
            aesthetic: Aesthetic::Shape,
        });
        self
    }

    pub fn alpha(mut self, col: &str) -> Self {
        self.mappings.push(AesMapping {
            column: col.to_string(),
            aesthetic: Aesthetic::Alpha,
        });
        self
    }

    pub fn group(mut self, col: &str) -> Self {
        self.mappings.push(AesMapping {
            column: col.to_string(),
            aesthetic: Aesthetic::Group,
        });
        self
    }

    pub fn ymin(mut self, col: &str) -> Self {
        self.mappings.push(AesMapping {
            column: col.to_string(),
            aesthetic: Aesthetic::Ymin,
        });
        self
    }

    pub fn ymax(mut self, col: &str) -> Self {
        self.mappings.push(AesMapping {
            column: col.to_string(),
            aesthetic: Aesthetic::Ymax,
        });
        self
    }

    pub fn label(mut self, col: &str) -> Self {
        self.mappings.push(AesMapping {
            column: col.to_string(),
            aesthetic: Aesthetic::Label,
        });
        self
    }

    pub fn weight(mut self, col: &str) -> Self {
        self.mappings.push(AesMapping {
            column: col.to_string(),
            aesthetic: Aesthetic::Weight,
        });
        self
    }

    pub fn xend(mut self, col: &str) -> Self {
        self.mappings.push(AesMapping {
            column: col.to_string(),
            aesthetic: Aesthetic::Xend,
        });
        self
    }

    pub fn yend(mut self, col: &str) -> Self {
        self.mappings.push(AesMapping {
            column: col.to_string(),
            aesthetic: Aesthetic::Yend,
        });
        self
    }

    pub fn linetype(mut self, col: &str) -> Self {
        self.mappings.push(AesMapping {
            column: col.to_string(),
            aesthetic: Aesthetic::Linetype,
        });
        self
    }

    /// Get the column mapped to a specific aesthetic.
    pub fn get_mapping(&self, aes: &Aesthetic) -> Option<&str> {
        self.mappings
            .iter()
            .find(|m| m.aesthetic == *aes)
            .map(|m| m.column.as_str())
    }

    /// Merge another Aes into this one. The other's mappings override on conflict.
    pub fn merge(&self, other: &Aes) -> Aes {
        let mut result = self.clone();
        for m in &other.mappings {
            // Remove existing mapping for same aesthetic
            result.mappings.retain(|existing| existing.aesthetic != m.aesthetic);
            result.mappings.push(m.clone());
        }
        result
    }
}
