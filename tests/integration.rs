//! Integration tests for mdbook-termlink preprocessor.

use std::path::PathBuf;

use mdbook_termlink::{Config, Term};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

#[test]
fn test_term_extraction_from_definition_list() {
    let content = r"
# Glossary

API (Application Programming Interface)
: A set of protocols for building software.

REST
: Representational State Transfer.

XPT
: SAS Transport file format.
";

    let mut options = Options::empty();
    options.insert(Options::ENABLE_DEFINITION_LIST);

    let parser = Parser::new_ext(content, options);

    let mut terms: Vec<String> = Vec::new();
    let mut in_title = false;
    let mut current_title = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::DefinitionListTitle) => {
                in_title = true;
                current_title.clear();
            }
            Event::End(TagEnd::DefinitionListTitle) => {
                if in_title && !current_title.trim().is_empty() {
                    terms.push(current_title.trim().to_string());
                }
                in_title = false;
            }
            Event::Text(text) => {
                if in_title {
                    current_title.push_str(&text);
                }
            }
            _ => {}
        }
    }

    assert_eq!(terms.len(), 3);
    assert!(terms.contains(&"API (Application Programming Interface)".to_string()));
    assert!(terms.contains(&"REST".to_string()));
    assert!(terms.contains(&"XPT".to_string()));
}

#[test]
fn test_term_struct_with_short_name() {
    let term = Term::new("API (Application Programming Interface)");

    assert_eq!(term.name(), "API (Application Programming Interface)");
    assert_eq!(term.anchor(), "api-application-programming-interface");
    assert_eq!(term.short_name(), Some("API"));

    let forms = term.searchable_forms();
    assert_eq!(forms.len(), 2);
    assert!(forms.contains(&"API (Application Programming Interface)"));
    assert!(forms.contains(&"API"));
}

#[test]
fn test_term_struct_without_short_name() {
    let term = Term::new("REST");

    assert_eq!(term.name(), "REST");
    assert_eq!(term.anchor(), "rest");
    assert_eq!(term.short_name(), None);

    let forms = term.searchable_forms();
    assert_eq!(forms.len(), 1);
    assert!(forms.contains(&"REST"));
}

#[test]
fn test_config_defaults() {
    let config = Config::default();

    assert_eq!(
        config.glossary_path(),
        PathBuf::from("reference/glossary.md").as_path()
    );
    assert!(config.link_first_only());
    assert_eq!(config.css_class(), "glossary-term");
    assert!(!config.case_sensitive());
}

#[test]
fn test_glossary_path_matching() {
    let config = Config::default();

    // Exact match
    assert!(config.is_glossary_path(&PathBuf::from("reference/glossary.md")));

    // Suffix match (for src/reference/glossary.md)
    assert!(config.is_glossary_path(&PathBuf::from("src/reference/glossary.md")));

    // Non-matches
    assert!(!config.is_glossary_path(&PathBuf::from("chapter1.md")));
    assert!(!config.is_glossary_path(&PathBuf::from("glossary.md")));
}

#[test]
fn test_anchor_generation() {
    // Test cases matching mdBook's algorithm
    assert_eq!(anchor("Hello World"), "hello-world");
    assert_eq!(anchor("API"), "api");
    assert_eq!(
        anchor("ADaM (Analysis Data Model)"),
        "adam-analysis-data-model"
    );
    assert_eq!(anchor("  Spaced  Text  "), "spaced-text");
    assert_eq!(anchor("dots.and.stuff"), "dots-and-stuff");
    assert_eq!(anchor("under_score"), "under-score");
}

/// Helper to generate anchor from term name (mirrors glossary.rs logic)
fn anchor(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut last_was_hyphen = true;

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
