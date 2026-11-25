//! Layer 0: the metamodel (packages, classes, attributes, references, enums, data types),
//! the dynamic object storage, the modification API and revisions.
//!
//! Depends on no other Ortelia crate.

pub mod builder;
pub mod document;
pub mod fragment;
pub mod metamodel;
pub mod store;
pub mod strings;
pub mod value;

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod tests {
    /// jiff, built without its default features, still parses an RFC 3339 instant with
    /// microseconds (task 1.1).
    #[test]
    fn jiff_parses_a_timestamp_without_default_features() {
        let instant: jiff::Timestamp = "2026-03-12T10:15:00.123456Z".parse().expect("timestamp");
        assert_eq!(instant.as_microsecond(), 1_773_310_500_123_456);
    }
}
