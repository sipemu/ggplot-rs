//! CLDR locale presets (DataZooDE/anofox-visualization#1358): each one
//! formats numbers and months as ICU does. The expected strings come from
//! Node's `Intl` (CLDR), with (narrow) no-break spaces as plain spaces;
//! the month labels of pt-PT (ICU: 10/2026) and cs-CZ (ICU: říjen 2026)
//! deliberately use the short name.

use ggplot_rs::locale::Locale;

/// (tag, 1234567.5, 1234.5, October 2026 as month_format)
#[rustfmt::skip]
const VECTORS: &[(&str, &str, &str, &str)] = &[
    ("de-AT", "1 234 567,5", "1 234,5", "Okt. 2026"),
    ("de-CH", "1'234'567.5", "1'234.5", "Okt. 2026"),
    ("fr-BE", "1 234 567,5", "1 234,5", "oct. 2026"),
    ("fr-CH", "1 234 567,5", "1 234,5", "oct. 2026"),
    ("fr-CA", "1 234 567,5", "1 234,5", "oct. 2026"),
    ("it-IT", "1.234.567,5", "1234,5", "ott 2026"),
    ("it-CH", "1'234'567.5", "1234.5", "ott 2026"),
    ("es-ES", "1.234.567,5", "1234,5", "oct 2026"),
    ("es-MX", "1,234,567.5", "1,234.5", "oct 2026"),
    ("nl-NL", "1.234.567,5", "1.234,5", "okt 2026"),
    ("nl-BE", "1.234.567,5", "1.234,5", "okt 2026"),
    ("pt-PT", "1 234 567,5", "1234,5", "out. 2026"),
    ("pt-BR", "1.234.567,5", "1.234,5", "out. de 2026"),
    ("pl-PL", "1 234 567,5", "1234,5", "paź 2026"),
    ("sv-SE", "1 234 567,5", "1 234,5", "okt. 2026"),
    ("da-DK", "1.234.567,5", "1.234,5", "okt. 2026"),
    ("nb-NO", "1 234 567,5", "1 234,5", "okt. 2026"),
    ("fi-FI", "1 234 567,5", "1 234,5", "loka 2026"),
    ("cs-CZ", "1 234 567,5", "1 234,5", "říj 2026"),
];

#[test]
fn every_cldr_preset_formats_like_icu() {
    for &(tag, big, small, month) in VECTORS {
        let l = Locale::try_from_tag(tag).unwrap_or_else(|| panic!("no preset for {tag}"));
        assert_eq!(l.tag, tag);
        assert_eq!(l.format_number(1234567.5), big, "{tag} 1234567.5");
        assert_eq!(l.format_number(1234.5), small, "{tag} 1234.5");
        // 2026-10-07 00:00 UTC
        assert_eq!(
            l.strftime(1_791_331_200, &l.month_format),
            month,
            "{tag} month"
        );
    }
}

#[test]
fn preset_tags_lists_every_preset() {
    let tags = Locale::preset_tags();
    assert!(tags.contains(&"en-US") && tags.contains(&"de-CH") && tags.contains(&"pt-BR"));
    for t in tags {
        assert_eq!(Locale::from_tag(t).tag, t);
    }
}
