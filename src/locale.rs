//! Locale conventions for number and date labels.
//!
//! A [`Locale`] bundles the decimal / thousands marks, the currency and
//! percent conventions and the month / weekday names of a region. It is used
//!
//! - explicitly, by the `label_*_locale` formatters in
//!   [`crate::scale::format`] (e.g. [`label_currency_locale`]);
//! - plot-wide, via [`GGPlot::locale`](crate::GGPlot::locale): every default
//!   continuous axis and legend then labels its breaks with the locale's
//!   marks, and date/datetime scales use its month names and date formats;
//! - per scale, via `ScaleContinuous::with_locale` /
//!   `ScaleDateTime::with_locale` (overrides the plot locale).
//!
//! Without a locale nothing changes: labels keep the `.` decimal mark, no
//! thousands grouping on default axes, and ISO dates.
//!
//! ```
//! use ggplot_rs::locale::Locale;
//! use ggplot_rs::scale::format::label_currency_locale;
//! let de = Locale::from_tag("de-DE");
//! assert_eq!(label_currency_locale(&de, Some(0.01))(1234.5), "1.234,50 €");
//! let us = Locale::en_us();
//! assert_eq!(label_currency_locale(&us, Some(0.01))(1234.5), "$1,234.50");
//! assert_eq!(de.strftime(1_792_022_400, "%-d. %b %Y"), "15. Okt. 2026");
//! ```
//!
//! [`label_currency_locale`]: crate::scale::format::label_currency_locale

/// Where a locale puts the currency symbol relative to the amount.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurrencyPosition {
    /// Symbol before the amount (`$1,234.50`, `£5`).
    Prefix,
    /// Symbol after the amount (`1.234,50 €`).
    Suffix,
}

/// Number and date formatting conventions of a locale.
///
/// Construct one with a preset ([`en_us`](Self::en_us), [`de_de`](Self::de_de),
/// [`fr_fr`](Self::fr_fr), [`en_gb`](Self::en_gb)) or [`from_tag`](Self::from_tag),
/// then adjust the public fields or use the `with_*` builders. The struct is
/// `#[non_exhaustive]` so new conventions can be added without a breaking
/// change.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Locale {
    /// Canonical BCP-47 tag of the preset, e.g. `"de-DE"`.
    pub tag: String,
    /// Decimal mark (`"."` / `","`).
    pub decimal_mark: String,
    /// Thousands separator (`","`, `"."`, `" "`, `"'"`; `""` disables grouping).
    pub big_mark: String,
    /// Group thousands only when the integer part has at least
    /// `3 + min_grouping_digits` digits (CLDR's minimum grouping digits).
    /// `1` (every preset) groups `1000` as `1.000`; `2` leaves `2020` alone
    /// and groups from `10.000`.
    pub min_grouping_digits: usize,
    /// The locale's currency symbol (`"$"`, `"€"`, `"£"`).
    pub currency_symbol: String,
    /// Where the currency symbol goes.
    pub currency_position: CurrencyPosition,
    /// Text between amount and currency symbol (`""` or `" "`).
    pub currency_spacing: String,
    /// Suffix of percentages (`"%"` in English, `" %"` in German / French).
    pub percent_suffix: String,
    /// Abbreviated month names, January first (`%b`).
    pub months_short: [String; 12],
    /// Full month names, January first (`%B`).
    pub months_long: [String; 12],
    /// Abbreviated weekday names, Sunday first (`%a`).
    pub days_short: [String; 7],
    /// Full weekday names, Sunday first (`%A`).
    pub days_long: [String; 7],
    /// strftime format of a calendar date (default date-axis labels at day
    /// resolution), e.g. `"%-d. %b %Y"` → `15. Okt. 2026`.
    pub date_format: String,
    /// strftime format of a month (default monthly date-axis labels), e.g.
    /// `"%b %Y"` → `Okt. 2026`.
    pub month_format: String,
    /// strftime format of a time of day, appended to the date for datetimes
    /// that are not at midnight, e.g. `"%H:%M"`.
    pub time_format: String,
}

/// CLDR-derived presets beyond the hand-written four (#538 follow-up,
/// DataZooDE/anofox-visualization#1358): marks, currency placement, month
/// and weekday names and date patterns as ICU formats them. Thousands marks
/// that CLDR writes as a (narrow) no-break space are a plain space, as in
/// [`Locale::fr_fr`].
struct Preset {
    tag: &'static str,
    decimal: &'static str,
    big: &'static str,
    currency: &'static str,
    spacing: &'static str,
    percent: &'static str,
    date: &'static str,
    month: &'static str,
    min_grouping: usize,
    suffix: bool,
    months_short: [&'static str; 12],
    months_long: [&'static str; 12],
    days_short: [&'static str; 7],
    days_long: [&'static str; 7],
}

#[rustfmt::skip]
const PRESETS: &[Preset] = &[
    Preset {
        tag: "de-AT", decimal: ",", big: " ", min_grouping: 1,
        currency: "€", suffix: false, spacing: " ", percent: " %",
        months_short: ["Jän.", "Feb.", "März", "Apr.", "Mai", "Juni", "Juli", "Aug.", "Sep.", "Okt.", "Nov.", "Dez."],
        months_long: ["Jänner", "Februar", "März", "April", "Mai", "Juni", "Juli", "August", "September", "Oktober", "November", "Dezember"],
        days_short: ["So.", "Mo.", "Di.", "Mi.", "Do.", "Fr.", "Sa."],
        days_long: ["Sonntag", "Montag", "Dienstag", "Mittwoch", "Donnerstag", "Freitag", "Samstag"],
        date: "%-d. %b %Y", month: "%b %Y",
    },
    Preset {
        tag: "de-CH", decimal: ".", big: "'", min_grouping: 1,
        currency: "CHF", suffix: false, spacing: " ", percent: "%",
        months_short: ["Jan.", "Feb.", "März", "Apr.", "Mai", "Juni", "Juli", "Aug.", "Sept.", "Okt.", "Nov.", "Dez."],
        months_long: ["Januar", "Februar", "März", "April", "Mai", "Juni", "Juli", "August", "September", "Oktober", "November", "Dezember"],
        days_short: ["So.", "Mo.", "Di.", "Mi.", "Do.", "Fr.", "Sa."],
        days_long: ["Sonntag", "Montag", "Dienstag", "Mittwoch", "Donnerstag", "Freitag", "Samstag"],
        date: "%-d. %b %Y", month: "%b %Y",
    },
    Preset {
        tag: "fr-BE", decimal: ",", big: " ", min_grouping: 1,
        currency: "€", suffix: true, spacing: " ", percent: " %",
        months_short: ["janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc."],
        months_long: ["janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"],
        days_short: ["dim.", "lun.", "mar.", "mer.", "jeu.", "ven.", "sam."],
        days_long: ["dimanche", "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi"],
        date: "%-d %b %Y", month: "%b %Y",
    },
    Preset {
        tag: "fr-CH", decimal: ",", big: " ", min_grouping: 1,
        currency: "CHF", suffix: true, spacing: " ", percent: "%",
        months_short: ["janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc."],
        months_long: ["janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"],
        days_short: ["dim.", "lun.", "mar.", "mer.", "jeu.", "ven.", "sam."],
        days_long: ["dimanche", "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi"],
        date: "%-d %b %Y", month: "%b %Y",
    },
    Preset {
        tag: "fr-CA", decimal: ",", big: " ", min_grouping: 1,
        currency: "$", suffix: true, spacing: " ", percent: " %",
        months_short: ["janv.", "févr.", "mars", "avr.", "mai", "juin", "juill.", "août", "sept.", "oct.", "nov.", "déc."],
        months_long: ["janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"],
        days_short: ["dim.", "lun.", "mar.", "mer.", "jeu.", "ven.", "sam."],
        days_long: ["dimanche", "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi"],
        date: "%-d %b %Y", month: "%b %Y",
    },
    Preset {
        tag: "it-IT", decimal: ",", big: ".", min_grouping: 2,
        currency: "€", suffix: true, spacing: " ", percent: "%",
        months_short: ["gen", "feb", "mar", "apr", "mag", "giu", "lug", "ago", "set", "ott", "nov", "dic"],
        months_long: ["gennaio", "febbraio", "marzo", "aprile", "maggio", "giugno", "luglio", "agosto", "settembre", "ottobre", "novembre", "dicembre"],
        days_short: ["dom", "lun", "mar", "mer", "gio", "ven", "sab"],
        days_long: ["domenica", "lunedì", "martedì", "mercoledì", "giovedì", "venerdì", "sabato"],
        date: "%-d %b %Y", month: "%b %Y",
    },
    Preset {
        tag: "it-CH", decimal: ".", big: "'", min_grouping: 2,
        currency: "CHF", suffix: false, spacing: " ", percent: "%",
        months_short: ["gen", "feb", "mar", "apr", "mag", "giu", "lug", "ago", "set", "ott", "nov", "dic"],
        months_long: ["gennaio", "febbraio", "marzo", "aprile", "maggio", "giugno", "luglio", "agosto", "settembre", "ottobre", "novembre", "dicembre"],
        days_short: ["dom", "lun", "mar", "mer", "gio", "ven", "sab"],
        days_long: ["domenica", "lunedì", "martedì", "mercoledì", "giovedì", "venerdì", "sabato"],
        date: "%-d %b %Y", month: "%b %Y",
    },
    Preset {
        tag: "es-ES", decimal: ",", big: ".", min_grouping: 2,
        currency: "€", suffix: true, spacing: " ", percent: " %",
        months_short: ["ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sept", "oct", "nov", "dic"],
        months_long: ["enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre", "noviembre", "diciembre"],
        days_short: ["dom", "lun", "mar", "mié", "jue", "vie", "sáb"],
        days_long: ["domingo", "lunes", "martes", "miércoles", "jueves", "viernes", "sábado"],
        date: "%-d %b %Y", month: "%b %Y",
    },
    Preset {
        tag: "es-MX", decimal: ".", big: ",", min_grouping: 1,
        currency: "$", suffix: false, spacing: "", percent: "%",
        months_short: ["ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic"],
        months_long: ["enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre", "noviembre", "diciembre"],
        days_short: ["dom", "lun", "mar", "mié", "jue", "vie", "sáb"],
        days_long: ["domingo", "lunes", "martes", "miércoles", "jueves", "viernes", "sábado"],
        date: "%-d %b %Y", month: "%b %Y",
    },
    Preset {
        tag: "nl-NL", decimal: ",", big: ".", min_grouping: 1,
        currency: "€", suffix: false, spacing: " ", percent: "%",
        months_short: ["jan", "feb", "mrt", "apr", "mei", "jun", "jul", "aug", "sep", "okt", "nov", "dec"],
        months_long: ["januari", "februari", "maart", "april", "mei", "juni", "juli", "augustus", "september", "oktober", "november", "december"],
        days_short: ["zo", "ma", "di", "wo", "do", "vr", "za"],
        days_long: ["zondag", "maandag", "dinsdag", "woensdag", "donderdag", "vrijdag", "zaterdag"],
        date: "%-d %b %Y", month: "%b %Y",
    },
    Preset {
        tag: "nl-BE", decimal: ",", big: ".", min_grouping: 1,
        currency: "€", suffix: false, spacing: " ", percent: "%",
        months_short: ["jan", "feb", "mrt", "apr", "mei", "jun", "jul", "aug", "sep", "okt", "nov", "dec"],
        months_long: ["januari", "februari", "maart", "april", "mei", "juni", "juli", "augustus", "september", "oktober", "november", "december"],
        days_short: ["zo", "ma", "di", "wo", "do", "vr", "za"],
        days_long: ["zondag", "maandag", "dinsdag", "woensdag", "donderdag", "vrijdag", "zaterdag"],
        date: "%-d %b %Y", month: "%b %Y",
    },
    Preset {
        tag: "pt-PT", decimal: ",", big: " ", min_grouping: 2,
        currency: "€", suffix: true, spacing: " ", percent: "%",
        months_short: ["jan.", "fev.", "mar.", "abr.", "mai.", "jun.", "jul.", "ago.", "set.", "out.", "nov.", "dez."],
        months_long: ["janeiro", "fevereiro", "março", "abril", "maio", "junho", "julho", "agosto", "setembro", "outubro", "novembro", "dezembro"],
        days_short: ["domingo", "segunda", "terça", "quarta", "quinta", "sexta", "sábado"],
        days_long: ["domingo", "segunda-feira", "terça-feira", "quarta-feira", "quinta-feira", "sexta-feira", "sábado"],
        date: "%-d/%-m/%Y", month: "%b %Y",
    },
    Preset {
        tag: "pt-BR", decimal: ",", big: ".", min_grouping: 1,
        currency: "R$", suffix: false, spacing: " ", percent: "%",
        months_short: ["jan.", "fev.", "mar.", "abr.", "mai.", "jun.", "jul.", "ago.", "set.", "out.", "nov.", "dez."],
        months_long: ["janeiro", "fevereiro", "março", "abril", "maio", "junho", "julho", "agosto", "setembro", "outubro", "novembro", "dezembro"],
        days_short: ["dom.", "seg.", "ter.", "qua.", "qui.", "sex.", "sáb."],
        days_long: ["domingo", "segunda-feira", "terça-feira", "quarta-feira", "quinta-feira", "sexta-feira", "sábado"],
        date: "%-d de %b de %Y", month: "%b de %Y",
    },
    Preset {
        tag: "pl-PL", decimal: ",", big: " ", min_grouping: 2,
        currency: "zł", suffix: true, spacing: " ", percent: "%",
        months_short: ["sty", "lut", "mar", "kwi", "maj", "cze", "lip", "sie", "wrz", "paź", "lis", "gru"],
        months_long: ["stycznia", "lutego", "marca", "kwietnia", "maja", "czerwca", "lipca", "sierpnia", "września", "października", "listopada", "grudnia"],
        days_short: ["niedz.", "pon.", "wt.", "śr.", "czw.", "pt.", "sob."],
        days_long: ["niedziela", "poniedziałek", "wtorek", "środa", "czwartek", "piątek", "sobota"],
        date: "%-d %b %Y", month: "%b %Y",
    },
    Preset {
        tag: "sv-SE", decimal: ",", big: " ", min_grouping: 1,
        currency: "kr", suffix: true, spacing: " ", percent: " %",
        months_short: ["jan.", "feb.", "mars", "apr.", "maj", "juni", "juli", "aug.", "sep.", "okt.", "nov.", "dec."],
        months_long: ["januari", "februari", "mars", "april", "maj", "juni", "juli", "augusti", "september", "oktober", "november", "december"],
        days_short: ["sön", "mån", "tis", "ons", "tors", "fre", "lör"],
        days_long: ["söndag", "måndag", "tisdag", "onsdag", "torsdag", "fredag", "lördag"],
        date: "%-d %b %Y", month: "%b %Y",
    },
    Preset {
        tag: "da-DK", decimal: ",", big: ".", min_grouping: 1,
        currency: "kr.", suffix: true, spacing: " ", percent: " %",
        months_short: ["jan.", "feb.", "mar.", "apr.", "maj", "jun.", "jul.", "aug.", "sep.", "okt.", "nov.", "dec."],
        months_long: ["januar", "februar", "marts", "april", "maj", "juni", "juli", "august", "september", "oktober", "november", "december"],
        days_short: ["søn.", "man.", "tirs.", "ons.", "tors.", "fre.", "lør."],
        days_long: ["søndag", "mandag", "tirsdag", "onsdag", "torsdag", "fredag", "lørdag"],
        date: "%-d. %b %Y", month: "%b %Y",
    },
    Preset {
        tag: "nb-NO", decimal: ",", big: " ", min_grouping: 1,
        currency: "kr", suffix: true, spacing: " ", percent: " %",
        months_short: ["jan.", "feb.", "mars", "apr.", "mai", "juni", "juli", "aug.", "sep.", "okt.", "nov.", "des."],
        months_long: ["januar", "februar", "mars", "april", "mai", "juni", "juli", "august", "september", "oktober", "november", "desember"],
        days_short: ["søn.", "man.", "tir.", "ons.", "tor.", "fre.", "lør."],
        days_long: ["søndag", "mandag", "tirsdag", "onsdag", "torsdag", "fredag", "lørdag"],
        date: "%-d. %b %Y", month: "%b %Y",
    },
    Preset {
        tag: "fi-FI", decimal: ",", big: " ", min_grouping: 1,
        currency: "€", suffix: true, spacing: " ", percent: " %",
        months_short: ["tammi", "helmi", "maalis", "huhti", "touko", "kesä", "heinä", "elo", "syys", "loka", "marras", "joulu"],
        months_long: ["tammikuuta", "helmikuuta", "maaliskuuta", "huhtikuuta", "toukokuuta", "kesäkuuta", "heinäkuuta", "elokuuta", "syyskuuta", "lokakuuta", "marraskuuta", "joulukuuta"],
        days_short: ["su", "ma", "ti", "ke", "to", "pe", "la"],
        days_long: ["sunnuntai", "maanantai", "tiistai", "keskiviikko", "torstai", "perjantai", "lauantai"],
        date: "%-d.%-m.%Y", month: "%b %Y",
    },
    Preset {
        tag: "cs-CZ", decimal: ",", big: " ", min_grouping: 1,
        currency: "Kč", suffix: true, spacing: " ", percent: " %",
        months_short: ["led", "úno", "bře", "dub", "kvě", "čvn", "čvc", "srp", "zář", "říj", "lis", "pro"],
        months_long: ["ledna", "února", "března", "dubna", "května", "června", "července", "srpna", "září", "října", "listopadu", "prosince"],
        days_short: ["ne", "po", "út", "st", "čt", "pá", "so"],
        days_long: ["neděle", "pondělí", "úterý", "středa", "čtvrtek", "pátek", "sobota"],
        date: "%-d. %-m. %Y", month: "%b %Y",
    },
];

fn strings<const N: usize>(names: [&str; N]) -> [String; N] {
    names.map(str::to_string)
}

const EN_MONTHS_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const EN_MONTHS_LONG: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const EN_DAYS_SHORT: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const EN_DAYS_LONG: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

impl Locale {
    /// English (United States): `1,234.5`, `$1,234.50`, `50%`, `Oct 15, 2026`.
    pub fn en_us() -> Self {
        Locale {
            tag: "en-US".into(),
            decimal_mark: ".".into(),
            big_mark: ",".into(),
            min_grouping_digits: 1,
            currency_symbol: "$".into(),
            currency_position: CurrencyPosition::Prefix,
            currency_spacing: String::new(),
            percent_suffix: "%".into(),
            months_short: strings(EN_MONTHS_SHORT),
            months_long: strings(EN_MONTHS_LONG),
            days_short: strings(EN_DAYS_SHORT),
            days_long: strings(EN_DAYS_LONG),
            date_format: "%b %-d, %Y".into(),
            month_format: "%b %Y".into(),
            time_format: "%H:%M".into(),
        }
    }

    /// English (United Kingdom): `1,234.5`, `£1,234.50`, `50%`, `15 Oct 2026`.
    pub fn en_gb() -> Self {
        Locale {
            tag: "en-GB".into(),
            currency_symbol: "£".into(),
            date_format: "%-d %b %Y".into(),
            ..Self::en_us()
        }
    }

    /// German (Germany): `1.234,5`, `1.234,50 €`, `50 %`, `15. Okt. 2026`.
    pub fn de_de() -> Self {
        Locale {
            tag: "de-DE".into(),
            decimal_mark: ",".into(),
            big_mark: ".".into(),
            min_grouping_digits: 1,
            currency_symbol: "€".into(),
            currency_position: CurrencyPosition::Suffix,
            currency_spacing: " ".into(),
            percent_suffix: " %".into(),
            months_short: strings([
                "Jan.", "Feb.", "März", "Apr.", "Mai", "Juni", "Juli", "Aug.", "Sept.", "Okt.",
                "Nov.", "Dez.",
            ]),
            months_long: strings([
                "Januar",
                "Februar",
                "März",
                "April",
                "Mai",
                "Juni",
                "Juli",
                "August",
                "September",
                "Oktober",
                "November",
                "Dezember",
            ]),
            days_short: strings(["So.", "Mo.", "Di.", "Mi.", "Do.", "Fr.", "Sa."]),
            days_long: strings([
                "Sonntag",
                "Montag",
                "Dienstag",
                "Mittwoch",
                "Donnerstag",
                "Freitag",
                "Samstag",
            ]),
            date_format: "%-d. %b %Y".into(),
            month_format: "%b %Y".into(),
            time_format: "%H:%M".into(),
        }
    }

    /// French (France): `1 234,5`, `1 234,50 €`, `50 %`, `15 oct. 2026`.
    /// The thousands mark is a plain space (CLDR uses a narrow no-break space,
    /// which many fonts lack); set [`big_mark`](Self::big_mark) to override.
    pub fn fr_fr() -> Self {
        Locale {
            tag: "fr-FR".into(),
            decimal_mark: ",".into(),
            big_mark: " ".into(),
            min_grouping_digits: 1,
            currency_symbol: "€".into(),
            currency_position: CurrencyPosition::Suffix,
            currency_spacing: " ".into(),
            percent_suffix: " %".into(),
            months_short: strings([
                "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.",
                "nov.", "déc.",
            ]),
            months_long: strings([
                "janvier",
                "février",
                "mars",
                "avril",
                "mai",
                "juin",
                "juillet",
                "août",
                "septembre",
                "octobre",
                "novembre",
                "décembre",
            ]),
            days_short: strings(["dim.", "lun.", "mar.", "mer.", "jeu.", "ven.", "sam."]),
            days_long: strings([
                "dimanche", "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi",
            ]),
            date_format: "%-d %b %Y".into(),
            month_format: "%b %Y".into(),
            time_format: "%H:%M".into(),
        }
    }

    /// The preset for a BCP-47-ish tag, if one matches: case-insensitive,
    /// `-` or `_` separated, POSIX suffixes (`.UTF-8`, `@euro`) ignored.
    /// A CLDR preset for the exact language and region first (`de-AT`,
    /// `de-CH`, `fr-CA`, `it-IT`, `es-MX`, `pt-BR`, … — [`preset_tags`](Self::preset_tags)),
    /// then the language's main locale (`it` → it-IT, `pt` → pt-BR, `nb`/`no`
    /// → nb-NO), then `de*` → [`de_de`](Self::de_de), `fr*` →
    /// [`fr_fr`](Self::fr_fr), `en-GB` / `en-UK` → [`en_gb`](Self::en_gb),
    /// other `en*` and `C` / `POSIX` → [`en_us`](Self::en_us).
    pub fn try_from_tag(tag: &str) -> Option<Self> {
        let t = tag
            .trim()
            .split(['.', '@'])
            .next()
            .unwrap_or("")
            .replace('_', "-")
            .to_ascii_lowercase();
        let mut parts = t.split('-');
        let lang = parts.next().unwrap_or("");
        // Skip a script subtag (e.g. "Latn") to find the region.
        let region = parts.find(|p| p.len() != 4).unwrap_or("");
        // a CLDR preset for exactly this language and region (de-AT, pt-BR, …)
        let exact = PRESETS.iter().find(|p| {
            let (l, r) = p.tag.split_once('-').unwrap_or((p.tag, ""));
            l.eq_ignore_ascii_case(lang) && r.eq_ignore_ascii_case(region)
        });
        if let Some(p) = exact {
            return Some(Self::from_preset(p));
        }
        // else the language's main locale (CLDR's default region)
        let main = match lang {
            "it" => "it-IT",
            "es" => "es-ES",
            "nl" => "nl-NL",
            "pt" => "pt-BR",
            "pl" => "pl-PL",
            "sv" => "sv-SE",
            "da" => "da-DK",
            "nb" | "no" | "nn" => "nb-NO",
            "fi" => "fi-FI",
            "cs" => "cs-CZ",
            _ => "",
        };
        if let Some(p) = PRESETS.iter().find(|p| p.tag == main) {
            return Some(Self::from_preset(p));
        }
        match (lang, region) {
            ("de", _) => Some(Self::de_de()),
            ("fr", _) => Some(Self::fr_fr()),
            ("en", "gb" | "uk") => Some(Self::en_gb()),
            ("en", _) | ("c", "") | ("posix", "") => Some(Self::en_us()),
            _ => None,
        }
    }

    /// Every tag [`try_from_tag`](Self::try_from_tag) has a preset for.
    pub fn preset_tags() -> Vec<&'static str> {
        ["en-US", "en-GB", "de-DE", "fr-FR"]
            .into_iter()
            .chain(PRESETS.iter().map(|p| p.tag))
            .collect()
    }

    fn from_preset(p: &Preset) -> Self {
        Locale {
            tag: p.tag.into(),
            decimal_mark: p.decimal.into(),
            big_mark: p.big.into(),
            min_grouping_digits: p.min_grouping,
            currency_symbol: p.currency.into(),
            currency_position: if p.suffix {
                CurrencyPosition::Suffix
            } else {
                CurrencyPosition::Prefix
            },
            currency_spacing: p.spacing.into(),
            percent_suffix: p.percent.into(),
            months_short: strings(p.months_short),
            months_long: strings(p.months_long),
            days_short: strings(p.days_short),
            days_long: strings(p.days_long),
            date_format: p.date.into(),
            month_format: p.month.into(),
            time_format: "%H:%M".into(),
        }
    }

    /// Like [`try_from_tag`](Self::try_from_tag), falling back to
    /// [`en_us`](Self::en_us) for unknown tags.
    ///
    /// ```
    /// use ggplot_rs::locale::Locale;
    /// assert_eq!(Locale::from_tag("de-DE").tag, "de-DE");
    /// assert_eq!(Locale::from_tag("fr_FR.UTF-8").tag, "fr-FR");
    /// assert_eq!(Locale::from_tag("en-GB").tag, "en-GB");
    /// assert_eq!(Locale::from_tag("xx-YY").tag, "en-US");
    /// ```
    pub fn from_tag(tag: &str) -> Self {
        Self::try_from_tag(tag).unwrap_or_else(Self::en_us)
    }

    /// Replace the decimal mark.
    pub fn with_decimal_mark(mut self, mark: &str) -> Self {
        self.decimal_mark = mark.to_string();
        self
    }

    /// Replace the thousands mark (`""` disables grouping).
    pub fn with_big_mark(mut self, mark: &str) -> Self {
        self.big_mark = mark.to_string();
        self
    }

    /// Replace the minimum grouping digits (see
    /// [`min_grouping_digits`](Self::min_grouping_digits)).
    pub fn with_min_grouping_digits(mut self, digits: usize) -> Self {
        self.min_grouping_digits = digits;
        self
    }

    /// Use another currency symbol, placed by this locale's convention:
    /// `Locale::de_de().with_currency("CHF")` formats `1.234,50 CHF`.
    pub fn with_currency(mut self, symbol: &str) -> Self {
        self.currency_symbol = symbol.to_string();
        self
    }

    /// `(prefix, suffix)` that place `symbol` by this locale's convention:
    /// `("$", "")` for en-US, `("", " €")` for de-DE.
    pub fn currency_affixes(&self, symbol: &str) -> (String, String) {
        match self.currency_position {
            CurrencyPosition::Prefix => {
                (format!("{symbol}{}", self.currency_spacing), String::new())
            }
            CurrencyPosition::Suffix => {
                (String::new(), format!("{}{symbol}", self.currency_spacing))
            }
        }
    }

    /// Re-render a number formatted the default way (`.` decimal mark, `,` or
    /// no thousands grouping, optional leading `-`) with this locale's marks:
    /// `"-1234.5"` / `"-1,234.5"` → `"-1.234,5"` in de-DE. Anything that is not
    /// such a number (`NaN`, `inf`, text) is returned unchanged.
    pub fn localize_number(&self, s: &str) -> String {
        localize_number(
            s,
            &self.decimal_mark,
            &self.big_mark,
            self.min_grouping_digits,
        )
    }

    /// Inverse of [`localize_number`](Self::localize_number) for parsing a
    /// label back to a number: strips thousands marks and restores `.`.
    pub fn parse_number(&self, s: &str) -> Option<f64> {
        let mut t = s.trim().to_string();
        if !self.big_mark.is_empty() {
            t = t.replace(&self.big_mark, "");
        }
        if self.decimal_mark != "." {
            t = t.replace(&self.decimal_mark, ".");
        }
        t.parse().ok()
    }

    /// Format a break value like the default axis labels (integers verbatim,
    /// otherwise up to two decimals, trailing zeros trimmed) with this
    /// locale's marks: `1234.5` → `"1.234,5"` in de-DE.
    pub fn format_number(&self, v: f64) -> String {
        crate::scale::util::format_number_locale(v, self)
    }

    /// Format epoch seconds (UTC) with a strftime-style format, using this
    /// locale's month and weekday names. Supported: `%Y %y %m %-m %b %B %d
    /// %-d %e %a %A %H %M %S %%`.
    pub fn strftime(&self, epoch_secs: i64, fmt: &str) -> String {
        crate::scale::datetime::strftime_locale(epoch_secs as f64, fmt, Some(self))
    }

    /// Format epoch seconds (UTC) as this locale's date ([`date_format`](Self::date_format)),
    /// plus [`time_format`](Self::time_format) when not at midnight.
    pub fn format_datetime(&self, epoch_secs: i64) -> String {
        let fmt = if epoch_secs.rem_euclid(86_400) == 0 {
            self.date_format.clone()
        } else {
            format!("{} {}", self.date_format, self.time_format)
        };
        self.strftime(epoch_secs, &fmt)
    }
}

impl Default for Locale {
    /// [`Locale::en_us`].
    fn default() -> Self {
        Self::en_us()
    }
}

impl std::str::FromStr for Locale {
    type Err = std::convert::Infallible;
    /// [`Locale::from_tag`] (never fails; unknown tags give en-US).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_tag(s))
    }
}

/// Re-render a default-formatted number (`-1,234.5` or `-1234.5`) with the
/// given marks; see [`Locale::localize_number`].
pub(crate) fn localize_number(
    s: &str,
    decimal_mark: &str,
    big_mark: &str,
    min_grouping_digits: usize,
) -> String {
    let (sign, rest) = match s.strip_prefix('-') {
        Some(r) => ("-", r),
        None => ("", s),
    };
    let (int_part, dec_part) = match rest.split_once('.') {
        Some((i, d)) => (i, Some(d)),
        None => (rest, None),
    };
    let digits: String = int_part.chars().filter(|&c| c != ',').collect();
    let is_number = !digits.is_empty()
        && digits.chars().all(|c| c.is_ascii_digit())
        && int_part.chars().all(|c| c.is_ascii_digit() || c == ',')
        && dec_part.is_none_or(|d| d.chars().all(|c| c.is_ascii_digit()));
    if !is_number {
        return s.to_string();
    }
    let n = digits.len();
    let mut out = String::with_capacity(s.len() + n / 3 * big_mark.len());
    out.push_str(sign);
    let group = !big_mark.is_empty() && n >= 3 + min_grouping_digits.max(1);
    for (i, ch) in digits.chars().enumerate() {
        if group && i > 0 && (n - i).is_multiple_of(3) {
            out.push_str(big_mark);
        }
        out.push(ch);
    }
    if let Some(d) = dec_part {
        out.push_str(decimal_mark);
        out.push_str(d);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn localize_regroups_and_swaps_marks() {
        let de = Locale::de_de();
        assert_eq!(de.localize_number("1234.5"), "1.234,5");
        assert_eq!(de.localize_number("-1,234,567.25"), "-1.234.567,25");
        assert_eq!(de.localize_number("999"), "999");
        assert_eq!(de.localize_number("0.5"), "0,5");
        assert_eq!(de.localize_number("NaN"), "NaN");
        assert_eq!(de.localize_number("-inf"), "-inf");
        assert_eq!(de.localize_number(""), "");
        let fr = Locale::fr_fr();
        assert_eq!(fr.localize_number("1234567.5"), "1 234 567,5");
        let no_group = Locale::de_de().with_big_mark("");
        assert_eq!(no_group.localize_number("1,234.5"), "1234,5");
        let min2 = Locale::de_de().with_min_grouping_digits(2);
        assert_eq!(min2.localize_number("2020"), "2020");
        assert_eq!(min2.localize_number("20200"), "20.200");
    }

    #[test]
    fn parse_number_inverts_localize() {
        let de = Locale::de_de();
        assert_eq!(de.parse_number("1.234,5"), Some(1234.5));
        assert_eq!(de.parse_number("-0,25"), Some(-0.25));
        assert_eq!(Locale::fr_fr().parse_number("1 000"), Some(1000.0));
        assert_eq!(Locale::en_us().parse_number("1,000.5"), Some(1000.5));
    }

    #[test]
    fn tags_resolve_to_presets() {
        for (tag, want) in [
            ("de-DE", "de-DE"),
            ("de", "de-DE"),
            ("DE_at", "de-AT"),
            ("de-LU", "de-DE"),
            ("de_DE.UTF-8", "de-DE"),
            ("de_DE@euro", "de-DE"),
            ("fr-FR", "fr-FR"),
            ("fr-CA", "fr-CA"),
            ("fr-LU", "fr-FR"),
            ("it", "it-IT"),
            ("pt", "pt-BR"),
            ("no", "nb-NO"),
            ("en-GB", "en-GB"),
            ("en_gb", "en-GB"),
            ("en-Latn-GB", "en-GB"),
            ("en-US", "en-US"),
            ("en", "en-US"),
            ("C", "en-US"),
            ("", "en-US"),
            ("ja-JP", "en-US"),
        ] {
            assert_eq!(Locale::from_tag(tag).tag, want, "{tag:?}");
        }
        assert!(Locale::try_from_tag("ja-JP").is_none());
        assert!(Locale::try_from_tag("").is_none());
        assert_eq!("de-DE".parse::<Locale>().unwrap(), Locale::de_de());
        assert_eq!(Locale::default(), Locale::en_us());
    }

    #[test]
    fn currency_affixes_follow_placement() {
        assert_eq!(
            Locale::en_us().currency_affixes("$"),
            ("$".into(), String::new())
        );
        assert_eq!(
            Locale::de_de().currency_affixes("€"),
            (String::new(), " €".into())
        );
        assert_eq!(
            Locale::en_gb().currency_affixes("£"),
            ("£".into(), String::new())
        );
    }

    #[test]
    fn dates_per_locale() {
        let t = 1_792_022_400; // 2026-10-15 00:00 UTC (Thursday)
        assert_eq!(Locale::de_de().format_datetime(t), "15. Okt. 2026");
        assert_eq!(Locale::en_us().format_datetime(t), "Oct 15, 2026");
        assert_eq!(Locale::en_gb().format_datetime(t), "15 Oct 2026");
        assert_eq!(Locale::fr_fr().format_datetime(t), "15 oct. 2026");
        assert_eq!(
            Locale::de_de().format_datetime(t + 9 * 3600 + 30 * 60),
            "15. Okt. 2026 09:30"
        );
        assert_eq!(
            Locale::de_de().strftime(t, "%A, %-d. %B %Y"),
            "Donnerstag, 15. Oktober 2026"
        );
        assert_eq!(Locale::fr_fr().strftime(t, "%a %-d %B"), "jeu. 15 octobre");
        assert_eq!(Locale::en_us().strftime(t, "%a %A"), "Thu Thursday");
        assert_eq!(Locale::de_de().strftime(t - 86_400 * 200, "%B"), "März");
    }
}
