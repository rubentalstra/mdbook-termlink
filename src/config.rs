//! Configuration parsing for the termlink preprocessor.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use mdbook_preprocessor::PreprocessorContext;
use serde::Deserialize;

/// Configuration for the termlink preprocessor.
///
/// All fields are private to allow future changes without breaking the API.
/// Use the getter methods to access configuration values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Path to the glossary file relative to src directory.
    glossary_path: PathBuf,
    /// Whether to only link the first occurrence of each term per page.
    link_first_only: bool,
    /// CSS class to apply to glossary term links.
    css_class: String,
    /// Whether term matching should be case-sensitive.
    case_sensitive: bool,
}

/// Raw configuration as deserialized from book.toml.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
struct RawConfig {
    glossary_path: Option<String>,
    link_first_only: Option<bool>,
    css_class: Option<String>,
    case_sensitive: Option<bool>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            glossary_path: PathBuf::from("reference/glossary.md"),
            link_first_only: true,
            css_class: String::from("glossary-term"),
            case_sensitive: false,
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

        Ok(Self {
            glossary_path: raw
                .glossary_path
                .map_or_else(|| PathBuf::from("reference/glossary.md"), PathBuf::from),
            link_first_only: raw.link_first_only.unwrap_or(true),
            css_class: raw
                .css_class
                .unwrap_or_else(|| String::from("glossary-term")),
            case_sensitive: raw.case_sensitive.unwrap_or(false),
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
}
