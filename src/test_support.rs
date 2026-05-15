//! Shared test fixtures and helpers.
//!
//! Only compiled in `cfg(test)`. Lives here so multiple submodules can reuse
//! the same `Config` and `Glossary` constructions without duplicating boilerplate.

use std::path::PathBuf;
use std::str::FromStr;

use mdbook_preprocessor::PreprocessorContext;
use mdbook_preprocessor::config::Config as MdBookConf;

use crate::config::{Config, DisplayMode};
use crate::glossary::{Glossary, Term};

/// Builds a [`Config`] whose only non-default field is the requested
/// [`DisplayMode`]. The value round-trips through `book.toml` so the
/// kebab-case `display-mode` parsing path is exercised.
pub fn config_with_display_mode(mode: DisplayMode) -> Config {
    let mode_str = match mode {
        DisplayMode::Link => "link",
        DisplayMode::Tooltip => "tooltip",
        DisplayMode::Both => "both",
    };
    let conf_str =
        format!("[book]\ntitle = 'Test'\n[preprocessor.termlink]\ndisplay-mode = '{mode_str}'\n");
    let mdb_conf = MdBookConf::from_str(&conf_str).unwrap();
    let ctx = PreprocessorContext::new(PathBuf::new(), mdb_conf, String::new());
    Config::from_context(&ctx).unwrap()
}

/// Glossary containing the canonical API + REST + XPT terms used by linker
/// tests.
pub fn sample_glossary() -> Glossary {
    Glossary::from_terms(vec![
        Term::with_definition("API", Some("Application Programming Interface".to_string())),
        Term::with_definition("REST", Some("Representational State Transfer.".to_string())),
        Term::new("XPT"),
    ])
}
