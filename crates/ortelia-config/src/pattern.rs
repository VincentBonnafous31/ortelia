//! The date patterns of the columns: a subset of the LDML patterns (Unicode Technical Standard
//! #35), parsed when `app.json` is checked. The presentation layer renders them, with the names
//! of the months and days of the current language.
//!
//! A pattern is a sequence of fields (runs of one letter: `dd`, `MMMM`) and of literal texts.
//! Any ASCII letter is reserved for the fields: a letter outside the supported fields is an
//! error. A text between apostrophes is copied as is, and `''` gives an apostrophe.

use std::fmt;

/// A field of a date pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// `yy`: the last two digits of the year.
    Year2,
    /// `yyyy`: the year, on four digits at least.
    Year4,
    /// `M`, `MM`: the number of the month, with `MM` on two digits.
    Month { padded: bool },
    /// `MMM`: the abbreviated name of the month.
    MonthAbbr,
    /// `MMMM`: the full name of the month.
    MonthFull,
    /// `d`, `dd`: the day of the month.
    Day { padded: bool },
    /// `EEE`: the abbreviated name of the day of the week.
    WeekdayAbbr,
    /// `EEEE`: the full name of the day of the week.
    WeekdayFull,
    /// `H`, `HH`: the hour, 0 to 23.
    Hour { padded: bool },
    /// `m`, `mm`: the minute.
    Minute { padded: bool },
    /// `s`, `ss`: the second.
    Second { padded: bool },
    /// `S` to `SSSSSS`: the first 1 to 6 digits of the fraction of the second.
    Fraction { digits: u8 },
}

impl Field {
    /// `true` for the fields of the time of day, refused in the pattern of a date.
    pub fn is_time(self) -> bool {
        matches!(
            self,
            Field::Hour { .. }
                | Field::Minute { .. }
                | Field::Second { .. }
                | Field::Fraction { .. }
        )
    }

    /// The field written as a run of `count` letters `letter`, if it is supported.
    fn from_run(letter: char, count: usize) -> Option<Field> {
        let field = match (letter, count) {
            ('y', 2) => Field::Year2,
            ('y', 4) => Field::Year4,
            ('M', 1 | 2) => Field::Month { padded: count == 2 },
            ('M', 3) => Field::MonthAbbr,
            ('M', 4) => Field::MonthFull,
            ('d', 1 | 2) => Field::Day { padded: count == 2 },
            ('E', 3) => Field::WeekdayAbbr,
            ('E', 4) => Field::WeekdayFull,
            ('H', 1 | 2) => Field::Hour { padded: count == 2 },
            ('m', 1 | 2) => Field::Minute { padded: count == 2 },
            ('s', 1 | 2) => Field::Second { padded: count == 2 },
            // The guard keeps `count` within 1..=6, which fits in a u8.
            ('S', 1..=6) => Field::Fraction {
                digits: count as u8,
            },
            _ => return None,
        };
        Some(field)
    }
}

/// An element of a date pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternItem {
    Field(Field),
    /// A text copied as is.
    Literal(String),
}

/// A date pattern, parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatePattern {
    items: Vec<PatternItem>,
}

/// Why a date pattern is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternError {
    /// A run of letters that is not a supported field: `Q`, `yyy`, `EEEEE`.
    UnknownField(String),
    /// An apostrophe opens a text that no apostrophe closes.
    UnclosedQuote,
    /// A field of the time of day (`HH`, `mm`…) in the pattern of a date.
    TimeInDate(String),
}

impl fmt::Display for PatternError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PatternError::UnknownField(run) => write!(f, "unknown field `{run}`"),
            PatternError::UnclosedQuote => f.write_str("unclosed quote"),
            PatternError::TimeInDate(run) => {
                write!(f, "field `{run}` is a time, and the column is a date")
            }
        }
    }
}

impl DatePattern {
    /// Parses a pattern, for a date (`with_time` false: no time field) or a date-time.
    pub fn parse(text: &str, with_time: bool) -> Result<DatePattern, PatternError> {
        let mut items = Vec::new();
        // The literal text being read, pushed as one item before the next field.
        let mut literal = String::new();
        // A `Peekable` iterator, like a reader with a look-ahead of one character.
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\'' {
                // `''` outside a quoted text: an apostrophe.
                if chars.next_if_eq(&'\'').is_some() {
                    literal.push('\'');
                    continue;
                }
                // A quoted text, up to the closing apostrophe; `''` inside it is an apostrophe.
                loop {
                    match chars.next() {
                        None => return Err(PatternError::UnclosedQuote),
                        Some('\'') if chars.next_if_eq(&'\'').is_some() => literal.push('\''),
                        Some('\'') => break,
                        Some(other) => literal.push(other),
                    }
                }
            } else if c.is_ascii_alphabetic() {
                let mut count = 1;
                while chars.next_if_eq(&c).is_some() {
                    count += 1;
                }
                let run = c.to_string().repeat(count);
                let field =
                    Field::from_run(c, count).ok_or(PatternError::UnknownField(run.clone()))?;
                if field.is_time() && !with_time {
                    return Err(PatternError::TimeInDate(run));
                }
                if !literal.is_empty() {
                    items.push(PatternItem::Literal(std::mem::take(&mut literal)));
                }
                items.push(PatternItem::Field(field));
            } else {
                literal.push(c);
            }
        }
        if !literal.is_empty() {
            items.push(PatternItem::Literal(literal));
        }
        Ok(DatePattern { items })
    }

    /// The fields and literal texts, in their order.
    pub fn items(&self) -> &[PatternItem] {
        &self.items
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Field::*;

    fn field(f: Field) -> PatternItem {
        PatternItem::Field(f)
    }

    fn text(t: &str) -> PatternItem {
        PatternItem::Literal(t.to_owned())
    }

    fn items(pattern: &str, with_time: bool) -> Vec<PatternItem> {
        DatePattern::parse(pattern, with_time)
            .unwrap_or_else(|e| panic!("{pattern}: {e}"))
            .items
    }

    #[test]
    fn day_month_year() {
        assert_eq!(
            items("dd/MM/yyyy", false),
            [
                field(Day { padded: true }),
                text("/"),
                field(Month { padded: true }),
                text("/"),
                field(Year4)
            ]
        );
    }

    /// Pattern of the scenario "Mois en toutes lettres".
    #[test]
    fn month_in_full() {
        assert_eq!(
            items("d MMMM yyyy", false),
            [
                field(Day { padded: false }),
                text(" "),
                field(MonthFull),
                text(" "),
                field(Year4)
            ]
        );
    }

    /// Pattern of the scenario "Jour de la semaine et texte entre apostrophes".
    #[test]
    fn weekday_and_quoted_text() {
        assert_eq!(
            items("EEEE d MMM 'à' HH'h'mm", true),
            [
                field(WeekdayFull),
                text(" "),
                field(Day { padded: false }),
                text(" "),
                field(MonthAbbr),
                text(" à "),
                field(Hour { padded: true }),
                text("h"),
                field(Minute { padded: true })
            ]
        );
    }

    /// Pattern of the scenario "Microsecondes", and the other fractions.
    #[test]
    fn fractions_of_seconds() {
        assert_eq!(
            items("HH:mm:ss.SSSSSS", true),
            [
                field(Hour { padded: true }),
                text(":"),
                field(Minute { padded: true }),
                text(":"),
                field(Second { padded: true }),
                text("."),
                field(Fraction { digits: 6 })
            ]
        );
        for digits in 1..=6u8 {
            let pattern = "S".repeat(usize::from(digits));
            assert_eq!(items(&pattern, true), [field(Fraction { digits })]);
        }
    }

    #[test]
    fn other_fields() {
        assert_eq!(
            items("yy M d EEE H m s", true)
                .into_iter()
                .filter(|i| matches!(i, PatternItem::Field(_)))
                .collect::<Vec<_>>(),
            [
                field(Year2),
                field(Month { padded: false }),
                field(Day { padded: false }),
                field(WeekdayAbbr),
                field(Hour { padded: false }),
                field(Minute { padded: false }),
                field(Second { padded: false })
            ]
        );
    }

    /// `''` gives an apostrophe, outside or inside a quoted text; non-ASCII letters are text.
    #[test]
    fn apostrophes() {
        assert_eq!(items("''", false), [text("'")]);
        assert_eq!(items("'l''an' yyyy", false), [text("l'an "), field(Year4)]);
        assert_eq!(
            items("d'' é", false),
            [field(Day { padded: false }), text("' é")]
        );
        assert_eq!(items("'yyyy'", false), [text("yyyy")]);
        assert_eq!(items("", false), []);
    }

    /// Scenario "Lettre inconnue", and runs of a known letter of an unsupported length.
    #[test]
    fn unknown_letters() {
        assert_eq!(
            DatePattern::parse("dd/MM/yyyy Q", false),
            Err(PatternError::UnknownField("Q".into()))
        );
        for (pattern, run) in [
            ("yyy", "yyy"),
            ("y", "y"),
            ("MMMMM", "MMMMM"),
            ("ddd", "ddd"),
            ("E", "E"),
            ("SSSSSSS", "SSSSSSS"),
            ("a", "a"),
        ] {
            assert_eq!(
                DatePattern::parse(pattern, true),
                Err(PatternError::UnknownField(run.into())),
                "{pattern}"
            );
        }
    }

    /// An unclosed quoted text.
    #[test]
    fn unclosed_quote() {
        for pattern in ["'à", "dd 'h''", "'"] {
            assert_eq!(
                DatePattern::parse(pattern, true),
                Err(PatternError::UnclosedQuote),
                "{pattern}"
            );
        }
    }

    /// Scenario "Heure pour une date": the same pattern is valid for a date-time.
    #[test]
    fn time_in_a_date() {
        assert_eq!(
            DatePattern::parse("dd/MM/yyyy HH:mm", false),
            Err(PatternError::TimeInDate("HH".into()))
        );
        assert!(DatePattern::parse("dd/MM/yyyy HH:mm", true).is_ok());
        let message = PatternError::TimeInDate("HH".into()).to_string();
        assert!(message.contains("HH"), "{message}");
    }
}
