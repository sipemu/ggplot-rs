//! Tree layouts (#311): tidy tree, org chart, dendrogram, radial.

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::geom::tree::{GeomTree, TreeLink};
use ggplot_rs::stat::tree::{dendrogram_links, TreeOrientation};
use ggplot_rs::GGPlot;

fn org() -> Vec<(String, Vec<Value>)> {
    let s = |v: &[&str]| v.iter().map(|x| Value::Str(x.to_string())).collect();
    vec![
        ("boss".into(), s(&["CEO", "CEO", "CTO", "CTO"])),
        ("report".into(), s(&["CTO", "CFO", "Dev", "Ops"])),
    ]
}

#[test]
fn org_chart_boxes_and_elbows() {
    let (svg, warnings) = GGPlot::new(org())
        .aes(Aes::new().from("boss").to("report"))
        .geom_tree_with(GeomTree::org_chart())
        .render_svg_native_with_warnings(500, 400)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(svg.contains("<title>CEO → CTO</title>"));
    for n in ["CEO", "CTO", "CFO", "Dev", "Ops"] {
        assert!(svg.contains(&format!(">{n}<")), "{n}");
    }
    // 5 boxes (rounded rects) and 4 elbow links of 4 points.
    assert_eq!(svg.matches("<title>CTO → Dev</title>").count(), 1);
    assert!(!svg.contains(">0.0<") && !svg.contains("NaN"));
}

#[test]
fn every_orientation_and_link_style_renders() {
    for orientation in [
        TreeOrientation::TopDown,
        TreeOrientation::LeftRight,
        TreeOrientation::Radial,
    ] {
        for link in [
            TreeLink::Elbow,
            TreeLink::Step,
            TreeLink::Diagonal,
            TreeLink::Straight,
        ] {
            let svg = GGPlot::new(org())
                .aes(Aes::new().from("boss").to("report"))
                .geom_tree_with(GeomTree {
                    link,
                    ..GeomTree::default().oriented(orientation)
                })
                .render_svg_native_with_size(400, 400)
                .unwrap();
            assert_eq!(
                svg.matches("<title>").count(),
                4 + 5,
                "{orientation:?} {link:?}"
            );
            assert!(!svg.contains("NaN"));
        }
    }
}

#[test]
fn dendrogram_labels_leaves_only() {
    let links = dendrogram_links(&[(-1, -2), (-3, 1)], &[1.0, 2.0], &["a", "b", "c"]);
    let svg = GGPlot::new(links)
        .aes(Aes::new().from("from").to("to"))
        .geom_dendrogram()
        .render_svg_native_with_size(400, 300)
        .unwrap();
    for n in ["a", "b", "c"] {
        assert!(svg.contains(&format!(">{n}<")));
    }
    assert!(
        !svg.contains(">#1<") && !svg.contains(">#2<"),
        "clusters unlabelled"
    );
}
