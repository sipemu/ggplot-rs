//! Censor marks for survival curves: `+` glyphs at the (time, survival)
//! points where observations were censored (ggsurvfit's `add_censor_mark`).

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, PointShape};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::point::{draw_points, PointDefaults};
use super::{Geom, GeomParams};

/// Points drawn with a fixed shape (default `+`) — censor marks on a
/// Kaplan–Meier curve. Mapped `color`/`size`/`alpha`/`shape` still win.
pub struct GeomCensorMarks {
    pub size: f64,
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub shape: PointShape,
}

impl Default for GeomCensorMarks {
    fn default() -> Self {
        GeomCensorMarks {
            size: 4.0,
            color: (0, 0, 0),
            alpha: 1.0,
            shape: PointShape::Plus,
        }
    }
}

impl Geom for GeomCensorMarks {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        draw_points(
            data,
            coord,
            scales,
            backend,
            PointDefaults {
                size: self.size,
                color: self.color,
                alpha: self.alpha,
                shape: self.shape,
            },
        )
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }
    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatIdentity)
    }
    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }
    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }
    fn name(&self) -> &str {
        "censor_marks"
    }
    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.color = color;
    }
}

/// Keep only the rows whose `column` marks a censoring: a number `> 0`
/// (e.g. `n_censor`), `true`, or the strings `"true"`/`"1"`/`"censored"`.
#[derive(Clone, Debug)]
pub struct StatCensored {
    pub column: String,
}

impl StatCensored {
    pub fn new(column: &str) -> Self {
        StatCensored {
            column: column.to_string(),
        }
    }
}

fn is_censored(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::Str(s) => matches!(
            s.trim().to_ascii_lowercase().as_str(),
            "true" | "1" | "censored" | "yes"
        ),
        Value::Na => false,
        other => other.as_f64().is_some_and(|f| f > 0.0 && f.is_finite()),
    }
}

impl Stat for StatCensored {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let Some(flag) = data.column(&self.column) else {
            return DataFrame::new();
        };
        let keep: Vec<usize> = (0..data.nrows())
            .filter(|&i| is_censored(&flag[i]))
            .collect();
        let mut out = DataFrame::new();
        for name in data.column_names() {
            if let Some(col) = data.column(name) {
                out.add_column(
                    name.to_string(),
                    keep.iter().map(|&i| col[i].clone()).collect(),
                );
            }
        }
        out
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn name(&self) -> &str {
        "censored"
    }
}
