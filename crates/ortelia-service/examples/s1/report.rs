//! Results of the benchmark: one line per measure, exchanged between processes as tab-separated
//! text, and gathered in a Markdown table.

use std::fmt::Write;

/// A measure taken by one case.
#[derive(Debug, Clone, PartialEq)]
pub struct Measure {
    /// `C1`, `M1`, `A3`...
    pub code: String,
    pub label: String,
    pub unit: String,
    /// `None` when the measure is not available on this system.
    pub value: Option<f64>,
}

impl Measure {
    pub fn new(code: &str, label: &str, unit: &str, value: Option<f64>) -> Measure {
        Measure {
            code: code.to_owned(),
            label: label.to_owned(),
            unit: unit.to_owned(),
            value,
        }
    }

    /// The line a case process prints for this measure.
    pub fn to_line(&self) -> String {
        let value = match self.value {
            Some(value) => value.to_string(),
            None => "-".to_owned(),
        };
        format!(
            "measure\t{}\t{}\t{}\t{value}",
            self.code, self.label, self.unit
        )
    }

    /// Reads a line printed by [`Measure::to_line`]; `None` for any other line.
    pub fn from_line(line: &str) -> Option<Measure> {
        let mut fields = line.split('\t');
        if fields.next()? != "measure" {
            return None;
        }
        let (code, label, unit, value) = (
            fields.next()?,
            fields.next()?,
            fields.next()?,
            fields.next()?,
        );
        let value = if value == "-" {
            None
        } else {
            Some(value.parse().ok()?)
        };
        Some(Measure::new(code, label, unit, value))
    }
}

/// A measure with the case that took it.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub implementation: String,
    pub mode: String,
    /// `None` if the case failed.
    pub measure: Option<Measure>,
}

/// Name of an implementation in the report.
pub fn implementation_name(implementation: &str) -> &str {
    match implementation {
        "floor" => "plancher",
        "v0" => "V0 (serde_json::Value)",
        "store" => "Store",
        other => other,
    }
}

/// Name of a reading mode in the report.
pub fn mode_name(mode: &str) -> &str {
    match mode {
        "stream" => "flux",
        "buffer" => "tampon",
        "count" => "flux, allocations comptées",
        other => other,
    }
}

/// The Markdown report: a heading, then one line per measure and per implementation, sorted by
/// measure so that the implementations of a measure are side by side.
pub fn markdown(title: &str, rows: &[Row]) -> String {
    let mut sorted: Vec<&Row> = rows.iter().collect();
    // A stable sort keeps the order of the cases within a measure.
    sorted.sort_by_key(|row| {
        row.measure
            .as_ref()
            .map(|m| sort_key(&m.code))
            .unwrap_or((u8::MAX, 0, String::new()))
    });
    let mut text = format!("## {title}\n\n");
    text.push_str("| Code | Mesure | Implémentation | Lecture | Valeur | Unité |\n");
    text.push_str("|---|---|---|---|---:|---|\n");
    for row in sorted {
        let implementation = implementation_name(&row.implementation);
        let mode = mode_name(&row.mode);
        match &row.measure {
            Some(m) => {
                let value = match m.value {
                    Some(value) => format_value(value),
                    None => "non disponible".to_owned(),
                };
                let _ = writeln!(
                    text,
                    "| {} | {} | {implementation} | {mode} | {value} | {} |",
                    m.code, m.label, m.unit
                );
            }
            None => {
                let _ = writeln!(
                    text,
                    "| — | cas en échec | {implementation} | {mode} | échec | |"
                );
            }
        }
    }
    text
}

/// Orders the codes C, M, A, E, then by number and suffix (`M2` before `M2r`).
fn sort_key(code: &str) -> (u8, u32, String) {
    let family = match code.as_bytes().first() {
        Some(b'C') => 0,
        Some(b'M') => 1,
        Some(b'A') => 2,
        Some(b'E') => 3,
        _ => 4,
    };
    let digits: String = code
        .chars()
        .skip(1)
        .take_while(char::is_ascii_digit)
        .collect();
    let suffix: String = code.chars().skip(1 + digits.len()).collect();
    (family, digits.parse().unwrap_or(0), suffix)
}

/// Three significant digits at least, without a useless exponent.
fn format_value(value: f64) -> String {
    let magnitude = value.abs();
    if magnitude >= 100.0 || value == value.trunc() {
        return format!("{value:.0}");
    }
    // Decimals so that the third significant digit is the last one shown.
    let decimals = (2.0 - magnitude.log10().floor()) as usize;
    format!("{value:.decimals$}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(implementation: &str, mode: &str, measure: Measure) -> Row {
        Row {
            implementation: implementation.into(),
            mode: mode.into(),
            measure: Some(measure),
        }
    }

    #[test]
    fn line_round_trip() {
        let measure = Measure::new("C1", "temps de chargement", "ms", Some(12.5));
        assert_eq!(Measure::from_line(&measure.to_line()), Some(measure));
        let unavailable = Measure::new("M1", "pic de mémoire", "Mio", None);
        assert_eq!(
            Measure::from_line(&unavailable.to_line()),
            Some(unavailable)
        );
        assert_eq!(Measure::from_line("anything else"), None);
    }

    #[test]
    fn report_from_rows() {
        let rows = vec![
            row(
                "v0",
                "stream",
                Measure::new("A1", "lecture au hasard", "ns", Some(3.21)),
            ),
            row(
                "store",
                "stream",
                Measure::new("C1", "temps de chargement", "ms", Some(1234.5)),
            ),
            row(
                "v0",
                "stream",
                Measure::new("C1", "temps de chargement", "ms", Some(987.0)),
            ),
            row(
                "store",
                "stream",
                Measure::new("M2r", "rapport", "×", Some(1.5)),
            ),
            row(
                "store",
                "stream",
                Measure::new("M2", "mémoire", "Mio", Some(150.0)),
            ),
        ];
        let text = markdown("reseau.json", &rows);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "## reseau.json");
        assert_eq!(
            lines[4],
            "| C1 | temps de chargement | Store | flux | 1234 | ms |"
        );
        assert_eq!(
            lines[5],
            "| C1 | temps de chargement | V0 (serde_json::Value) | flux | 987 | ms |"
        );
        assert!(lines[6].starts_with("| M2 |") && lines[7].starts_with("| M2r |"));
        assert_eq!(
            lines[8],
            "| A1 | lecture au hasard | V0 (serde_json::Value) | flux | 3.21 | ns |"
        );
        assert_eq!(lines.len(), 9);
    }

    #[test]
    fn small_values_keep_three_digits() {
        assert_eq!(format_value(1234.5), "1234");
        assert_eq!(format_value(12.5), "12.5");
        assert_eq!(format_value(0.0183), "0.0183");
        assert_eq!(format_value(0.0004), "0.000400");
    }

    /// Scenario "Mémoire indisponible".
    #[test]
    fn report_with_memory_unavailable() {
        let rows = vec![
            row(
                "store",
                "stream",
                Measure::new("M1", "pic de mémoire", "Mio", None),
            ),
            row(
                "store",
                "stream",
                Measure::new("C1", "temps de chargement", "ms", Some(5.0)),
            ),
            Row {
                implementation: "v0".into(),
                mode: "buffer".into(),
                measure: None,
            },
        ];
        let text = markdown("f", &rows);
        assert!(
            text.contains("| M1 | pic de mémoire | Store | flux | non disponible | Mio |"),
            "{text}"
        );
        assert!(text.contains("| C1 | temps de chargement | Store | flux | 5 | ms |"));
        assert!(text.contains("| — | cas en échec | V0 (serde_json::Value) | tampon | échec | |"));
    }
}
