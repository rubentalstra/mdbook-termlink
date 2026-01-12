//! Configuration parsing for the termlink preprocessor.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use glob::Pattern;
use mdbook_preprocessor::PreprocessorContext;
use serde::Deserialize;

/// Configuration for the termlink preprocessor.
///
/// All fields are private to allow future changes without breaking the API.
/// Use the getter methods to access configuration values.
#[derive(Debug, Clone)]
pub struct Config {
    /// Path to the glossary file relative to src directory.
    glossary_path: PathBuf,
    /// Whether to only link the first occurrence of each term per page.
    link_first_only: bool,
    /// CSS class to apply to glossary term links.
    css_class: String,
    /// Whether term matching should be case-sensitive.
    case_sensitive: bool,
    /// Glob patterns for pages to exclude from term linking.
    exclude_pages: Vec<Pattern>,
    /// Additional aliases for terms (term name -> list of aliases).
    aliases: HashMap<String, Vec<String>>,
}

/// Raw configuration as deserialized from book.toml.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
struct RawConfig {
    glossary_path: Option<String>,
    link_first_only: Option<bool>,
    css_class: Option<String>,
    case_sensitive: Option<bool>,
    exclude_pages: Option<Vec<String>>,
    aliases: Option<HashMap<String, Vec<String>>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            glossary_path: PathBuf::from("reference/glossary.md"),
            link_first_only: true,
            css_class: String::from("glossary-term"),
            case_sensitive: false,
            exclude_pages: Vec::new(),
            aliases: HashMap::new(),
        }
    }
}

impl Config {
    /// Creates configuration from the preprocessor context.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration in `book.toml` is malformed.
    pub fn from_context(ctx: &PreprocessorContext) -> Result<Self> {
        // Get all preprocessor configs as a BTreeMap
        let preprocessors: std::collections::BTreeMap<String, RawConfig> = ctx
            .config
            .preprocessors()
            .context("Failed to parse preprocessor configuration")?;

        // Get the termlink config, or use defaults
        let raw = preprocessors.get("termlink").cloned().unwrap_or_default();

        // Parse exclude-pages glob patterns with warnings for invalid patterns
        let exclude_pages: Vec<Pattern> = raw
            .exclude_pages
            .unwrap_or_default()
            .iter()
            .filter_map(|p| match Pattern::new(p) {
                Ok(pattern) => Some(pattern),
                Err(e) => {
                    log::warn!("Invalid exclude-pages glob pattern '{p}': {e}");
                    None
                }
            })
            .collect();

        Ok(Self {
            glossary_path: raw
                .glossary_path
                .map_or_else(|| PathBuf::from("reference/glossary.md"), PathBuf::from),
            link_first_only: raw.link_first_only.unwrap_or(true),
            css_class: raw
                .css_class
                .unwrap_or_else(|| String::from("glossary-term")),
            case_sensitive: raw.case_sensitive.unwrap_or(false),
            exclude_pages,
            aliases: raw.aliases.unwrap_or_default(),
        })
    }

    /// Returns the path to the glossary file.
    #[must_use]
    pub fn glossary_path(&self) -> &Path {
        &self.glossary_path
    }

    /// Returns true if only the first occurrence of each term should be linked.
    #[must_use]
    pub const fn link_first_only(&self) -> bool {
        self.link_first_only
    }

    /// Returns the CSS class to apply to glossary term links.
    #[must_use]
    pub fn css_class(&self) -> &str {
        &self.css_class
    }

    /// Returns true if term matching should be case-sensitive.
    #[must_use]
    pub const fn case_sensitive(&self) -> bool {
        self.case_sensitive
    }

    /// Checks if the given path is the glossary file.
    #[must_use]
    pub fn is_glossary_path(&self, path: &Path) -> bool {
        path == self.glossary_path || path.ends_with(&self.glossary_path)
    }

    /// Checks if the given path should be excluded from term linking.
    #[must_use]
    pub fn should_exclude(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        self.exclude_pages.iter().any(|p| p.matches(&path_str))
    }

    /// Returns aliases for a term name (if configured).
    #[must_use]
    pub fn aliases(&self, term_name: &str) -> Option<&Vec<String>> {
        self.aliases.get(term_name)
    }

    /// Returns iterator over all aliases (for conflict detection).
    pub fn all_aliases(&self) -> impl Iterator<Item = (&String, &Vec<String>)> {
        self.aliases.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.glossary_path(), Path::new("reference/glossary.md"));
        assert!(config.link_first_only());
        assert_eq!(config.css_class(), "glossary-term");
        assert!(!config.case_sensitive());
    }

    #[test]
    fn test_is_glossary_path_exact_match() {
        let config = Config::default();
        assert!(config.is_glossary_path(Path::new("reference/glossary.md")));
    }

    #[test]
    fn test_is_glossary_path_suffix_match() {
        let config = Config::default();
        assert!(config.is_glossary_path(Path::new("src/reference/glossary.md")));
    }

    #[test]
    fn test_is_glossary_path_no_match() {
        let config = Config::default();
        assert!(!config.is_glossary_path(Path::new("chapter1.md")));
        assert!(!config.is_glossary_path(Path::new("glossary.md")));
    }

    #[test]
    fn test_should_exclude_exact_match() {
        let config = Config {
            exclude_pages: vec![Pattern::new("changelog.md").unwrap()],
            ..Default::default()
        };
        assert!(config.should_exclude(Path::new("changelog.md")));
        assert!(!config.should_exclude(Path::new("chapter1.md")));
    }

    #[test]
    fn test_should_exclude_wildcard() {
        let config = Config {
            exclude_pages: vec![Pattern::new("appendix/*").unwrap()],
            ..Default::default()
        };
        assert!(config.should_exclude(Path::new("appendix/a.md")));
        assert!(config.should_exclude(Path::new("appendix/b.md")));
        assert!(!config.should_exclude(Path::new("chapter1.md")));
    }

    #[test]
    fn test_should_exclude_recursive_glob() {
        let config = Config {
            exclude_pages: vec![Pattern::new("**/draft-*.md").unwrap()],
            ..Default::default()
        };
        assert!(config.should_exclude(Path::new("draft-intro.md")));
        assert!(config.should_exclude(Path::new("chapters/draft-chapter1.md")));
        assert!(!config.should_exclude(Path::new("chapter1.md")));
    }

    #[test]
    fn test_aliases_getter() {
        let mut aliases = HashMap::new();
        aliases.insert(
            "API".to_string(),
            vec!["apis".to_string(), "api endpoint".to_string()],
        );
        let config = Config {
            aliases,
            ..Default::default()
        };

        assert_eq!(
            config.aliases("API"),
            Some(&vec!["apis".to_string(), "api endpoint".to_string()])
        );
        assert_eq!(config.aliases("REST"), None);
    }

    #[test]
    fn test_all_aliases_iterator() {
        let mut aliases = HashMap::new();
        aliases.insert("API".to_string(), vec!["apis".to_string()]);
        aliases.insert("REST".to_string(), vec!["RESTful".to_string()]);
        let config = Config {
            aliases,
            ..Default::default()
        };

        assert_eq!(config.all_aliases().count(), 2);
    }
}
