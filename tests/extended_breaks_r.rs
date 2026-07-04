//! Extended-Wilkinson breaks match R's scales::extended_breaks() (verified vs R).
use ggplot_rs::scale::util::extended_breaks;

fn approx(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9)
}

#[test]
fn matches_r_extended_breaks() {
    let cases: &[((f64, f64), &[f64])] = &[
        ((0.0, 10.0), &[0.0, 2.5, 5.0, 7.5, 10.0]),
        ((0.0, 1.0), &[0.0, 0.25, 0.5, 0.75, 1.0]),
        ((-3.0, 3.0), &[-3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0]),
        ((0.0, 97.0), &[0.0, 25.0, 50.0, 75.0, 100.0]),
        ((1.0, 50.0), &[0.0, 10.0, 20.0, 30.0, 40.0, 50.0]),
        ((0.0, 0.045), &[0.0, 0.01, 0.02, 0.03, 0.04]),
    ];
    for &((a, b), expected) in cases {
        let got = extended_breaks(a, b, 5);
        assert!(
            approx(&got, expected),
            "[{a},{b}] -> {got:?}, want {expected:?}"
        );
    }
}

#[test]
fn degenerate_inputs_are_safe() {
    assert!(extended_breaks(5.0, 5.0, 5).is_empty());
    assert!(extended_breaks(f64::NAN, 1.0, 5).is_empty());
}
