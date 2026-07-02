//! A tiny arithmetic expression evaluator for **computed aesthetics** — so
//! `aes` can map e.g. `"pop / 1e6"`, `"log(gdp)"`, or `"a * b + 1"` instead of a
//! bare column name. Anything that isn't an existing column is parsed and
//! evaluated per row against the data's numeric columns.
//!
//! Grammar (standard precedence, `^` right-associative):
//! `expr := term (('+'|'-') term)*`, `term := factor (('*'|'/'|'%') factor)*`,
//! `factor := unary ('^' factor)?`, `unary := ('-'|'+') unary | primary`,
//! `primary := number | ident | ident '(' expr ')' | '(' expr ')'`.
//! Functions: `ln`/`log`, `log10`, `log2`, `sqrt`, `exp`, `abs`, `sin`, `cos`,
//! `tan`, `floor`, `ceil`, `round`, `sign`. Aggregate functions reduce their
//! argument over *all* rows to a scalar (broadcast to every row): `sum`, `mean`
//! (`avg`), `max`, `min`, `count`, `median`, `prod` — enabling normalized
//! `after_stat` mappings such as `"count / sum(count)"`.

use crate::data::{DataFrame, Value};

#[derive(Debug, Clone)]
enum Expr {
    Num(f64),
    Col(String),
    Neg(Box<Expr>),
    Bin(char, Box<Expr>, Box<Expr>),
    Func(String, Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Op(char),
}

fn tokenize(s: &str) -> Option<Vec<Tok>> {
    let chars: Vec<char> = s.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit()
            || (c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit())
        {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                i += 1;
                if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
                    i += 1;
                }
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let num: String = chars[start..i].iter().collect();
            toks.push(Tok::Num(num.parse().ok()?));
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len()
                && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '.')
            {
                i += 1;
            }
            toks.push(Tok::Ident(chars[start..i].iter().collect()));
        } else if "+-*/%^()".contains(c) {
            toks.push(Tok::Op(c));
            i += 1;
        } else {
            return None; // unknown character → not an expression
        }
    }
    Some(toks)
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }
    fn eat_op(&mut self, c: char) -> bool {
        if matches!(self.peek(), Some(Tok::Op(o)) if *o == c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn expr(&mut self) -> Option<Expr> {
        let mut left = self.term()?;
        while let Some(Tok::Op(c @ ('+' | '-'))) = self.peek().cloned() {
            self.pos += 1;
            let right = self.term()?;
            left = Expr::Bin(c, Box::new(left), Box::new(right));
        }
        Some(left)
    }
    fn term(&mut self) -> Option<Expr> {
        let mut left = self.factor()?;
        while let Some(Tok::Op(c @ ('*' | '/' | '%'))) = self.peek().cloned() {
            self.pos += 1;
            let right = self.factor()?;
            left = Expr::Bin(c, Box::new(left), Box::new(right));
        }
        Some(left)
    }
    fn factor(&mut self) -> Option<Expr> {
        let base = self.unary()?;
        if self.eat_op('^') {
            let exp = self.factor()?; // right-associative
            return Some(Expr::Bin('^', Box::new(base), Box::new(exp)));
        }
        Some(base)
    }
    fn unary(&mut self) -> Option<Expr> {
        if self.eat_op('-') {
            return Some(Expr::Neg(Box::new(self.unary()?)));
        }
        if self.eat_op('+') {
            return self.unary();
        }
        self.primary()
    }
    fn primary(&mut self) -> Option<Expr> {
        let tok = self.toks.get(self.pos).cloned()?;
        self.pos += 1;
        match tok {
            Tok::Num(n) => Some(Expr::Num(n)),
            Tok::Op('(') => {
                let e = self.expr()?;
                self.eat_op(')').then_some(e)
            }
            Tok::Ident(name) => {
                if self.eat_op('(') {
                    let arg = self.expr()?;
                    if !self.eat_op(')') {
                        return None;
                    }
                    Some(Expr::Func(name.to_lowercase(), Box::new(arg)))
                } else {
                    Some(Expr::Col(name))
                }
            }
            _ => None,
        }
    }
}

fn parse(s: &str) -> Option<Expr> {
    let toks = tokenize(s)?;
    if toks.is_empty() {
        return None;
    }
    let mut p = Parser { toks, pos: 0 };
    let e = p.expr()?;
    (p.pos == p.toks.len()).then_some(e)
}

fn eval(e: &Expr, data: &DataFrame, row: usize) -> Option<f64> {
    match e {
        Expr::Num(n) => Some(*n),
        Expr::Col(name) => data
            .column(name)
            .and_then(|c| c.get(row))
            .and_then(|v| v.as_f64()),
        Expr::Neg(a) => Some(-eval(a, data, row)?),
        Expr::Bin(op, a, b) => {
            let (x, y) = (eval(a, data, row)?, eval(b, data, row)?);
            Some(match op {
                '+' => x + y,
                '-' => x - y,
                '*' => x * y,
                '/' => x / y,
                '%' => x % y,
                '^' => x.powf(y),
                _ => return None,
            })
        }
        Expr::Func(name, a) => {
            // Aggregate functions reduce the argument over all rows to a scalar,
            // broadcast identically to every row (e.g. sum(count) for proportions).
            if let Some(agg) = aggregate(name) {
                let vals: Vec<f64> = (0..data.nrows())
                    .filter_map(|r| eval(a, data, r))
                    .filter(|v| v.is_finite())
                    .collect();
                return Some(agg(&vals));
            }
            let x = eval(a, data, row)?;
            Some(match name.as_str() {
                "ln" | "log" => x.ln(),
                "log10" => x.log10(),
                "log2" => x.log2(),
                "sqrt" => x.sqrt(),
                "exp" => x.exp(),
                "abs" => x.abs(),
                "sin" => x.sin(),
                "cos" => x.cos(),
                "tan" => x.tan(),
                "floor" => x.floor(),
                "ceil" => x.ceil(),
                "round" => x.round(),
                "sign" => x.signum(),
                _ => return None,
            })
        }
    }
}

/// If `name` is an aggregate function, return its reducer over a column's finite
/// values. An empty input reduces to a neutral value (0 for sum/count, NaN for
/// the rest, which becomes `Na`).
fn aggregate(name: &str) -> Option<fn(&[f64]) -> f64> {
    Some(match name {
        "sum" => |v: &[f64]| v.iter().sum(),
        "count" => |v: &[f64]| v.len() as f64,
        "prod" => |v: &[f64]| v.iter().product(),
        "mean" | "avg" => |v: &[f64]| {
            if v.is_empty() {
                f64::NAN
            } else {
                v.iter().sum::<f64>() / v.len() as f64
            }
        },
        "max" => |v: &[f64]| v.iter().copied().fold(f64::NAN, f64::max),
        "min" => |v: &[f64]| v.iter().copied().fold(f64::NAN, f64::min),
        "median" => |v: &[f64]| {
            if v.is_empty() {
                return f64::NAN;
            }
            let mut s = v.to_vec();
            s.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let m = s.len() / 2;
            if s.len().is_multiple_of(2) {
                (s[m - 1] + s[m]) / 2.0
            } else {
                s[m]
            }
        },
        _ => return None,
    })
}

fn references_known_column(e: &Expr, data: &DataFrame) -> bool {
    match e {
        Expr::Col(name) => data.has_column(name),
        Expr::Num(_) => false,
        Expr::Neg(a) | Expr::Func(_, a) => references_known_column(a, data),
        Expr::Bin(_, a, b) => references_known_column(a, data) || references_known_column(b, data),
    }
}

/// Evaluate `expr` over every row of `data`, producing one `Value` per row
/// (non-finite results become `Value::Na`). Returns `None` if the string is not
/// a valid expression or references no existing column (so a plain unknown
/// column name / typo is left for the caller to handle, not silently computed).
pub fn eval_expression(expr: &str, data: &DataFrame) -> Option<Vec<Value>> {
    let parsed = parse(expr)?;
    if !references_known_column(&parsed, data) {
        return None;
    }
    let n = data.nrows();
    let mut out = Vec::with_capacity(n);
    for row in 0..n {
        out.push(match eval(&parsed, data, row) {
            Some(v) if v.is_finite() => Value::Float(v),
            _ => Value::Na,
        });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn df() -> DataFrame {
        let mut d = DataFrame::new();
        d.add_column("a".into(), vec![Value::Float(2.0), Value::Float(4.0)]);
        d.add_column("b".into(), vec![Value::Float(8.0), Value::Float(2.0)]);
        d
    }

    fn f(vals: &[Value]) -> Vec<f64> {
        vals.iter().filter_map(|v| v.as_f64()).collect()
    }

    #[test]
    fn arithmetic_and_precedence() {
        let d = df();
        assert_eq!(f(&eval_expression("a / b", &d).unwrap()), vec![0.25, 2.0]);
        assert_eq!(
            f(&eval_expression("a + b * 2", &d).unwrap()),
            vec![18.0, 8.0]
        );
        assert_eq!(
            f(&eval_expression("(a + b) * 2", &d).unwrap()),
            vec![20.0, 12.0]
        );
        assert_eq!(f(&eval_expression("2 ^ a", &d).unwrap()), vec![4.0, 16.0]);
        assert_eq!(f(&eval_expression("-a", &d).unwrap()), vec![-2.0, -4.0]);
    }

    #[test]
    fn functions() {
        let d = df();
        assert_eq!(
            f(&eval_expression("sqrt(b)", &d).unwrap()),
            vec![8f64.sqrt(), 2f64.sqrt()]
        );
        assert_eq!(f(&eval_expression("log2(b)", &d).unwrap()), vec![3.0, 1.0]);
        assert_eq!(
            f(&eval_expression("abs(a - b)", &d).unwrap()),
            vec![6.0, 2.0]
        );
    }

    #[test]
    fn non_expression_or_unknown_returns_none() {
        let d = df();
        assert!(eval_expression("nonexistent_col", &d).is_none());
        assert!(eval_expression("1 + 2", &d).is_none()); // no column referenced
        assert!(eval_expression("a +", &d).is_none()); // parse error
        assert!(eval_expression("a $ b", &d).is_none()); // bad char
    }

    #[test]
    fn aggregates_broadcast_over_column() {
        let mut d = DataFrame::new();
        d.add_column(
            "count".into(),
            vec![Value::Float(1.0), Value::Float(3.0), Value::Float(4.0)],
        );
        // proportion = count / sum(count); sum = 8
        assert_eq!(
            f(&eval_expression("count / sum(count)", &d).unwrap()),
            vec![0.125, 0.375, 0.5]
        );
        // normalized = count / max(count); max = 4
        assert_eq!(
            f(&eval_expression("count / max(count)", &d).unwrap()),
            vec![0.25, 0.75, 1.0]
        );
        assert_eq!(
            f(&eval_expression("mean(count)", &d).unwrap()),
            vec![8.0 / 3.0; 3]
        );
    }

    #[test]
    fn division_by_zero_is_na() {
        let mut d = DataFrame::new();
        d.add_column("a".into(), vec![Value::Float(1.0)]);
        d.add_column("z".into(), vec![Value::Float(0.0)]);
        assert!(matches!(
            eval_expression("a / z", &d).unwrap()[0],
            Value::Na
        ));
    }
}
