use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// How consecutive points are joined (ggplot2 4.0's `stat_connect(connection)`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Connection {
    /// Horizontal then vertical (`"hv"`, a step that holds `y` until the next x).
    #[default]
    Hv,
    /// Vertical then horizontal (`"vh"`).
    Vh,
    /// Step at the midpoint between x values (`"mid"`).
    Mid,
    /// Straight lines — the points unchanged (`"linear"`).
    Linear,
}

/// Inserts the corner points that join each group's consecutive points (in x
/// order) with `connection`, for a path geom — ggplot2's `stat_connect()`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StatConnect {
    pub connection: Connection,
}

impl StatConnect {
    pub fn new(connection: Connection) -> Self {
        StatConnect { connection }
    }
}

impl Stat for StatConnect {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let (Some(xc), Some(yc)) = (data.column("x"), data.column("y")) else {
            return DataFrame::new();
        };
        let mut idx: Vec<usize> = (0..data.nrows())
            .filter(|&i| xc[i].as_f64().is_some() && yc[i].as_f64().is_some())
            .collect();
        idx.sort_by(|&a, &b| xc[a].as_f64().unwrap().total_cmp(&xc[b].as_f64().unwrap()));
        // (source row whose other columns the point carries, x, y)
        let mut pts: Vec<(usize, f64, f64)> = Vec::new();
        for (k, &i) in idx.iter().enumerate() {
            let (x, y) = (xc[i].as_f64().unwrap(), yc[i].as_f64().unwrap());
            if let Some(&prev) = k.checked_sub(1).map(|p| &idx[p]) {
                let (px, py) = (xc[prev].as_f64().unwrap(), yc[prev].as_f64().unwrap());
                match self.connection {
                    Connection::Hv => pts.push((prev, x, py)),
                    Connection::Vh => pts.push((prev, px, y)),
                    Connection::Mid => {
                        let m = (px + x) / 2.0;
                        pts.push((prev, m, py));
                        pts.push((prev, m, y));
                    }
                    Connection::Linear => {}
                }
            }
            // `mid` (as ggplot2) keeps only the end points of the original
            // ones: the interior ones sit on the horizontal runs.
            let interior = k > 0 && k + 1 < idx.len();
            if !(self.connection == Connection::Mid && interior) {
                pts.push((i, x, y));
            }
        }
        let mut out = DataFrame::new();
        for name in data.column_names() {
            let src = data.column(name).unwrap();
            let col: Vec<Value> = match name {
                "x" => pts.iter().map(|p| Value::Float(p.1)).collect(),
                "y" => pts.iter().map(|p| Value::Float(p.2)).collect(),
                _ => pts.iter().map(|p| src[p.0].clone()).collect(),
            };
            out.add_column(name.to_string(), col);
        }
        out
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn name(&self) -> &str {
        "connect"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(c: Connection) -> Vec<(f64, f64)> {
        let mut df = DataFrame::new();
        df.add_column(
            "x".into(),
            vec![Value::Float(3.0), Value::Float(1.0), Value::Float(2.0)],
        );
        df.add_column(
            "y".into(),
            vec![Value::Float(30.0), Value::Float(10.0), Value::Float(20.0)],
        );
        let out = StatConnect::new(c).compute_group(&df, &ScaleSet::new());
        let x = out.column("x").unwrap();
        let y = out.column("y").unwrap();
        (0..out.nrows())
            .map(|i| (x[i].as_f64().unwrap(), y[i].as_f64().unwrap()))
            .collect()
    }

    #[test]
    fn connections_match_ggplot2() {
        // ggplot2 4.0.3 layer_data(stat_connect(connection = …)) on
        // (1,10),(2,20),(3,30). "linear" there repeats the interior points
        // (same drawn line); we don't.
        assert_eq!(
            run(Connection::Hv),
            [
                (1.0, 10.0),
                (2.0, 10.0),
                (2.0, 20.0),
                (3.0, 20.0),
                (3.0, 30.0)
            ]
        );
        assert_eq!(
            run(Connection::Vh),
            [
                (1.0, 10.0),
                (1.0, 20.0),
                (2.0, 20.0),
                (2.0, 30.0),
                (3.0, 30.0)
            ]
        );
        assert_eq!(
            run(Connection::Mid),
            [
                (1.0, 10.0),
                (1.5, 10.0),
                (1.5, 20.0),
                (2.5, 20.0),
                (2.5, 30.0),
                (3.0, 30.0)
            ]
        );
        assert_eq!(
            run(Connection::Linear),
            [(1.0, 10.0), (2.0, 20.0), (3.0, 30.0)]
        );
    }
}
