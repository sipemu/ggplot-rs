//! `annotation_attribution()`'s optional link URL / linked segments
//! (feature `sf`, #196): the SVG backend wraps a linked run in
//! `<a href target="_blank" rel="noopener">` with an underline; a bad
//! (non-http(s)) URL is dropped as a warning, not drawn as a link.
#![cfg(feature = "sf")]

use ggplot_rs::data::{DataFrame, Value};
use ggplot_rs::geom::annotation_scale::GeomAttribution;
use ggplot_rs::prelude::*;

fn df() -> DataFrame {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        vec![Value::Str("POLYGON ((0 0, 1 0, 1 1, 0 1, 0 0))".into())],
    );
    df
}

#[test]
fn plain_text_renders_as_before_with_no_link() {
    let svg = GGPlot::new(df())
        .aes(Aes::new())
        .geom_sf()
        .annotation_attribution("Made with Natural Earth")
        .render_svg_native_with_size(300, 300)
        .unwrap();
    assert!(svg.contains("Made with Natural Earth"));
    assert!(!svg.contains("<a "), "{svg}");
}

#[test]
fn link_wraps_the_whole_text_in_an_anchor() {
    let geom =
        GeomAttribution::new("Made with Natural Earth").link("https://www.naturalearthdata.com");
    let svg = GGPlot::new(df())
        .aes(Aes::new())
        .geom_sf()
        .annotation_attribution_with(geom)
        .render_svg_native_with_size(300, 300)
        .unwrap();
    assert!(
        svg.contains(
            "<a href=\"https://www.naturalearthdata.com\" target=\"_blank\" rel=\"noopener\">"
        ),
        "{svg}"
    );
    assert!(svg.contains("text-decoration=\"underline\""), "{svg}");
    assert!(svg.contains("Made with Natural Earth"), "{svg}");
}

#[test]
fn a_newline_in_the_link_url_is_entity_escaped_in_the_href_attribute() {
    // #226: the `href` is a quoted attribute value — a literal `\n` inside
    // it would be collapsed to a space by XML attribute-value
    // normalisation, silently changing the URL. Still passes the
    // http(s)-prefix check, so it's a real (if contrived) path here.
    let geom = GeomAttribution::new("note").link("https://example.com/a\nb");
    let svg = GGPlot::new(df())
        .aes(Aes::new())
        .geom_sf()
        .annotation_attribution_with(geom)
        .render_svg_native_with_size(300, 300)
        .unwrap();
    assert!(
        svg.contains("href=\"https://example.com/a&#10;b\""),
        "{svg}"
    );
    assert!(!svg.contains('\n'), "{svg}");
}

#[test]
fn runs_mix_linked_and_plain_segments_in_one_text_element() {
    let geom = GeomAttribution::new("").runs(vec![
        ("© ", None),
        ("BKG", Some("https://www.bkg.bund.de")),
        (" (2024) ", None),
        ("dl-de/by-2-0", Some("https://www.govdata.de/dl-de/by-2-0")),
    ]);
    let svg = GGPlot::new(df())
        .aes(Aes::new())
        .geom_sf()
        .annotation_attribution_with(geom)
        .render_svg_native_with_size(300, 300)
        .unwrap();
    // Exactly one <text data-geom="attribution"> element carries the whole
    // line (so text-anchor aligns the combined width, not each run
    // separately) — axis tick labels are also plain <text>, so this must
    // be scoped to the attribution's own mark.
    assert_eq!(svg.matches("data-geom=\"attribution\"").count(), 1, "{svg}");
    assert!(svg.contains("<a href=\"https://www.bkg.bund.de\""), "{svg}");
    assert!(
        svg.contains("<a href=\"https://www.govdata.de/dl-de/by-2-0\""),
        "{svg}"
    );
    assert!(svg.contains("BKG"));
    assert!(svg.contains("dl-de/by-2-0"));
    assert!(svg.contains("(2024)"));
}

#[test]
fn a_non_http_link_is_dropped_and_warned_not_drawn() {
    let geom = GeomAttribution::new("click me").link("javascript:alert(1)");
    let svg = GGPlot::new(df())
        .aes(Aes::new())
        .geom_sf()
        .annotation_attribution_with(geom)
        .render_svg_native_with_size(300, 300)
        .unwrap();
    assert!(!svg.contains("javascript:"), "{svg}");
    assert!(!svg.contains("<a "), "{svg}");
    assert!(svg.contains("click me"), "{svg}");
}

#[test]
fn other_backends_draw_plain_text_dropping_the_link() {
    // The default `draw_text_runs` impl (pixel/`canvas`, `plotters`
    // backends) concatenates the runs and drops every link — exercised
    // here through the default trait impl directly, since plain-text
    // output is otherwise invisible to assert on.
    use ggplot_rs::render::backend::{
        DrawBackend, FontFace, PointStyle, RectStyle, TextAnchor, TextStyle,
    };
    use ggplot_rs::render::{Rect, RenderError};

    struct Recorder(Vec<String>);
    impl DrawBackend for Recorder {
        fn draw_circle(
            &mut self,
            _: (f64, f64),
            _: f64,
            _: &PointStyle,
        ) -> Result<(), RenderError> {
            Ok(())
        }
        fn draw_line(
            &mut self,
            _: &[(f64, f64)],
            _: &ggplot_rs::render::backend::LineStyle,
        ) -> Result<(), RenderError> {
            Ok(())
        }
        fn draw_rect(
            &mut self,
            _: (f64, f64),
            _: (f64, f64),
            _: &RectStyle,
        ) -> Result<(), RenderError> {
            Ok(())
        }
        fn draw_text(
            &mut self,
            text: &str,
            _: (f64, f64),
            _: &TextStyle,
        ) -> Result<(), RenderError> {
            self.0.push(text.to_string());
            Ok(())
        }
        fn draw_polygon(&mut self, _: &[(f64, f64)], _: &RectStyle) -> Result<(), RenderError> {
            Ok(())
        }
        fn plot_area(&self) -> Rect {
            Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            }
        }
        fn total_area(&self) -> Rect {
            self.plot_area()
        }
    }

    let mut backend = Recorder(Vec::new());
    backend
        .draw_text_runs(
            &[("© ", None), ("BKG", Some("https://www.bkg.bund.de"))],
            (0.0, 0.0),
            &TextStyle {
                color: (0, 0, 0),
                size: 8.0,
                anchor: TextAnchor::Start,
                angle: 0.0,
                family: None,
                face: FontFace::Plain,
                bg_colour: None,
                bg_r: 0.0,
            },
        )
        .unwrap();
    assert_eq!(backend.0, vec!["© BKG".to_string()]);
}
