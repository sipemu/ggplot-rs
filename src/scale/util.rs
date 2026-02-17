/// Round step size to a "nice" value (1, 2, 5, 10, 20, 50, ...).
pub fn nice_step(raw: f64) -> f64 {
    let magnitude = 10f64.powf(raw.abs().log10().floor());
    let fraction = raw / magnitude;

    let nice = if fraction <= 1.5 {
        1.0
    } else if fraction <= 3.5 {
        2.0
    } else if fraction <= 7.5 {
        5.0
    } else {
        10.0
    };

    nice * magnitude
}

/// Format a number nicely, removing trailing zeros.
pub fn format_number(v: f64) -> String {
    if v == v.round() && v.abs() < 1e10 {
        format!("{}", v as i64)
    } else {
        let s = format!("{:.2}", v);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}
