//! #491: `facet_wrap` + `coord_flip` — every panel's axes follow the flip,
//! as a single panel's do: categories on the left, counts along the bottom,
//! and the axis titles swap sides with them.
use ggplot_rs::prelude::*;

fn texts(svg: &str) -> Vec<(String, f64, f64, bool)> {
    svg.split("<text")
        .skip(1)
        .filter_map(|t| {
            let head = &t[..t.find('>')?];
            let body = &t[t.find('>')? + 1..t.find("</text>")?];
            let attr = |k: &str| -> Option<f64> {
                let i = head.find(&format!(" {k}=\""))? + k.len() + 3;
                head[i..i + head[i..].find('"')?].parse().ok()
            };
            Some((
                body.to_string(),
                attr("x")?,
                attr("y")?,
                head.contains("rotate("),
            ))
        })
        .collect()
}

fn plot(flip: bool) -> String {
    let g: Vec<Value> = (0..60)
        .map(|i| Value::Str(["a", "b"][i % 2].into()))
        .collect();
    let c: Vec<Value> = (0..60)
        .map(|i| Value::Str(["p", "q", "r"][i % 3].into()))
        .collect();
    let mut p = GGPlot::new(vec![("g".to_string(), g), ("cat".to_string(), c)])
        .aes(Aes::new().x("cat"))
        .geom_bar()
        .facet_wrap("g", None)
        .xlab("Category")
        .ylab("Count");
    if flip {
        p = p.coord_flip();
    }
    p.render_svg_native_with_size(600, 400).unwrap()
}

#[test]
fn flipped_facets_put_categories_left_and_counts_bottom() {
    let t = texts(&plot(true));
    let at = |s: &str| t.iter().filter(|x| x.0 == s).cloned().collect::<Vec<_>>();
    // Category ticks: one column on the left (first panel only), stacked.
    let p = at("p");
    let q = at("q");
    assert_eq!(p.len(), 1, "categories label the left column only: {p:?}");
    assert!((p[0].1 - q[0].1).abs() < 0.5, "same x: {p:?} {q:?}");
    assert!((p[0].2 - q[0].2).abs() > 10.0, "stacked: {p:?} {q:?}");
    // Count ticks along the bottom of both panels, in a row.
    let zeros = at("0");
    assert_eq!(zeros.len(), 2, "a count axis under each panel: {zeros:?}");
    assert!((zeros[0].2 - zeros[1].2).abs() < 0.5);
    let tens = at("10");
    assert!((tens[0].2 - zeros[0].2).abs() < 0.5 && tens[0].1 > zeros[0].1);
    // Titles: the x title ("Category") on the left, rotated; "Count" below.
    let cat = &at("Category")[0];
    let count = &at("Count")[0];
    assert!(cat.3, "x title is rotated on the left: {cat:?}");
    assert!(
        !count.3 && count.2 > zeros[0].2,
        "y title under the ticks: {count:?}"
    );
}

#[test]
fn unflipped_facets_unchanged() {
    let t = texts(&plot(false));
    let p: Vec<_> = t.iter().filter(|x| x.0 == "p").collect();
    assert_eq!(p.len(), 2, "categories under each panel");
    let count = t.iter().find(|x| x.0 == "Count").unwrap();
    assert!(count.3, "y title rotated on the left");
}
