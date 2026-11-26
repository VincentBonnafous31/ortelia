//! The folding of texts, for the sort and the filter of tables: lowercase, then canonical
//! decomposition (NFD) and removal of the combining marks, such as accents. `Émile` and `emile`
//! fold to the same text.
//!
//! It is not a linguistic collation (ICU): it is enough for French and English, and it gives a
//! key computed once per text, which keeps the sort of 100 000 rows fast. Everything goes
//! through [`fold`], which can be replaced.

use icu_normalizer::DecomposingNormalizerBorrowed;
use icu_normalizer::properties::CanonicalCombiningClassMapBorrowed;

/// The folded form of `text`.
pub fn fold(text: &str) -> String {
    let mut folded = String::with_capacity(text.len());
    fold_into(text, &mut folded);
    folded
}

/// Appends the folded form of `text` to `out`, without allocating a string per text.
pub fn fold_into(text: &str, out: &mut String) {
    // Most texts are plain ASCII: no accent to remove, and a much cheaper lowercase.
    if text.is_ascii() {
        out.extend(text.chars().map(|c| c.to_ascii_lowercase()));
        return;
    }
    // Both are constant data compiled into the program: creating them costs nothing.
    let nfd = DecomposingNormalizerBorrowed::new_nfd();
    let classes = CanonicalCombiningClassMapBorrowed::new();
    // `flat_map`: a character may lowercase to several (`İ` gives `i` and a combining dot).
    let lowercase = text.chars().flat_map(char::to_lowercase);
    // A combining mark has a non-zero canonical combining class (230 for the acute accent).
    out.extend(
        nfd.normalize_iter(lowercase)
            .filter(|&c| classes.get_u8(c) == 0),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accents_are_removed() {
        assert_eq!(fold("Émile"), "emile");
        assert_eq!(fold("éric"), "eric");
        assert_eq!(fold("Pacsé(e)"), "pacse(e)");
        assert_eq!(fold("Noël Çà"), "noel ca");
        // A precomposed letter and its decomposed form fold alike.
        assert_eq!(fold("e\u{301}"), fold("\u{e9}"));
    }

    #[test]
    fn case_is_ignored() {
        assert_eq!(fold("de La Tour"), "de la tour");
        assert_eq!(fold("DUPONT"), fold("Dupont"));
        assert_eq!(fold("ÉLÉONORE"), "eleonore");
    }

    /// Ligatures are letters of their own: NFD does not split them (NFKD would split `ﬁ`).
    #[test]
    fn ligatures_are_kept() {
        assert_eq!(fold("Œuvre"), "œuvre");
        assert_eq!(fold("Æsop"), "æsop");
        assert_eq!(fold("ﬁn"), "ﬁn");
    }

    #[test]
    fn ascii_texts() {
        assert_eq!(fold("Marie Dupont 1985"), "marie dupont 1985");
        assert_eq!(fold(""), "");
        let mut out = String::from("a\u{1f}");
        fold_into("B", &mut out);
        assert_eq!(out, "a\u{1f}b");
    }

    /// The order of the folded texts is the one of the scenario "Accents et casse".
    #[test]
    fn folded_order() {
        let mut names = ["Émile", "de La Tour", "Dupont", "éric", "Durand"];
        names.sort_by_key(|n| fold(n));
        assert_eq!(names, ["de La Tour", "Dupont", "Durand", "Émile", "éric"]);
    }
}
