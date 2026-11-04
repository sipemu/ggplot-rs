//! Manual and identity scales for the numeric aesthetics (#408):
//! ggplot2's `scale_size_manual()` / `scale_alpha_manual()` /
//! `scale_linewidth_manual()` ([`ScaleValueManual`]) and
//! `scale_size_identity()` / `scale_alpha_identity()` /
//! `scale_linewidth_identity()` / `scale_shape_identity()`
//! ([`ScaleValueIdentity`]).

use crate::aes::Aesthetic;
use crate::data::Value;
use crate::render::backend::PointShape;
use indexmap::IndexSet;

use super::Scale;

/// A discrete scale mapping named levels to user-chosen numbers — point
/// sizes, alpha values or line widths, by `aesthetic` (`Size`, `Alpha` or
/// `Linewidth`). Levels the data has beyond `values` cycle through them,
/// as [`ScaleShapeManual`](super::shape_manual::ScaleShapeManual) does.
#[derive(Clone, Debug)]
pub struct ScaleValueManual {
    aesthetic: Aesthetic,
    name: String,
    configured: Vec<String>,
    levels: IndexSet<String>,
    values: Vec<f64>,
}

impl ScaleValueManual {
    pub fn new(aesthetic: Aesthetic, values: Vec<(&str, f64)>) -> Self {
        let configured: Vec<String> = values.iter().map(|(k, _)| k.to_string()).collect();
        ScaleValueManual {
            aesthetic,
            name: String::new(),
            levels: configured.iter().cloned().collect(),
            configured,
            values: values.iter().map(|(_, v)| *v).collect(),
        }
    }

    fn value_for(&self, value: &Value) -> Option<f64> {
        if self.values.is_empty() {
            return None;
        }
        let idx = self
            .levels
            .get_index_of(value.key_str().as_ref())
            .unwrap_or(0);
        Some(self.values[idx % self.values.len()])
    }
}

impl Scale for ScaleValueManual {
    fn aesthetic(&self) -> Aesthetic {
        self.aesthetic.clone()
    }

    fn train(&mut self, values: &[Value]) {
        for v in values {
            if v.is_na() {
                continue;
            }
            let key = v.key_str();
            if !self.levels.contains(key.as_ref()) {
                self.levels.insert(key.into_owned());
            }
        }
    }

    fn map(&self, value: &Value) -> f64 {
        self.levels
            .get_index_of(value.key_str().as_ref())
            .map(|i| i as f64)
            .unwrap_or(0.0)
    }

    fn breaks(&self) -> Vec<(f64, String)> {
        self.levels
            .iter()
            .enumerate()
            .map(|(i, l)| (i as f64, l.clone()))
            .collect()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    fn is_discrete(&self) -> bool {
        true
    }

    fn map_to_size(&self, value: &Value) -> Option<f64> {
        match self.aesthetic {
            Aesthetic::Size | Aesthetic::Linewidth => self.value_for(value),
            _ => None,
        }
    }

    fn map_to_alpha(&self, value: &Value) -> Option<f64> {
        match self.aesthetic {
            Aesthetic::Alpha => self.value_for(value).map(|a| a.clamp(0.0, 1.0)),
            _ => None,
        }
    }

    fn clone_box(&self) -> Box<dyn Scale> {
        Box::new(self.clone())
    }

    fn reset_training(&mut self) {
        self.levels = self.configured.iter().cloned().collect();
    }
}

/// An identity scale: the data already holds the size / alpha / line width
/// (numbers) or point shape (ggplot2 `pch` codes, or a shape name such as
/// `"triangle"`) to draw with, used as-is. No legend, as in ggplot2
/// (`guide = "none"`).
#[derive(Clone, Debug)]
pub struct ScaleValueIdentity {
    aesthetic: Aesthetic,
}

impl ScaleValueIdentity {
    /// `aesthetic` is `Size`, `Alpha`, `Linewidth` or `Shape`.
    pub fn new(aesthetic: Aesthetic) -> Self {
        ScaleValueIdentity { aesthetic }
    }
}

/// A ggplot2 `pch` code or shape name as the nearest [`PointShape`].
pub(crate) fn shape_from_value(value: &Value) -> Option<PointShape> {
    if let Some(name) = value.as_str() {
        return match name.trim().to_ascii_lowercase().as_str() {
            "circle" | "circle filled" | "circle open" | "circle small" => Some(PointShape::Circle),
            "triangle" | "triangle filled" | "triangle open" => Some(PointShape::Triangle),
            "square" | "square filled" | "square open" => Some(PointShape::Square),
            "diamond" | "diamond filled" | "diamond open" => Some(PointShape::Diamond),
            "cross" => Some(PointShape::Cross),
            "plus" => Some(PointShape::Plus),
            n => n.parse::<f64>().ok().and_then(shape_from_code),
        };
    }
    value.as_f64().and_then(shape_from_code)
}

fn shape_from_code(f: f64) -> Option<PointShape> {
    if !f.is_finite() || f.fract() != 0.0 {
        return None;
    }
    match f as i64 {
        1 | 10 | 13 | 16 | 19 | 20 | 21 => Some(PointShape::Circle),
        2 | 6 | 17 | 24 | 25 => Some(PointShape::Triangle),
        0 | 7 | 12 | 14 | 15 | 22 => Some(PointShape::Square),
        5 | 9 | 18 | 23 => Some(PointShape::Diamond),
        4 | 8 => Some(PointShape::Cross),
        3 => Some(PointShape::Plus),
        _ => None,
    }
}

impl Scale for ScaleValueIdentity {
    fn aesthetic(&self) -> Aesthetic {
        self.aesthetic.clone()
    }

    fn train(&mut self, _values: &[Value]) {}

    fn map(&self, _value: &Value) -> f64 {
        0.0
    }

    fn breaks(&self) -> Vec<(f64, String)> {
        Vec::new()
    }

    fn name(&self) -> &str {
        ""
    }

    fn set_name(&mut self, _name: &str) {}

    fn map_to_size(&self, value: &Value) -> Option<f64> {
        match self.aesthetic {
            Aesthetic::Size | Aesthetic::Linewidth => {
                value.as_f64().filter(|f| f.is_finite() && *f >= 0.0)
            }
            _ => None,
        }
    }

    fn map_to_alpha(&self, value: &Value) -> Option<f64> {
        match self.aesthetic {
            Aesthetic::Alpha => value
                .as_f64()
                .filter(|f| f.is_finite())
                .map(|a| a.clamp(0.0, 1.0)),
            _ => None,
        }
    }

    fn map_to_shape(&self, value: &Value) -> Option<PointShape> {
        match self.aesthetic {
            Aesthetic::Shape => shape_from_value(value),
            _ => None,
        }
    }

    fn clone_box(&self) -> Box<dyn Scale> {
        Box::new(self.clone())
    }
}

/// A binned scale for a non-position aesthetic (#408): ggplot2's
/// `scale_size_binned()` / `scale_alpha_binned()` / `scale_shape_binned()` /
/// `scale_linetype_binned()` / `scale_linewidth_binned()`. A continuous value
/// is cut into round, right-closed bins between about `n` breaks; each bin gets one size /
/// alpha / line width (spread over `range`; sizes by area) or one shape /
/// linetype, and the legend shows one key per bin, labelled `lo–hi`.
#[derive(Clone, Debug)]
pub struct ScaleValueBinned {
    aesthetic: Aesthetic,
    name: String,
    n: usize,
    range: (f64, f64),
    min: f64,
    max: f64,
}

impl ScaleValueBinned {
    /// `aesthetic` is `Size`, `Alpha`, `Linewidth`, `Shape` or `Linetype`.
    /// The default output range is ggplot2's (size 1–6, alpha 0.1–1,
    /// linewidth 1–6).
    pub fn new(aesthetic: Aesthetic, n: usize) -> Self {
        let range = match aesthetic {
            Aesthetic::Alpha => (0.1, 1.0),
            _ => (1.0, 6.0),
        };
        ScaleValueBinned {
            aesthetic,
            name: String::new(),
            n: n.max(1),
            range,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        }
    }

    /// The output range for size / alpha / linewidth.
    pub fn with_range(mut self, lo: f64, hi: f64) -> Self {
        self.range = (lo, hi);
        self
    }

    fn edges(&self) -> Vec<f64> {
        if self.min > self.max {
            return vec![];
        }
        let (lo, hi) = if self.max > self.min {
            (self.min, self.max)
        } else {
            (self.min - 0.5, self.max + 0.5)
        };
        let mut e = super::util::extended_breaks(lo, hi, self.n);
        if e.len() < 2 {
            e = vec![lo, hi];
        }
        let step = e[1] - e[0];
        while e[0] > lo + step * 1e-9 {
            e.insert(0, e[0] - step);
        }
        while *e.last().unwrap() < hi - step * 1e-9 {
            e.push(e.last().unwrap() + step);
        }
        e
    }

    fn labels(&self, e: &[f64]) -> Vec<String> {
        let l = super::util::format_numbers_opt(e, None);
        l.windows(2).map(|w| format!("{}–{}", w[0], w[1])).collect()
    }

    /// The bin of `value` — a number, or a legend label passed back.
    fn bin(&self, value: &Value) -> Option<(usize, usize)> {
        let e = self.edges();
        let nb = e.len().checked_sub(1).filter(|n| *n > 0)?;
        if let Some(f) = value.as_f64().filter(|f| f.is_finite()) {
            let k = e.windows(2).position(|w| f <= w[1]).unwrap_or(nb - 1);
            return Some((k, nb));
        }
        let s = value.as_str()?;
        self.labels(&e).iter().position(|l| l == s).map(|k| (k, nb))
    }

    fn fraction(&self, value: &Value) -> Option<f64> {
        self.bin(value).map(|(k, nb)| {
            if nb > 1 {
                k as f64 / (nb - 1) as f64
            } else {
                0.5
            }
        })
    }
}

impl Scale for ScaleValueBinned {
    fn aesthetic(&self) -> Aesthetic {
        self.aesthetic.clone()
    }

    fn train(&mut self, values: &[Value]) {
        for f in values
            .iter()
            .filter_map(Value::as_f64)
            .filter(|f| f.is_finite())
        {
            self.min = self.min.min(f);
            self.max = self.max.max(f);
        }
    }

    fn map(&self, value: &Value) -> f64 {
        self.bin(value).map_or(0.0, |(k, _)| k as f64)
    }

    fn breaks(&self) -> Vec<(f64, String)> {
        let e = self.edges();
        self.labels(&e)
            .into_iter()
            .enumerate()
            .map(|(i, l)| (i as f64, l))
            .collect()
    }

    fn bin_edge_labels(&self) -> Option<Vec<String>> {
        let e = self.edges();
        (e.len() >= 2).then(|| super::util::format_numbers_opt(&e, None))
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    fn is_discrete(&self) -> bool {
        true
    }

    fn map_to_size(&self, value: &Value) -> Option<f64> {
        let t = self.fraction(value)?;
        let (lo, hi) = self.range;
        match self.aesthetic {
            // ggplot2's size scales spread *area*.
            Aesthetic::Size => Some((lo * lo + t * (hi * hi - lo * lo)).max(0.0).sqrt()),
            Aesthetic::Linewidth => Some(lo + t * (hi - lo)),
            _ => None,
        }
    }

    fn map_to_alpha(&self, value: &Value) -> Option<f64> {
        let t = self.fraction(value)?;
        let (lo, hi) = self.range;
        matches!(self.aesthetic, Aesthetic::Alpha).then(|| (lo + t * (hi - lo)).clamp(0.0, 1.0))
    }

    fn map_to_shape(&self, value: &Value) -> Option<PointShape> {
        let (k, _) = self.bin(value)?;
        matches!(self.aesthetic, Aesthetic::Shape)
            .then(|| PointShape::ALL[k % PointShape::ALL.len()])
    }

    fn map_to_linetype(&self, value: &Value) -> Option<crate::render::backend::Linetype> {
        use crate::render::backend::Linetype as L;
        let (k, _) = self.bin(value)?;
        let all = [
            L::Solid,
            L::Dashed,
            L::Dotted,
            L::DashDot,
            L::LongDash,
            L::TwoDash,
        ];
        matches!(self.aesthetic, Aesthetic::Linetype).then(|| all[k % all.len()].clone())
    }

    fn clone_box(&self) -> Box<dyn Scale> {
        Box::new(self.clone())
    }

    fn reset_training(&mut self) {
        self.min = f64::INFINITY;
        self.max = f64::NEG_INFINITY;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_maps_levels_and_cycles() {
        let mut s = ScaleValueManual::new(Aesthetic::Size, vec![("a", 2.0), ("b", 6.0)]);
        s.train(&[Value::Str("c".into())]);
        assert_eq!(s.map_to_size(&Value::Str("b".into())), Some(6.0));
        assert_eq!(s.map_to_size(&Value::Str("c".into())), Some(2.0));
        assert_eq!(s.map_to_alpha(&Value::Str("a".into())), None);
        assert_eq!(s.breaks().len(), 3);
        s.reset_training();
        assert_eq!(s.breaks().len(), 2);
        let a = ScaleValueManual::new(Aesthetic::Alpha, vec![("a", 1.5)]);
        assert_eq!(a.map_to_alpha(&Value::Str("a".into())), Some(1.0));
    }

    #[test]
    fn binned_maps_by_bin() {
        let mut s = ScaleValueBinned::new(Aesthetic::Size, 5);
        s.train(&[Value::Float(0.0), Value::Float(100.0)]);
        let b = s.breaks();
        assert_eq!(b.len(), 4, "{b:?}");
        assert_eq!(b[0].1, "0–25");
        let lo = s.map_to_size(&Value::Float(10.0)).unwrap();
        let hi = s.map_to_size(&Value::Float(90.0)).unwrap();
        assert!((lo - 1.0).abs() < 1e-9 && (hi - 6.0).abs() < 1e-9);
        assert_eq!(s.map_to_size(&Value::Str("75–100".into())), Some(hi));
        // 25 is right-closed: in the first bin.
        assert_eq!(s.map_to_size(&Value::Float(25.0)), Some(lo));
        let mut sh = ScaleValueBinned::new(Aesthetic::Shape, 5);
        sh.train(&[Value::Float(0.0), Value::Float(100.0)]);
        assert_eq!(
            sh.map_to_shape(&Value::Float(60.0)),
            Some(PointShape::ALL[2])
        );
        let mut a = ScaleValueBinned::new(Aesthetic::Alpha, 5);
        a.train(&[Value::Float(0.0), Value::Float(100.0)]);
        assert_eq!(a.map_to_alpha(&Value::Float(0.0)), Some(0.1));
    }

    #[test]
    fn identity_uses_the_values() {
        let s = ScaleValueIdentity::new(Aesthetic::Size);
        assert_eq!(s.map_to_size(&Value::Float(3.5)), Some(3.5));
        assert!(s.breaks().is_empty());
        let a = ScaleValueIdentity::new(Aesthetic::Alpha);
        assert_eq!(a.map_to_alpha(&Value::Float(0.25)), Some(0.25));
        let sh = ScaleValueIdentity::new(Aesthetic::Shape);
        assert_eq!(
            sh.map_to_shape(&Value::Integer(17)),
            Some(PointShape::Triangle)
        );
        assert_eq!(
            sh.map_to_shape(&Value::Str("diamond".into())),
            Some(PointShape::Diamond)
        );
        assert_eq!(
            sh.map_to_shape(&Value::Str("4".into())),
            Some(PointShape::Cross)
        );
        assert_eq!(sh.map_to_shape(&Value::Float(99.0)), None);
    }
}
