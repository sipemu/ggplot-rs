pub mod mapping;

pub use mapping::{apply_after_stat, resolve_mappings};

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
    Angle,
    Radius,
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
            Aesthetic::Angle => "angle",
            Aesthetic::Radius => "radius",
        }
    }
}

/// When an aesthetic mapping should be resolved.
#[derive(Clone, Debug, PartialEq)]
pub enum MappingStage {
    /// Resolve before stat computation (default — maps from raw data columns).
    BeforeStat,
    /// Resolve after stat computation (maps from stat-computed columns like `density`, `count`).
    AfterStat,
}

/// Maps a source column to an aesthetic channel.
#[derive(Clone, Debug)]
pub struct AesMapping {
    pub column: String,
    pub aesthetic: Aesthetic,
    pub stage: MappingStage,
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

    fn push(mut self, col: &str, aesthetic: Aesthetic) -> Self {
        self.mappings.push(AesMapping {
            column: col.to_string(),
            aesthetic,
            stage: MappingStage::BeforeStat,
        });
        self
    }

    fn push_after_stat(mut self, col: &str, aesthetic: Aesthetic) -> Self {
        self.mappings.push(AesMapping {
            column: col.to_string(),
            aesthetic,
            stage: MappingStage::AfterStat,
        });
        self
    }

    pub fn x(self, col: &str) -> Self {
        self.push(col, Aesthetic::X)
    }
    pub fn y(self, col: &str) -> Self {
        self.push(col, Aesthetic::Y)
    }
    pub fn color(self, col: &str) -> Self {
        self.push(col, Aesthetic::Color)
    }
    pub fn fill(self, col: &str) -> Self {
        self.push(col, Aesthetic::Fill)
    }
    pub fn size(self, col: &str) -> Self {
        self.push(col, Aesthetic::Size)
    }
    pub fn shape(self, col: &str) -> Self {
        self.push(col, Aesthetic::Shape)
    }
    pub fn alpha(self, col: &str) -> Self {
        self.push(col, Aesthetic::Alpha)
    }
    pub fn group(self, col: &str) -> Self {
        self.push(col, Aesthetic::Group)
    }
    pub fn ymin(self, col: &str) -> Self {
        self.push(col, Aesthetic::Ymin)
    }
    pub fn ymax(self, col: &str) -> Self {
        self.push(col, Aesthetic::Ymax)
    }
    pub fn label(self, col: &str) -> Self {
        self.push(col, Aesthetic::Label)
    }
    pub fn weight(self, col: &str) -> Self {
        self.push(col, Aesthetic::Weight)
    }
    pub fn xend(self, col: &str) -> Self {
        self.push(col, Aesthetic::Xend)
    }
    pub fn yend(self, col: &str) -> Self {
        self.push(col, Aesthetic::Yend)
    }
    pub fn xmin(self, col: &str) -> Self {
        self.push(col, Aesthetic::Xmin)
    }
    pub fn xmax(self, col: &str) -> Self {
        self.push(col, Aesthetic::Xmax)
    }
    pub fn angle(self, col: &str) -> Self {
        self.push(col, Aesthetic::Angle)
    }
    pub fn radius(self, col: &str) -> Self {
        self.push(col, Aesthetic::Radius)
    }
    pub fn linetype(self, col: &str) -> Self {
        self.push(col, Aesthetic::Linetype)
    }

    // ─── after_stat() mappings ──────────────────────────────────
    // Map stat-computed columns (e.g., `density`, `count`, `ncount`, `ndensity`)
    // to an aesthetic. These are resolved after the stat step in the build pipeline.

    /// Map a stat-computed column to the y aesthetic (e.g., `after_stat_y("density")`).
    pub fn after_stat_y(self, col: &str) -> Self {
        self.push_after_stat(col, Aesthetic::Y)
    }

    /// Map a stat-computed column to the x aesthetic.
    pub fn after_stat_x(self, col: &str) -> Self {
        self.push_after_stat(col, Aesthetic::X)
    }

    /// Map a stat-computed column to the fill aesthetic.
    pub fn after_stat_fill(self, col: &str) -> Self {
        self.push_after_stat(col, Aesthetic::Fill)
    }

    /// Map a stat-computed column to the color aesthetic.
    pub fn after_stat_color(self, col: &str) -> Self {
        self.push_after_stat(col, Aesthetic::Color)
    }

    /// Map a stat-computed column to the size aesthetic.
    pub fn after_stat_size(self, col: &str) -> Self {
        self.push_after_stat(col, Aesthetic::Size)
    }

    /// Map a stat-computed column to the alpha aesthetic.
    pub fn after_stat_alpha(self, col: &str) -> Self {
        self.push_after_stat(col, Aesthetic::Alpha)
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
            result
                .mappings
                .retain(|existing| existing.aesthetic != m.aesthetic);
            result.mappings.push(m.clone());
        }
        result
    }
}
