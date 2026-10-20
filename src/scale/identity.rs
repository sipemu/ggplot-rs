use crate::aes::Aesthetic;
use crate::data::Value;

use super::Scale;

/// Parse a `#RRGGBB` or `#RGB` hex colour string. Anything else (missing
/// `#`, wrong length, non-hex digits) is `None`.
fn parse_hex(s: &str) -> Option<(u8, u8, u8)> {
    let s = s.strip_prefix('#')?;
    let digit = |c: u8| -> Option<u8> {
        match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            b'A'..=b'F' => Some(c - b'A' + 10),
            _ => None,
        }
    };
    let bytes = s.as_bytes();
    match bytes.len() {
        6 => {
            let byte =
                |i: usize| -> Option<u8> { Some(digit(bytes[i])? * 16 + digit(bytes[i + 1])?) };
            Some((byte(0)?, byte(2)?, byte(4)?))
        }
        3 => {
            let byte = |i: usize| -> Option<u8> {
                let d = digit(bytes[i])?;
                Some(d * 16 + d)
            };
            Some((byte(0)?, byte(1)?, byte(2)?))
        }
        _ => None,
    }
}

/// Identity colour/fill scale (R's `scale_colour_identity`/
/// `scale_fill_identity`): the data column already holds `#RRGGBB`/`#RGB`
/// hex colour strings, used as-is. No legend (`breaks()` is empty). An
/// invalid value maps to `None`, so the geom's own default colour applies.
#[derive(Clone)]
pub struct ScaleColorIdentity {
    aesthetic: Aesthetic,
}

impl ScaleColorIdentity {
    /// `aes` is [`Aesthetic::Color`] or [`Aesthetic::Fill`] — whichever
    /// aesthetic this identity scale takes over.
    pub fn new(aes: Aesthetic) -> Self {
        ScaleColorIdentity { aesthetic: aes }
    }
}

impl Scale for ScaleColorIdentity {
    fn aesthetic(&self) -> Aesthetic {
        self.aesthetic.clone()
    }

    fn train(&mut self, _values: &[Value]) {
        // No domain to train: every value maps to its own literal colour.
    }

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

    fn map_to_color(&self, value: &Value) -> Option<(u8, u8, u8)> {
        value.as_str().and_then(parse_hex)
    }

    fn clone_box(&self) -> Box<dyn Scale> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_6_and_3_digit_hex() {
        assert_eq!(parse_hex("#FF0000"), Some((255, 0, 0)));
        assert_eq!(parse_hex("#00ff00"), Some((0, 255, 0)));
        assert_eq!(parse_hex("#00F"), Some((0, 0, 255)));
        assert_eq!(parse_hex("#abc"), Some((0xaa, 0xbb, 0xcc)));
    }

    #[test]
    fn invalid_values_map_to_none() {
        for bad in ["red", "#GGG", "#12345", "", "#"] {
            assert_eq!(parse_hex(bad), None, "{bad} should not parse");
        }
        let scale = ScaleColorIdentity::new(Aesthetic::Color);
        assert_eq!(scale.map_to_color(&Value::Str("not-a-colour".into())), None);
        assert_eq!(scale.map_to_color(&Value::Float(1.0)), None);
    }

    #[test]
    fn no_legend() {
        let scale = ScaleColorIdentity::new(Aesthetic::Fill);
        assert!(scale.breaks().is_empty());
    }

    #[test]
    fn renders_through_the_plot_with_no_legend_and_falls_back_on_invalid_values() {
        use crate::data::Value;
        use crate::prelude::*;

        let svg = GGPlot::new(vec![
            (
                "x".into(),
                vec![Value::Str("a".into()), Value::Str("b".into())],
            ),
            ("y".into(), vec![Value::Float(1.0), Value::Float(2.0)]),
            (
                "fill".into(),
                vec![Value::Str("#FF0000".into()), Value::Str("bogus".into())],
            ),
        ])
        .aes(Aes::new().x("x").y("y").fill("fill"))
        .geom_col()
        .scale_fill_identity()
        .render_svg_native_with_size(200, 200)
        .unwrap();

        assert!(svg.contains("#FF0000"));
        // No legend: the "fill" scale produces no breaks, so no legend title.
        assert!(!svg.contains(">fill<"));
    }
}
