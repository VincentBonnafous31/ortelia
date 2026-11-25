//! The value of an attribute, as read from or written to a store, and the encoding of dates in
//! the 8 bytes of a store slot.

use jiff::Timestamp;
use jiff::civil::Date;

/// The value of an attribute.
///
/// The lifetime `'a` ties a borrowed string to the store it comes from: like an internal
/// `const char*`, it must not be used after the store changes, except that here the compiler
/// enforces it (the store cannot be modified while such a value is alive).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value<'a> {
    /// The attribute has no value.
    Unset,
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(&'a str),
    /// Index of the literal in its enum.
    Enum(u32),
    Date(Date),
    /// Always a whole number of microseconds.
    DateTime(Timestamp),
}

/// Encodes a date as `year × 512 + month × 32 + day`.
///
/// The integer keeps the chronological order, negative years included (`month × 32 + day` is
/// below 512), and decodes without any calendar computation.
pub fn encode_date(date: Date) -> i64 {
    i64::from(date.year()) * 512 + i64::from(date.month()) * 32 + i64::from(date.day())
}

/// Decodes a date encoded by [`encode_date`].
///
/// # Panics
/// If `code` was not produced by [`encode_date`]: a store only decodes what it encoded.
pub fn decode_date(code: i64) -> Date {
    // Euclidean division keeps the remainder positive for negative years, so that
    // -1 × 512 + 100 decodes back to year -1 and remainder 100.
    let year = code.div_euclid(512);
    let rest = code.rem_euclid(512);
    let (month, day) = (rest / 32, rest % 32);
    let (Ok(year), Ok(month), Ok(day)) =
        (i16::try_from(year), i8::try_from(month), i8::try_from(day))
    else {
        panic!("invalid date code {code}");
    };
    Date::new(year, month, day).unwrap_or_else(|_| panic!("invalid date code {code}"))
}

/// `true` if an instant is a whole number of microseconds, so that it can be stored exactly.
pub fn is_whole_microsecond(instant: Timestamp) -> bool {
    instant.subsec_nanosecond() % 1000 == 0
}

/// Encodes an instant as a number of microseconds since 1970-01-01T00:00:00Z.
pub fn encode_datetime(instant: Timestamp) -> i64 {
    instant.as_microsecond()
}

/// Decodes an instant encoded by [`encode_datetime`].
///
/// # Panics
/// If `code` is out of the range of [`Timestamp`]: a store only decodes what it encoded.
pub fn decode_datetime(code: i64) -> Timestamp {
    Timestamp::from_microsecond(code).unwrap_or_else(|_| panic!("invalid instant code {code}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(text: &str) -> Date {
        text.parse().expect("date")
    }

    fn instant(text: &str) -> Timestamp {
        text.parse().expect("instant")
    }

    #[test]
    fn dates_round_trip() {
        for text in [
            "1985-03-12",
            "2026-12-31",
            "0000-01-01",
            "-000044-03-15",
            "-009999-01-01",
            "9999-12-31",
        ] {
            assert_eq!(decode_date(encode_date(date(text))), date(text), "{text}");
        }
    }

    #[test]
    fn date_codes_keep_the_chronological_order() {
        let dates = [
            "-009999-01-01",
            "-000044-03-15",
            "-000001-12-31",
            "0000-01-01",
            "0001-01-01",
            "1985-03-12",
            "1985-03-13",
            "1985-04-01",
            "9999-12-31",
        ];
        let codes: Vec<i64> = dates.iter().map(|t| encode_date(date(t))).collect();
        assert!(codes.windows(2).all(|w| w[0] < w[1]), "{codes:?}");
    }

    #[test]
    fn instants_round_trip() {
        for text in [
            "2026-03-12T10:15:00.123456Z",
            "1970-01-01T00:00:00Z",
            "1969-12-31T23:59:59.999999Z",
            "-000044-03-15T12:00:00.000001Z",
        ] {
            let value = instant(text);
            assert!(is_whole_microsecond(value), "{text}");
            assert_eq!(decode_datetime(encode_datetime(value)), value, "{text}");
        }
    }

    #[test]
    fn instant_codes_keep_the_chronological_order() {
        let instants = [
            "-000044-03-15T12:00:00Z",
            "1969-12-31T23:59:59.999999Z",
            "1970-01-01T00:00:00Z",
            "2026-03-12T10:15:00.123456Z",
            "2026-03-12T10:15:00.123457Z",
        ];
        let codes: Vec<i64> = instants
            .iter()
            .map(|t| encode_datetime(instant(t)))
            .collect();
        assert!(codes.windows(2).all(|w| w[0] < w[1]), "{codes:?}");
    }

    #[test]
    fn nanoseconds_are_not_whole_microseconds() {
        assert!(!is_whole_microsecond(instant(
            "2026-03-12T10:15:00.123456789Z"
        )));
        assert!(!is_whole_microsecond(instant(
            "1969-12-31T23:59:59.999999999Z"
        )));
    }
}
