//! Glossary term parsing using pulldown-cmark.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use mdbook_preprocessor::book::{Book, BookItem};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use crate::Config;

/// A glossary term extracted from a definition list.
///
/// Terms are parsed from definition list markdown format and used to
/// create links throughout the documentation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Term {
    /// The full term name as it appears in the glossary.
    name: String,
    /// The URL anchor for this term (lowercase, hyphenated).
    anchor: String,
    /// Optional short form for terms like "API (Application Programming Interface)".
    short_name: Option<String>,
    /// The definition text for this term (used for tooltip preview).
    definition: Option<String>,
    /// Additional aliases configured in book.toml.
    aliases: Vec<String>,
}

impl Term {
    /// Creates a new term with auto-generated anchor.
    ///
    /// The anchor is generated to match mdBook's algorithm:
    /// - Convert to lowercase
    /// - Replace non-alphanumeric characters with hyphens
    /// - Collapse consecutive hyphens
    /// - Trim leading/trailing hyphens
    ///
    /// If the term follows the pattern "SHORT (Long Description)", the short
    /// form is extracted for matching.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let anchor = generate_anchor(&name);
        let short_name = extract_short_name(&name);
        Self {
            name,
            anchor,
            short_name,
            definition: None,
            aliases: Vec::new(),
        }
    }

    /// Creates a new term with a definition.
    #[must_use]
    pub fn with_definition(name: impl Into<String>, definition: Option<String>) -> Self {
        let mut term = Self::new(name);
        term.definition = definition;
        term
    }

    /// Adds aliases to this term.
    #[must_use]
    pub fn with_aliases(mut self, aliases: Vec<String>) -> Self {
        self.aliases = aliases;
        self
    }

    /// Returns the full term name as it appears in the glossary.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the URL anchor for this term.
    #[must_use]
    pub fn anchor(&self) -> &str {
        &self.anchor
    }

    /// Returns the optional short form of the term.
    ///
    /// For example, "API" from "API (Application Programming Interface)".
    #[must_use]
    pub fn short_name(&self) -> Option<&str> {
        self.short_name.as_deref()
    }

    /// Returns the definition text for this term (if available).
    ///
    /// Used for tooltip preview on hover.
    #[must_use]
    pub fn definition(&self) -> Option<&str> {
        self.definition.as_deref()
    }

    /// Returns all searchable forms of this term.
    ///
    /// This includes the full name, short name (if present), and any aliases.
    #[must_use]
    pub fn searchable_forms(&self) -> Vec<&str> {
        let mut forms = vec![self.name.as_str()];
        if let Some(ref short) = self.short_name {
            forms.push(short.as_str());
        }
        forms.extend(self.aliases.iter().map(String::as_str));
        forms
    }
}

/// Extracts glossary terms from the book.
///
/// # Errors
///
/// Returns an error if the glossary file specified in the config is not found.
pub fn extract_terms(book: &Book, config: &Config) -> Result<Vec<Term>> {
    let glossary_content = find_glossary_content(book, config.glossary_path())?;
    Ok(parse_definition_lists(&glossary_content))
}

/// Finds and returns the content of the glossary chapter.
fn find_glossary_content(book: &Book, glossary_path: &Path) -> Result<String> {
    for item in book.iter() {
        if let BookItem::Chapter(chapter) = item
            && let Some(ref path) = chapter.path
            && (path == glossary_path || path.ends_with(glossary_path))
        {
            return Ok(chapter.content.clone());
        }
    }
    bail!("Glossary file not found: {}", glossary_path.display())
}

/// Parses definition lists from markdown content using pulldown-cmark.
fn parse_definition_lists(content: &str) -> Vec<Term> {
    let mut terms = Vec::new();

    // Enable definition list extension
    let mut options = Options::empty();
    options.insert(Options::ENABLE_DEFINITION_LIST);

    let parser = Parser::new_ext(content, options);

    let mut in_definition_list = false;
    let mut in_title = false;
    let mut in_definition = false;
    let mut current_title_text = String::new();
    let mut current_definition_text = String::new();
    let mut pending_title: Option<String> = None;

    for event in parser {
        match event {
            Event::Start(Tag::DefinitionList) => {
                in_definition_list = true;
            }
            Event::End(TagEnd::DefinitionList) => {
                in_definition_list = false;
                // Handle any pending term without definition
                if let Some(title) = pending_title.take()
                    && !title.is_empty()
                {
                    terms.push(Term::new(title));
                }
            }
            Event::Start(Tag::DefinitionListTitle) => {
                if in_definition_list {
                    // If we have a pending term, save it before starting a new one
                    if let Some(title) = pending_title.take()
                        && !title.is_empty()
                    {
                        let definition = if current_definition_text.trim().is_empty() {
                            None
                        } else {
                            Some(current_definition_text.trim().to_string())
                        };
                        terms.push(Term::with_definition(title, definition));
                    }
                    in_title = true;
                    current_title_text.clear();
                    current_definition_text.clear();
                }
            }
            Event::End(TagEnd::DefinitionListTitle) => {
                if in_title {
                    pending_title = Some(current_title_text.trim().to_string());
                    in_title = false;
                }
            }
            Event::Start(Tag::DefinitionListDefinition) => {
                if in_definition_list {
                    in_definition = true;
                }
            }
            Event::End(TagEnd::DefinitionListDefinition) => {
                if in_definition {
                    in_definition = false;
                    // Apply definition to pending term and save it
                    if let Some(title) = pending_title.take()
                        && !title.is_empty()
                    {
                        let definition = if current_definition_text.trim().is_empty() {
                            None
                        } else {
                            Some(current_definition_text.trim().to_string())
                        };
                        terms.push(Term::with_definition(title, definition));
                        current_definition_text.clear();
                    }
                }
            }
            Event::Text(text) | Event::Code(text) => {
                if in_title {
                    current_title_text.push_str(&text);
                } else if in_definition {
                    current_definition_text.push_str(&text);
                }
            }
            _ => {}
        }
    }

    terms
}

/// Generates a URL anchor from a term name.
///
/// Matches mdBook's anchor generation algorithm:
/// - Convert to lowercase
/// - Replace non-alphanumeric characters with hyphens
/// - Collapse consecutive hyphens
/// - Trim leading/trailing hyphens
#[must_use]
pub fn generate_anchor(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut last_was_hyphen = true; // Start true to skip leading hyphens

    for c in name.chars() {
        if c.is_alphanumeric() {
            result.push(c.to_ascii_lowercase());
            last_was_hyphen = false;
        } else if !last_was_hyphen {
            result.push('-');
            last_was_hyphen = true;
        }
    }

    // Remove trailing hyphen
    if result.ends_with('-') {
        result.pop();
    }

    result
}

/// Extracts short name from terms like "API (Application Programming Interface)".
fn extract_short_name(name: &str) -> Option<String> {
    // Look for pattern: "SHORT (Long Description)"
    let paren_idx = name.find('(')?;
    let short = name[..paren_idx].trim();

    // Only use as short name if it's actually shorter and non-empty
    if !short.is_empty() && short.len() < name.len() / 2 {
        Some(short.to_string())
    } else {
        None
    }
}

/// Converts a markdown path to its HTML equivalent.
#[must_use]
pub fn get_glossary_html_path(md_path: &Path) -> PathBuf {
    md_path.with_extension("html")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_anchor_simple() {
        assert_eq!(generate_anchor("Hello World"), "hello-world");
        assert_eq!(generate_anchor("API"), "api");
        assert_eq!(generate_anchor("XPT"), "xpt");
    }

    #[test]
    fn test_generate_anchor_with_parentheses() {
        assert_eq!(
            generate_anchor("ADaM (Analysis Data Model)"),
            "adam-analysis-data-model"
        );
        assert_eq!(
            generate_anchor("API (Application Programming Interface)"),
            "api-application-programming-interface"
        );
    }

    #[test]
    fn test_generate_anchor_special_chars() {
        assert_eq!(generate_anchor("  Spaced  Text  "), "spaced-text");
        assert_eq!(generate_anchor("dots.and.stuff"), "dots-and-stuff");
        assert_eq!(generate_anchor("under_score"), "under-score");
    }

    #[test]
    fn test_extract_short_name_with_parentheses() {
        assert_eq!(
            extract_short_name("API (Application Programming Interface)"),
            Some("API".to_string())
        );
        assert_eq!(
            extract_short_name("ADaM (Analysis Data Model)"),
            Some("ADaM".to_string())
        );
        assert_eq!(
            extract_short_name("FDA (Food and Drug Administration)"),
            Some("FDA".to_string())
        );
    }

    #[test]
    fn test_extract_short_name_none() {
        assert_eq!(extract_short_name("Simple Term"), None);
        assert_eq!(extract_short_name("XPT"), None);
        assert_eq!(extract_short_name("REST"), None);
    }

    #[test]
    fn test_term_new() {
        let term = Term::new("API (Application Programming Interface)");
        assert_eq!(term.name(), "API (Application Programming Interface)");
        assert_eq!(term.anchor(), "api-application-programming-interface");
        assert_eq!(term.short_name(), Some("API"));
        assert_eq!(term.definition(), None);
    }

    #[test]
    fn test_term_with_definition() {
        let term = Term::with_definition("API", Some("Application Programming Interface".to_string()));
        assert_eq!(term.name(), "API");
        assert_eq!(term.definition(), Some("Application Programming Interface"));
    }

    #[test]
    fn test_term_with_aliases() {
        let term = Term::new("API").with_aliases(vec!["apis".to_string(), "api endpoint".to_string()]);
        let forms = term.searchable_forms();
        assert_eq!(forms.len(), 3);
        assert!(forms.contains(&"API"));
        assert!(forms.contains(&"apis"));
        assert!(forms.contains(&"api endpoint"));
    }

    #[test]
    fn test_term_searchable_forms_with_short_name() {
        let term = Term::new("API (Application Programming Interface)");
        let forms = term.searchable_forms();
        assert_eq!(forms.len(), 2);
        assert!(forms.contains(&"API (Application Programming Interface)"));
        assert!(forms.contains(&"API"));
    }

    #[test]
    fn test_term_searchable_forms_without_short_name() {
        let term = Term::new("XPT");
        let forms = term.searchable_forms();
        assert_eq!(forms.len(), 1);
        assert!(forms.contains(&"XPT"));
    }

    #[test]
    fn test_term_searchable_forms_with_aliases() {
        let term = Term::new("REST").with_aliases(vec!["RESTful".to_string()]);
        let forms = term.searchable_forms();
        assert_eq!(forms.len(), 2);
        assert!(forms.contains(&"REST"));
        assert!(forms.contains(&"RESTful"));
    }

    #[test]
    fn test_parse_definition_lists() {
        let content = r"
# Glossary

API (Application Programming Interface)
: A set of protocols for building software.

REST
: Representational State Transfer.

XPT
: SAS Transport file format.
";
        let terms = parse_definition_lists(content);

        assert_eq!(terms.len(), 3);

        assert_eq!(terms[0].name(), "API (Application Programming Interface)");
        assert_eq!(terms[0].short_name(), Some("API"));
        assert_eq!(terms[0].anchor(), "api-application-programming-interface");
        assert_eq!(terms[0].definition(), Some("A set of protocols for building software."));

        assert_eq!(terms[1].name(), "REST");
        assert_eq!(terms[1].anchor(), "rest");
        assert_eq!(terms[1].definition(), Some("Representational State Transfer."));

        assert_eq!(terms[2].name(), "XPT");
        assert_eq!(terms[2].anchor(), "xpt");
        assert_eq!(terms[2].definition(), Some("SAS Transport file format."));
    }

    #[test]
    fn test_parse_definition_lists_empty() {
        let content = "# Just a heading\n\nSome paragraph text.";
        let terms = parse_definition_lists(content);
        assert!(terms.is_empty());
    }

    #[test]
    fn test_get_glossary_html_path() {
        assert_eq!(
            get_glossary_html_path(Path::new("glossary.md")),
            PathBuf::from("glossary.html")
        );
        assert_eq!(
            get_glossary_html_path(Path::new("reference/glossary.md")),
            PathBuf::from("reference/glossary.html")
        );
    }
}
