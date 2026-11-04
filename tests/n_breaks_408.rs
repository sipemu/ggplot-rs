//! #408: `ScaleContinuous::with_n_breaks` — ggplot2's `n.breaks`.

use ggplot_rs::data::Value;
use ggplot_rs::scale::continuous::ScaleContinuous;
use ggplot_rs::scale::Scale;

fn labels(s: ScaleContinuous) -> Vec<String> {
    let mut s = s;
    s.train(&[Value::Float(0.0), Value::Float(100.0)]);
    s.breaks().into_iter().map(|(_, l)| l).collect()
}

#[test]
fn n_breaks_changes_the_break_count() {
    assert_eq!(
        labels(ScaleContinuous::new()),
        ["0", "25", "50", "75", "100"]
    );
    let ten = labels(ScaleContinuous::new().with_n_breaks(10));
    assert_eq!(ten.len(), 11, "{ten:?}");
    assert_eq!(ten[1], "10");
    let two = labels(ScaleContinuous::new().with_n_breaks(2));
    assert!(two.len() <= 3, "{two:?}");
}

#[test]
fn time_scale_breaks_on_round_times() {
    // 08:00 to 12:30 — hourly breaks, HH:MM:SS labels.
    let mut s = ScaleContinuous::time();
    s.train(&[Value::Float(8.0 * 3600.0), Value::Float(12.5 * 3600.0)]);
    let l: Vec<String> = s.breaks().into_iter().map(|(_, l)| l).collect();
    assert_eq!(
        l,
        ["08:00:00", "09:00:00", "10:00:00", "11:00:00", "12:00:00"]
    );
    // 90 seconds — 30 s steps.
    let mut s = ScaleContinuous::time();
    s.train(&[Value::Float(0.0), Value::Float(90.0)]);
    let l: Vec<String> = s.breaks().into_iter().map(|(_, l)| l).collect();
    assert_eq!(l, ["00:00:00", "00:00:30", "00:01:00", "00:01:30"]);
    assert_eq!(ggplot_rs::scale::util::format_hms(-3661.0), "-01:01:01");
}
