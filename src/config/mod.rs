//! Configuration parsing for the termlink preprocessor.

mod display_mode;

pub use display_mode::DisplayMode;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use glob::Pattern;
use mdbook_preprocessor::PreprocessorContext;
use serde::Deserialize;

use crate::error::{Result, TermlinkError};

/// Configuration for the termlink preprocessor.
///
/// All fields are private to allow future changes without breaking the API.
/// Use the getter methods to access configuration values.
#[derive(Debug, Clone)]
pub struct Config {
    glossary_path: PathBuf,
    link_first_only: bool,
    css_class: String,
    case_sensitive: bool,
    exclude_pages: Vec<Pattern>,
    aliases: HashMap<String, Vec<String>>,
    split_pattern: Option<String>,
    display_mode: DisplayMode,
}

/// Raw configuration as deserialized from `book.toml`.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
struct RawConfig {
    glossary_path: Option<String>,
    link_first_only: Option<bool>,
    css_class: Option<String>,
    case_sensitive: Option<bool>,
    exclude_pages: Option<Vec<String>>,
    aliases: Option<HashMap<String, Vec<String>>>,
    split_pattern: Option<String>,
    display_mode: Option<String>,
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
            split_pattern: None,
            display_mode: DisplayMode::default(),
        }
    }
}

impl Config {
    /// Creates configuration from the preprocessor context.
    ///
    /// Unknown `display-mode` values fall back to the default with a
    /// `log::warn!`; invalid glob patterns under `exclude-pages` are dropped
    /// the same way. These are user-typo cases, not hard errors.
    ///
    /// # Errors
    ///
    /// Returns [`TermlinkError::BadConfig`] if `[preprocessor.termlink]` in
    /// `book.toml` fails to deserialize.
    pub fn from_context(ctx: &PreprocessorContext) -> Result<Self> {
        let preprocessors: std::collections::BTreeMap<String, RawConfig> = ctx
            .config
            .preprocessors()
            .map_err(|e| TermlinkError::BadConfig(e.into()))?;

        let raw = preprocessors.get("termlink").cloned().unwrap_or_default();

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

        let display_mode = raw
            .display_mode
            .as_deref()
            .map_or_else(DisplayMode::default, |v| {
                v.parse::<DisplayMode>().unwrap_or_else(|err| {
                    log::warn!("{err}. Falling back to 'link'.");
                    DisplayMode::default()
                })
            });

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
            split_pattern: raw.split_pattern.filter(|p| !p.is_empty()),
            display_mode,
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

    /// Returns the split delimiter for glossary definitions, if configured.
    #[must_use]
    pub fn split_pattern(&self) -> Option<&str> {
        self.split_pattern.as_deref()
    }

    /// Returns how linked terms should be rendered.
    #[must_use]
    pub const fn display_mode(&self) -> DisplayMode {
        self.display_mode
    }

    /// Returns an iterator over every configured alias (for conflict detection).
    pub fn all_aliases(&self) -> impl Iterator<Item = (&String, &Vec<String>)> {
        self.aliases.iter()
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use mdbook_preprocessor::config::Config as MdBookConf;

    fn config_from_toml(toml: &str) -> Result<Config> {
        let mdb_conf = MdBookConf::from_str(toml).unwrap();
        let ctx = PreprocessorContext::new(PathBuf::new(), mdb_conf, String::new());
        Config::from_context(&ctx)
    }

    #[test]
    fn default_config_has_expected_values() {
        let config = Config::default();
        assert_eq!(config.glossary_path(), Path::new("reference/glossary.md"));
        assert!(config.link_first_only());
        assert_eq!(config.css_class(), "glossary-term");
        assert!(!config.case_sensitive());
        assert_eq!(config.display_mode(), DisplayMode::Link);
    }

    #[test]
    fn is_glossary_path_exact_and_suffix_match() {
        let config = Config::default();
        assert!(config.is_glossary_path(Path::new("reference/glossary.md")));
        assert!(config.is_glossary_path(Path::new("src/reference/glossary.md")));
        assert!(!config.is_glossary_path(Path::new("chapter1.md")));
        assert!(!config.is_glossary_path(Path::new("glossary.md")));
    }

    #[test]
    fn should_exclude_matches_exact_wildcard_and_recursive_patterns() {
        let config = Config {
            exclude_pages: vec![
                Pattern::new("changelog.md").unwrap(),
                Pattern::new("appendix/*").unwrap(),
                Pattern::new("**/draft-*.md").unwrap(),
            ],
            ..Default::default()
        };
        assert!(config.should_exclude(Path::new("changelog.md")));
        assert!(config.should_exclude(Path::new("appendix/a.md")));
        assert!(config.should_exclude(Path::new("chapters/draft-x.md")));
        assert!(!config.should_exclude(Path::new("chapter1.md")));
    }

    #[test]
    fn aliases_getter_and_iterator() {
        let mut aliases = HashMap::new();
        aliases.insert("API".to_string(), vec!["apis".to_string()]);
        aliases.insert("REST".to_string(), vec!["RESTful".to_string()]);
        let config = Config {
            aliases,
            ..Default::default()
        };
        assert_eq!(config.aliases("API"), Some(&vec!["apis".to_string()]));
        assert_eq!(config.aliases("none"), None);
        assert_eq!(config.all_aliases().count(), 2);
    }

    #[test]
    fn empty_split_pattern_disables_splitting() {
        let conf =
            config_from_toml("[book]\ntitle = 'T'\n[preprocessor.termlink]\nsplit-pattern = ''\n")
                .unwrap();
        assert_eq!(conf.split_pattern(), None);
    }

    #[test]
    fn display_mode_parses_each_variant_from_toml() {
        for (value, expected) in [
            ("link", DisplayMode::Link),
            ("tooltip", DisplayMode::Tooltip),
            ("both", DisplayMode::Both),
        ] {
            let toml =
                format!("[book]\ntitle = 'T'\n[preprocessor.termlink]\ndisplay-mode = '{value}'\n");
            assert_eq!(config_from_toml(&toml).unwrap().display_mode(), expected);
        }
    }

    #[test]
    fn display_mode_invalid_value_falls_back_to_link() {
        for value in ["nonsense", ""] {
            let toml =
                format!("[book]\ntitle = 'T'\n[preprocessor.termlink]\ndisplay-mode = '{value}'\n");
            assert_eq!(
                config_from_toml(&toml).unwrap().display_mode(),
                DisplayMode::Link
            );
        }
    }
}
