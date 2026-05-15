//! Build the regex that locates a single term in chapter text.

use regex::{Regex, RegexBuilder};

use crate::glossary::Term;

/// Builds a word-boundary regex that matches any of the term's searchable
/// forms (full name, short form, aliases). Returns `None` if the resulting
/// pattern fails to compile (which should never happen given that every form
/// is regex-escaped).
pub(super) fn build_term_regex(term: &Term, case_sensitive: bool) -> Option<Regex> {
    let alternatives = term
        .searchable_forms()
        .iter()
        .map(|f| regex::escape(f))
        .collect::<Vec<_>>()
        .join("|");
    let pattern = format!(r"\b({alternatives})\b");

    RegexBuilder::new(&pattern)
        .case_insensitive(!case_sensitive)
        .build()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_insensitive_matches_any_casing() {
        let regex = build_term_regex(&Term::new("XPT"), false).unwrap();
        assert!(regex.is_match("The XPT format"));
        assert!(regex.is_match("The xpt format"));
        assert!(regex.is_match("The Xpt format"));
    }

    #[test]
    fn case_sensitive_only_matches_exact_casing() {
        let regex = build_term_regex(&Term::new("XPT"), true).unwrap();
        assert!(regex.is_match("The XPT format"));
        assert!(!regex.is_match("The xpt format"));
    }

    #[test]
    fn word_boundary_prevents_partial_match() {
        let regex = build_term_regex(&Term::new("API"), false).unwrap();
        assert!(regex.is_match("The API is"));
        assert!(!regex.is_match("The APIs are"));
    }

    #[test]
    fn alternation_covers_full_name_and_short_form() {
        let regex =
            build_term_regex(&Term::new("API (Application Programming Interface)"), false).unwrap();
        assert!(regex.is_match("Use the API"));
        assert!(regex.is_match("API (Application Programming Interface) is"));
    }
}
