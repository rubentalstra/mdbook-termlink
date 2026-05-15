//! Glossary [`Term`] data type and the anchor-slug algorithm it relies on.

use std::sync::OnceLock;

use regex::Regex;

/// A single glossary term and everything we know about it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Term {
    name: String,
    anchor: String,
    short_name: Option<String>,
    definition: Option<String>,
    aliases: Vec<String>,
}

impl Term {
    /// Creates a term from its glossary name. Derives the URL anchor (matching
    /// mdBook's slug algorithm) and the short form, if the name follows the
    /// `SHORT (Long Description)` pattern.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let anchor = slugify(&name);
        let short_name = extract_short_name(&name);
        Self {
            name,
            anchor,
            short_name,
            definition: None,
            aliases: Vec::new(),
        }
    }

    /// Creates a term with a definition.
    #[must_use]
    pub fn with_definition(name: impl Into<String>, definition: Option<String>) -> Self {
        let mut term = Self::new(name);
        term.definition = definition;
        term
    }

    /// Attaches a list of additional aliases to this term.
    #[must_use]
    pub fn with_aliases(mut self, aliases: Vec<String>) -> Self {
        self.aliases = aliases;
        self
    }

    /// The full term name as it appears in the glossary.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The URL anchor (lowercase, hyphenated) for linking to this term.
    #[must_use]
    pub fn anchor(&self) -> &str {
        &self.anchor
    }

    /// The optional short form parsed from `SHORT (Long Description)` names.
    #[must_use]
    pub fn short_name(&self) -> Option<&str> {
        self.short_name.as_deref()
    }

    /// The definition text, if available. Surfaced in tooltips.
    #[must_use]
    pub fn definition(&self) -> Option<&str> {
        self.definition.as_deref()
    }

    /// Every string that should match this term: the full name, the short
    /// form (if any), and every configured alias.
    #[must_use]
    pub fn searchable_forms(&self) -> Vec<&str> {
        let mut forms = vec![self.name()];
        if let Some(short) = self.short_name() {
            forms.push(short);
        }
        forms.extend(self.aliases.iter().map(String::as_str));
        forms
    }
}

/// Slugifies a term name to a URL anchor, matching mdBook's algorithm:
/// lowercase, non-alphanumeric runs collapsed to single hyphens, leading and
/// trailing hyphens trimmed.
#[must_use]
pub fn slugify(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut last_was_hyphen = true; // skip any leading non-alphanumerics

    for c in name.chars() {
        if c.is_alphanumeric() {
            result.push(c.to_ascii_lowercase());
            last_was_hyphen = false;
        } else if !last_was_hyphen {
            result.push('-');
            last_was_hyphen = true;
        }
    }

    if result.ends_with('-') {
        result.pop();
    }

    result
}

/// Returns the `SHORT` portion of a `SHORT (Long Description)` name.
///
/// The full name must match the shape *"non-paren text, then `(`, then
/// non-paren text, then `)`"*, with optional whitespace anywhere. Both
/// halves must be non-empty after trimming. Examples:
///
/// - `"API (Application Programming Interface)"` → `Some("API")`
/// - `"ADaM (Analysis Data Model)"` → `Some("ADaM")`
/// - `"REST"` → `None` (no parens)
/// - `"foo (bar) baz"` → `None` (trailing text past `)`)
/// - `"() (Empty)"` → `None` (left half empty)
///
/// This replaces an older length-ratio heuristic that mis-handled cases like
/// `"AAAA (BB)"`, where the short form is not strictly less than half the
/// total length.
fn extract_short_name(name: &str) -> Option<String> {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    let regex = PATTERN.get_or_init(|| {
        // `[^()]+?` = at least one non-paren character (non-greedy so the
        // `\s*\(` boundary matches the first `(`); anchored with `^…$`.
        Regex::new(r"^\s*([^()]+?)\s*\(\s*[^()]+?\s*\)\s*$")
            .expect("hard-coded short-name regex must compile")
    });

    let captures = regex.captures(name)?;
    let short = captures.get(1)?.as_str().trim();
    if short.is_empty() {
        None
    } else {
        Some(short.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_handles_common_shapes() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("API"), "api");
        assert_eq!(slugify("XPT"), "xpt");
        assert_eq!(
            slugify("ADaM (Analysis Data Model)"),
            "adam-analysis-data-model"
        );
        assert_eq!(slugify("  Spaced  Text  "), "spaced-text");
        assert_eq!(slugify("dots.and.stuff"), "dots-and-stuff");
        assert_eq!(slugify("under_score"), "under-score");
    }

    #[test]
    fn extract_short_name_table_driven() {
        // (input, expected) — None means no short form should be derived.
        let cases: &[(&str, Option<&str>)] = &[
            // Canonical positives.
            ("API (Application Programming Interface)", Some("API")),
            ("ADaM (Analysis Data Model)", Some("ADaM")),
            ("FDA (Food and Drug Administration)", Some("FDA")),
            // Previously-broken: the legacy len-ratio heuristic rejected
            // these because the short form wasn't strictly less than half
            // the total length.
            ("API (App)", Some("API")),
            ("AAAA (BB)", Some("AAAA")),
            // Tolerates whitespace at every position.
            ("  API  (  Application  )  ", Some("API")),
            // No parentheses at all.
            ("Simple Term", None),
            ("XPT", None),
            ("REST", None),
            // Trailing text past the closing paren is not a `SHORT (Long)`
            // shape and must not be matched.
            ("foo (bar) baz", None),
            // Empty halves on either side.
            ("(Empty)", None),
            ("API ()", None),
            // Nested parens are out of scope (and rare in glossary titles).
            ("X ((Y))", None),
        ];

        for (input, expected) in cases {
            assert_eq!(
                extract_short_name(input).as_deref(),
                *expected,
                "extract_short_name({input:?})"
            );
        }
    }

    #[test]
    fn term_new_populates_all_derived_fields() {
        let term = Term::new("API (Application Programming Interface)");
        assert_eq!(term.name(), "API (Application Programming Interface)");
        assert_eq!(term.anchor(), "api-application-programming-interface");
        assert_eq!(term.short_name(), Some("API"));
        assert_eq!(term.definition(), None);
    }

    #[test]
    fn term_with_definition_attaches_definition() {
        let term =
            Term::with_definition("API", Some("Application Programming Interface".to_string()));
        assert_eq!(term.definition(), Some("Application Programming Interface"));
    }

    #[test]
    fn searchable_forms_include_name_short_and_aliases() {
        let term = Term::new("API (Application Programming Interface)")
            .with_aliases(vec!["apis".to_string()]);
        let forms = term.searchable_forms();
        assert_eq!(forms.len(), 3);
        assert!(forms.contains(&"API (Application Programming Interface)"));
        assert!(forms.contains(&"API"));
        assert!(forms.contains(&"apis"));
    }

    #[test]
    fn searchable_forms_minimal_when_no_short_or_aliases() {
        let term = Term::new("XPT");
        let forms = term.searchable_forms();
        assert_eq!(forms, vec!["XPT"]);
    }
}
