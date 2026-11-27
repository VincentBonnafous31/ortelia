//! The text shown for each type of value, in the current language, and the rendering of the
//! date patterns (parsed by `ortelia-config`).
//!
//! Two sets of conventions: French for `fr` (decimal comma, French names, `dd/MM/yyyy`), and
//! English for any other language for now. The names of the months and of the days come from
//! CLDR.

use std::fmt::Write;
use std::sync::LazyLock;

use jiff::civil::{Date, DateTime};
use jiff::tz::Offset;
use ortelia_config::{Align, DatePattern, Field, PatternItem};
use ortelia_model::metamodel::AttrType;
use ortelia_model::value::Value;

/// The conventions of a language for numbers and dates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conventions {
    French,
    English,
}

impl Conventions {
    /// The conventions of a language code: French for `fr` and its regional variants (`fr-FR`,
    /// `fr_CA`), English otherwise.
    pub fn of(lang: &str) -> Conventions {
        if lang.split(['-', '_']).next() == Some("fr") {
            Conventions::French
        } else {
            Conventions::English
        }
    }

    /// The default pattern of a date (`with_time` false) or of a date-time.
    pub fn default_pattern(self, with_time: bool) -> &'static DatePattern {
        // Parsed once, at the first use (`LazyLock`, like a static initialised on demand in
        // Java): a date column without a pattern of its own uses it for each cell.
        static PATTERNS: LazyLock<[DatePattern; 4]> = LazyLock::new(|| {
            let parse = |text, with_time| {
                DatePattern::parse(text, with_time).expect("the default patterns are valid")
            };
            [
                parse("dd/MM/yyyy", false),
                parse("dd/MM/yyyy HH:mm:ss", true),
                parse("yyyy-MM-dd", false),
                parse("yyyy-MM-dd HH:mm:ss", true),
            ]
        });
        let index = match self {
            Conventions::French => 0,
            Conventions::English => 2,
        };
        &PATTERNS[index + usize::from(with_time)]
    }

    fn months(self) -> &'static [&'static str; 12] {
        match self {
            Conventions::French => &[
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
            ],
            Conventions::English => &[
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
            ],
        }
    }

    fn short_months(self) -> &'static [&'static str; 12] {
        match self {
            Conventions::French => &[
                "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.",
                "nov.", "déc.",
            ],
            Conventions::English => &[
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
            ],
        }
    }

    /// Monday first.
    fn weekdays(self) -> &'static [&'static str; 7] {
        match self {
            Conventions::French => &[
                "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche",
            ],
            Conventions::English => &[
                "Monday",
                "Tuesday",
                "Wednesday",
                "Thursday",
                "Friday",
                "Saturday",
                "Sunday",
            ],
        }
    }

    fn short_weekdays(self) -> &'static [&'static str; 7] {
        match self {
            Conventions::French => &["lun.", "mar.", "mer.", "jeu.", "ven.", "sam.", "dim."],
            Conventions::English => &["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"],
        }
    }
}

/// What a cell of a table shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cell {
    /// No value.
    Empty,
    Text(String),
    /// A boolean: a check box, not editable.
    Check(bool),
}

impl Cell {
    /// The text of the cell, for the filter: none for an empty cell or a check box.
    pub fn text(&self) -> Option<&str> {
        match self {
            Cell::Text(text) => Some(text),
            Cell::Empty | Cell::Check(_) => None,
        }
    }
}

/// The default alignment of a type: numbers to the right, booleans and dates centred, texts and
/// enums to the left.
pub fn default_align(attr_type: AttrType) -> Align {
    match attr_type {
        AttrType::Int | AttrType::Float => Align::Right,
        AttrType::Boolean | AttrType::Date | AttrType::DateTime => Align::Center,
        AttrType::String | AttrType::Enum(_) => Align::Left,
    }
}

/// The default width of a column of a type, in points.
pub fn default_width(attr_type: AttrType) -> f32 {
    match attr_type {
        AttrType::String => 140.0,
        AttrType::Enum(_) => 120.0,
        AttrType::Int => 70.0,
        AttrType::Float => 80.0,
        AttrType::Boolean => 60.0,
        AttrType::Date => 100.0,
        AttrType::DateTime => 160.0,
    }
}

/// How the values of a column are shown: what the type alone does not tell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueFormat {
    pub conventions: Conventions,
    /// The pattern of a date or date-time column; `None`: the default of the language.
    pub pattern: Option<DatePattern>,
    /// The labels of the literals of an enum column, in the order of the literals.
    pub literals: Vec<String>,
}

impl ValueFormat {
    /// The cell of a value, on one line.
    pub fn cell(&self, value: Value<'_>) -> Cell {
        match value {
            Value::Unset => Cell::Empty,
            Value::Bool(b) => Cell::Check(b),
            other => Cell::Text(self.text(other, false)),
        }
    }

    /// The text of a value; with `keep_lines` false, a text on one line (each line break or
    /// tab becomes a space), as in a cell of a table. Empty for an absent value and for a
    /// boolean, which a check box shows.
    pub fn text(&self, value: Value<'_>, keep_lines: bool) -> String {
        match value {
            Value::Unset | Value::Bool(_) => String::new(),
            Value::Str(text) if keep_lines => text.to_owned(),
            Value::Str(text) => one_line(text),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => format_float(f, self.conventions),
            Value::Enum(index) => usize::try_from(index)
                .ok()
                .and_then(|i| self.literals.get(i))
                .cloned()
                .unwrap_or_default(),
            Value::Date(date) => self.date_text(date, None),
            Value::DateTime(instant) => {
                // In UTC, whatever the time zone of the computer.
                let civil = Offset::UTC.to_datetime(instant);
                self.date_text(civil.date(), Some(civil))
            }
        }
    }

    fn date_text(&self, date: Date, time: Option<DateTime>) -> String {
        let pattern = match &self.pattern {
            Some(pattern) => pattern,
            None => self.conventions.default_pattern(time.is_some()),
        };
        render(pattern, self.conventions, date, time)
    }
}

/// A text on one line: each line break or tab becomes a space (`\r\n` gives one space).
fn one_line(text: &str) -> String {
    if !text.contains(['\n', '\r', '\t']) {
        return text.to_owned();
    }
    text.replace("\r\n", " ").replace(['\n', '\r', '\t'], " ")
}

/// A float with the fewest digits that represent it exactly, without exponent: Rust's `Display`
/// of `f64` does just that (`1.68`, `2`, `0.0000001`). The decimal separator follows the
/// language.
fn format_float(value: f64, conventions: Conventions) -> String {
    let text = value.to_string();
    match conventions {
        Conventions::French => text.replace('.', ","),
        Conventions::English => text,
    }
}

/// Renders a pattern for a date, and for its time if it is a date-time.
fn render(
    pattern: &DatePattern,
    conventions: Conventions,
    date: Date,
    time: Option<DateTime>,
) -> String {
    let mut out = String::new();
    // `write!` into a `String` cannot fail: its result is ignored with `let _`.
    let padded = |out: &mut String, value: i64, pad: bool| {
        let _ = if pad {
            write!(out, "{value:02}")
        } else {
            write!(out, "{value}")
        };
    };
    let month = usize::try_from(date.month() - 1).unwrap_or(0);
    let weekday = usize::try_from(date.weekday().to_monday_zero_offset()).unwrap_or(0);
    for item in pattern.items() {
        let field = match item {
            PatternItem::Literal(text) => {
                out.push_str(text);
                continue;
            }
            PatternItem::Field(field) => *field,
        };
        match field {
            Field::Year2 => padded(&mut out, i64::from(date.year()).rem_euclid(100), true),
            Field::Year4 => {
                let _ = write!(out, "{:04}", date.year());
            }
            Field::Month { padded: pad } => padded(&mut out, i64::from(date.month()), pad),
            Field::MonthAbbr => out.push_str(conventions.short_months()[month]),
            Field::MonthFull => out.push_str(conventions.months()[month]),
            Field::Day { padded: pad } => padded(&mut out, i64::from(date.day()), pad),
            Field::WeekdayAbbr => out.push_str(conventions.short_weekdays()[weekday]),
            Field::WeekdayFull => out.push_str(conventions.weekdays()[weekday]),
            // A date has no time fields: the check of `app.json` refuses them.
            Field::Hour { padded: pad } => {
                padded(&mut out, time.map_or(0, |t| i64::from(t.hour())), pad)
            }
            Field::Minute { padded: pad } => {
                padded(&mut out, time.map_or(0, |t| i64::from(t.minute())), pad)
            }
            Field::Second { padded: pad } => {
                padded(&mut out, time.map_or(0, |t| i64::from(t.second())), pad)
            }
            Field::Fraction { digits } => {
                // The fraction is truncated, as LDML does: the first `digits` of the six
                // digits of the microseconds.
                let micros = time.map_or(0, |t| t.subsec_nanosecond() / 1000);
                let six = format!("{micros:06}");
                out.push_str(&six[..usize::from(digits).min(6)]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format(conventions: Conventions, pattern: Option<&str>, with_time: bool) -> ValueFormat {
        ValueFormat {
            conventions,
            pattern: pattern.map(|p| DatePattern::parse(p, with_time).expect("pattern")),
            literals: Vec::new(),
        }
    }

    fn french() -> ValueFormat {
        format(Conventions::French, None, false)
    }

    fn english() -> ValueFormat {
        format(Conventions::English, None, false)
    }

    fn text(text: &str) -> Cell {
        Cell::Text(text.to_owned())
    }

    fn date(text: &str) -> Value<'static> {
        Value::Date(text.parse().expect("date"))
    }

    fn instant(text: &str) -> Value<'static> {
        Value::DateTime(text.parse().expect("instant"))
    }

    /// Scenario "Notes absentes": an absent value of any type is an empty cell.
    #[test]
    fn absent_value() {
        assert_eq!(french().cell(Value::Unset), Cell::Empty);
        assert_eq!(Cell::Empty.text(), None);
    }

    /// Scenario "Texte sur plusieurs lignes", and tabs.
    #[test]
    fn texts_on_one_line() {
        assert_eq!(
            french().cell(Value::Str("Ligne 1\nLigne 2")),
            text("Ligne 1 Ligne 2")
        );
        assert_eq!(french().cell(Value::Str("a\r\nb\tc")), text("a b c"));
        assert_eq!(french().cell(Value::Str("Dupont")), text("Dupont"));
    }

    /// The text of a value, with its line breaks kept (a text area) or not.
    #[test]
    fn texts_that_keep_their_lines() {
        let value = Value::Str("Ligne 1\nLigne 2");
        assert_eq!(french().text(value, true), "Ligne 1\nLigne 2");
        assert_eq!(french().text(value, false), "Ligne 1 Ligne 2");
        // The other types are shown as in a cell; an absent value and a boolean have no text.
        assert_eq!(french().text(Value::Float(1.68), true), "1,68");
        assert_eq!(french().text(Value::Unset, true), "");
        assert_eq!(french().text(Value::Bool(true), false), "");
    }

    /// Scenarios "Taille en français", "Taille en anglais" and "Grand entier".
    #[test]
    fn numbers() {
        assert_eq!(french().cell(Value::Float(1.68)), text("1,68"));
        assert_eq!(english().cell(Value::Float(1.68)), text("1.68"));
        assert_eq!(french().cell(Value::Int(1_234_567)), text("1234567"));
        assert_eq!(french().cell(Value::Int(-42)), text("-42"));
        // The fewest digits, without exponent.
        assert_eq!(english().cell(Value::Float(2.0)), text("2"));
        assert_eq!(english().cell(Value::Float(1e-7)), text("0.0000001"));
        assert_eq!(english().cell(Value::Float(1.7)), text("1.7"));
    }

    /// Scenario "Contact autorisé": a check box, without text for the filter.
    #[test]
    fn booleans() {
        assert_eq!(french().cell(Value::Bool(true)), Cell::Check(true));
        assert_eq!(Cell::Check(true).text(), None);
    }

    /// Scenario "Situation avec libellé": the label of the literal, given by the column.
    #[test]
    fn enums() {
        let format = ValueFormat {
            literals: vec!["Célibataire".into(), "Marié(e)".into()],
            ..french()
        };
        assert_eq!(format.cell(Value::Enum(1)), text("Marié(e)"));
    }

    /// Scenarios "Date de naissance" and "Inscription en UTC", and the English defaults.
    #[test]
    fn default_dates() {
        assert_eq!(french().cell(date("1985-03-12")), text("12/03/1985"));
        assert_eq!(english().cell(date("1985-03-12")), text("1985-03-12"));
        let registered = instant("2024-01-15T09:30:00Z");
        assert_eq!(french().cell(registered), text("15/01/2024 09:30:00"));
        assert_eq!(english().cell(registered), text("2024-01-15 09:30:00"));
        // An instant given with an offset is shown in UTC.
        assert_eq!(
            french().cell(instant("2024-01-15T23:30:00-02:00")),
            text("16/01/2024 01:30:00")
        );
    }

    /// Scenarios "Mois en toutes lettres" and "Même motif en anglais".
    #[test]
    fn month_in_full() {
        let value = date("1985-03-12");
        let pattern = Some("d MMMM yyyy");
        assert_eq!(
            format(Conventions::French, pattern, false).cell(value),
            text("12 mars 1985")
        );
        assert_eq!(
            format(Conventions::English, pattern, false).cell(value),
            text("12 March 1985")
        );
    }

    /// Scenario "Jour de la semaine et texte entre apostrophes".
    #[test]
    fn weekday_and_quoted_text() {
        let format = format(Conventions::French, Some("EEEE d MMM 'à' HH'h'mm"), true);
        assert_eq!(
            format.cell(instant("2024-01-15T09:30:00Z")),
            text("lundi 15 janv. à 09h30")
        );
        let format = ValueFormat {
            conventions: Conventions::English,
            ..format
        };
        assert_eq!(
            format.cell(instant("2024-01-15T09:30:00Z")),
            text("Monday 15 Jan à 09h30")
        );
    }

    /// Scenario "Microsecondes", and a truncated fraction.
    #[test]
    fn microseconds() {
        let value = instant("2026-11-23T10:15:30.123456Z");
        assert_eq!(
            format(Conventions::French, Some("HH:mm:ss.SSSSSS"), true).cell(value),
            text("10:15:30.123456")
        );
        assert_eq!(
            format(Conventions::French, Some("s.SSS"), true).cell(value),
            text("30.123")
        );
    }

    /// The other fields, with and without padding, and the abbreviated days.
    #[test]
    fn other_fields() {
        let value = instant("2005-07-03T04:05:06Z");
        assert_eq!(
            format(Conventions::French, Some("yy M d H m s EEE"), true).cell(value),
            text("05 7 3 4 5 6 dim.")
        );
        assert_eq!(
            format(Conventions::English, Some("EEE, MMM d yyyy"), true).cell(value),
            text("Sun, Jul 3 2005")
        );
    }

    /// Scenario "Alignements de l'annuaire", by type; and the default widths.
    #[test]
    fn default_alignments_and_widths() {
        assert_eq!(default_align(AttrType::Int), Align::Right);
        assert_eq!(default_align(AttrType::Float), Align::Right);
        assert_eq!(default_align(AttrType::Boolean), Align::Center);
        assert_eq!(default_align(AttrType::Date), Align::Center);
        assert_eq!(default_align(AttrType::DateTime), Align::Center);
        assert_eq!(default_align(AttrType::String), Align::Left);
        assert_eq!(default_width(AttrType::String), 140.0);
        assert_eq!(default_width(AttrType::DateTime), 160.0);
    }

    #[test]
    fn conventions_of_languages() {
        assert_eq!(Conventions::of("fr"), Conventions::French);
        assert_eq!(Conventions::of("fr-CA"), Conventions::French);
        assert_eq!(Conventions::of("fr_FR"), Conventions::French);
        assert_eq!(Conventions::of("fra"), Conventions::English);
        assert_eq!(Conventions::of("en"), Conventions::English);
        assert_eq!(Conventions::of("de"), Conventions::English);
    }
}
