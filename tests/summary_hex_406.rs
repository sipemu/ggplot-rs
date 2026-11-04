//! #406: `stat_summary_hex` matches ggplot2 4.0.3
//! (`layer_data(ggplot(h, aes(x, y, z = z)) + stat_summary_hex(fun = "mean", bins = 5))`,
//! fixture `validation/fixtures/stat_summary_hex.csv`).

use ggplot_rs::data::DataFrame;
use ggplot_rs::prelude::*;
use ggplot_rs::scale::ScaleSet;
use ggplot_rs::stat::Stat;

#[test]
fn summary_hex_matches_ggplot2() {
    let input = DataFrame::from_csv("validation/fixtures/data/summary_hex_input.csv").unwrap();
    let want = DataFrame::from_csv("validation/fixtures/stat_summary_hex.csv").unwrap();
    let got = StatSummaryHex::new(SummaryFun::Mean)
        .with_bins(5, 5)
        .compute_group(&input, &ScaleSet::new());
    let rows = |df: &DataFrame| -> Vec<(f64, f64, f64)> {
        let c = |n: &str| {
            df.column(n)
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect::<Vec<_>>()
        };
        let (x, y, v) = (c("x"), c("y"), c("value"));
        let mut r: Vec<_> = (0..x.len()).map(|i| (x[i], y[i], v[i])).collect();
        r.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.total_cmp(&b.0)));
        r
    };
    let (got, want) = (rows(&got), rows(&want));
    assert_eq!(got.len(), want.len(), "cell count");
    for (g, w) in got.iter().zip(&want) {
        for (a, b) in [(g.0, w.0), (g.1, w.1), (g.2, w.2)] {
            assert!((a - b).abs() < 1e-5, "got {g:?}, ggplot2 {w:?}");
        }
    }
}
