//! Glossary extraction and the [`Glossary`] container.

mod parse;
mod term;

pub use term::Term;

use std::path::{Path, PathBuf};

use mdbook_preprocessor::book::{Book, BookItem};

use crate::Config;
use crate::error::{Result, TermlinkError};

/// A glossary: every term parsed out of the book's glossary file, sorted
/// longest-name-first so the linker matches multi-word terms before any of
/// their prefixes.
#[derive(Debug, Clone)]
pub struct Glossary {
    terms: Vec<Term>,
}

impl Glossary {
    /// Parses the glossary file pointed to by `config` and returns every term.
    ///
    /// # Errors
    ///
    /// Returns [`TermlinkError::GlossaryNotFound`] if the file isn't in the
    /// book.
    pub fn extract(book: &Book, config: &Config) -> Result<Self> {
        let content = find_glossary_content(book, config.glossary_path())?;
        let terms = parse::parse_definition_lists(&content, config.split_pattern());
        Ok(Self::from_terms(terms))
    }

    /// Builds a glossary from a pre-parsed term list. Sorts internally to
    /// enforce the longest-first invariant. Useful for unit tests.
    pub fn from_terms(mut terms: Vec<Term>) -> Self {
        sort_longest_first(&mut terms);
        Self { terms }
    }

    /// Applies every alias from `config` to its corresponding term, preserving
    /// the longest-first invariant.
    #[must_use]
    pub fn with_aliases(mut self, config: &Config) -> Self {
        self.terms = self
            .terms
            .into_iter()
            .map(|term| match config.aliases(term.name()) {
                Some(aliases) => term.with_aliases(aliases.clone()),
                None => term,
            })
            .collect();
        // Aliases don't change the term *name* the sort key uses, so the
        // longest-first invariant is unaffected — no resort needed.
        self
    }

    pub const fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    pub const fn len(&self) -> usize {
        self.terms.len()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Term> {
        self.terms.iter()
    }
}

/// Locates the glossary chapter's raw markdown content within `book`.
fn find_glossary_content(book: &Book, glossary_path: &Path) -> Result<String> {
    for item in book.iter() {
        if let BookItem::Chapter(chapter) = item
            && let Some(ref path) = chapter.path
            && (path == glossary_path || path.ends_with(glossary_path))
        {
            return Ok(chapter.content.clone());
        }
    }
    Err(TermlinkError::GlossaryNotFound(glossary_path.to_path_buf()))
}

/// Sorts terms in-place so the longest names come first.
fn sort_longest_first(terms: &mut [Term]) {
    terms.sort_by_key(|t| std::cmp::Reverse(t.name().len()));
}

/// Converts a markdown glossary path to its rendered HTML equivalent.
#[must_use]
pub fn get_glossary_html_path(md_path: &Path) -> PathBuf {
    md_path.with_extension("html")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_glossary_html_path_swaps_extension() {
        assert_eq!(
            get_glossary_html_path(Path::new("glossary.md")),
            PathBuf::from("glossary.html")
        );
        assert_eq!(
            get_glossary_html_path(Path::new("reference/glossary.md")),
            PathBuf::from("reference/glossary.html")
        );
    }

    #[test]
    fn sort_longest_first_orders_by_name_length_descending() {
        let mut terms = vec![Term::new("A"), Term::new("XX"), Term::new("YYY")];
        sort_longest_first(&mut terms);
        assert_eq!(
            terms.iter().map(Term::name).collect::<Vec<_>>(),
            vec!["YYY", "XX", "A"]
        );
    }

    #[test]
    fn from_terms_enforces_longest_first_invariant() {
        let glossary =
            Glossary::from_terms(vec![Term::new("A"), Term::new("LONGEST"), Term::new("MID")]);
        let names: Vec<&str> = glossary.iter().map(Term::name).collect();
        assert_eq!(names, vec!["LONGEST", "MID", "A"]);
    }

    #[test]
    fn with_aliases_attaches_configured_aliases_to_matching_terms() {
        use crate::config::DisplayMode;
        use crate::test_support::config_with_display_mode;
        use mdbook_preprocessor::PreprocessorContext;
        use mdbook_preprocessor::config::Config as MdBookConf;
        use std::str::FromStr;

        // Default config has no aliases — applying it should leave terms
        // unchanged.
        let plain = config_with_display_mode(DisplayMode::Link);
        let glossary = Glossary::from_terms(vec![Term::new("REST")]).with_aliases(&plain);
        assert!(glossary.iter().next().unwrap().searchable_forms() == vec!["REST"]);

        // Config with aliases attaches them.
        let conf_str = "[book]\ntitle='t'\n[preprocessor.termlink.aliases]\nREST = ['RESTful']\n";
        let mdb_conf = MdBookConf::from_str(conf_str).unwrap();
        let ctx = PreprocessorContext::new(PathBuf::new(), mdb_conf, String::new());
        let config = Config::from_context(&ctx).unwrap();

        let glossary = Glossary::from_terms(vec![Term::new("REST")]).with_aliases(&config);
        let forms = glossary.iter().next().unwrap().searchable_forms();
        assert!(forms.contains(&"REST"));
        assert!(forms.contains(&"RESTful"));
    }
}
