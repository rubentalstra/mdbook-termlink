//! Integration tests that exercise only the public API of `mdbook-termlink`.

use std::path::PathBuf;

use mdbook_termlink::{Config, DisplayMode};

#[test]
fn config_default_values_match_documented_defaults() {
    let config = Config::default();

    assert_eq!(
        config.glossary_path(),
        PathBuf::from("reference/glossary.md").as_path()
    );
    assert!(config.link_first_only());
    assert_eq!(config.css_class(), "glossary-term");
    assert!(!config.case_sensitive());
    assert_eq!(config.display_mode(), DisplayMode::Link);
}

#[test]
fn glossary_path_matches_both_exact_and_suffix_paths() {
    let config = Config::default();

    assert!(config.is_glossary_path(&PathBuf::from("reference/glossary.md")));
    assert!(config.is_glossary_path(&PathBuf::from("src/reference/glossary.md")));
    assert!(!config.is_glossary_path(&PathBuf::from("chapter1.md")));
    assert!(!config.is_glossary_path(&PathBuf::from("glossary.md")));
}
