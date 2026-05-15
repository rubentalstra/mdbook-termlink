//! mdBook preprocessor: the `Preprocessor` trait impl and the run pipeline.

use std::collections::HashSet;

use mdbook_preprocessor::book::{Book, BookItem};
use mdbook_preprocessor::{Preprocessor, PreprocessorContext};

use crate::config::Config;
use crate::error::{Result, TermlinkError};
use crate::glossary::{self, Glossary};
use crate::linker;

/// mdBook preprocessor that auto-links glossary terms throughout documentation.
#[derive(Debug)]
pub struct TermlinkPreprocessor {
    config: Config,
}

impl TermlinkPreprocessor {
    /// Constructs the preprocessor from an mdBook preprocessor context.
    ///
    /// # Errors
    ///
    /// Returns [`TermlinkError::BadConfig`] if the `[preprocessor.termlink]`
    /// table in `book.toml` is malformed.
    pub fn new(ctx: &PreprocessorContext) -> Result<Self> {
        let config = Config::from_context(ctx)?;
        Ok(Self { config })
    }

    /// Inner fallible body of [`Preprocessor::run`]. Lives outside the trait
    /// so we can return a typed [`TermlinkError`]; the trait impl bridges it
    /// into `anyhow::Result` at exactly one place.
    fn run_inner(&self, mut book: Book) -> Result<Book> {
        let glossary = Glossary::extract(&book, &self.config)?;

        if glossary.is_empty() {
            log::warn!(
                "No glossary terms found in {}",
                self.config.glossary_path().display()
            );
            return Ok(book);
        }
        log::info!("Found {} glossary terms", glossary.len());

        validate_alias_conflicts(&glossary, &self.config)?;

        let glossary = glossary.with_aliases(&self.config);
        let glossary_html_path = glossary::get_glossary_html_path(self.config.glossary_path());

        book.for_each_mut(|item| {
            if let BookItem::Chapter(chapter) = item {
                let Some(chapter_path) = chapter.path.as_ref() else {
                    return;
                };
                if self.config.is_glossary_path(chapter_path) {
                    log::debug!("Skipping glossary file: {}", chapter_path.display());
                    return;
                }
                if self.config.should_exclude(chapter_path) {
                    log::debug!("Skipping excluded page: {}", chapter_path.display());
                    return;
                }

                let relative_glossary =
                    linker::calculate_relative_path(chapter_path, &glossary_html_path);

                match linker::add_term_links(
                    &chapter.content,
                    &glossary,
                    &relative_glossary,
                    &self.config,
                ) {
                    Ok(new_content) => chapter.content = new_content,
                    Err(e) => {
                        log::error!("Failed to process chapter {}: {e}", chapter_path.display());
                    }
                }
            }
        });

        Ok(book)
    }
}

/// Returns an error if any configured alias collides with another term's name.
fn validate_alias_conflicts(glossary: &Glossary, config: &Config) -> Result<()> {
    let term_names: HashSet<String> = glossary.iter().map(|t| t.name().to_lowercase()).collect();

    for (term, aliases) in config.all_aliases() {
        for alias in aliases {
            let alias_lower = alias.to_lowercase();
            if term_names.contains(&alias_lower) && alias_lower != term.to_lowercase() {
                return Err(TermlinkError::AliasConflict {
                    alias: alias.clone(),
                    term: term.clone(),
                });
            }
        }
    }
    Ok(())
}

impl Preprocessor for TermlinkPreprocessor {
    fn name(&self) -> &'static str {
        "termlink"
    }

    fn run(&self, _ctx: &PreprocessorContext, book: Book) -> anyhow::Result<Book> {
        // The one place where the typed library error widens into `anyhow`.
        self.run_inner(book).map_err(anyhow::Error::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glossary::Term;
    use std::collections::HashMap;

    fn glossary_with(names: &[&str]) -> Glossary {
        Glossary::from_terms(names.iter().copied().map(Term::new).collect())
    }

    fn config_with_aliases(aliases: &HashMap<String, Vec<String>>) -> Config {
        // The only way to seed aliases without changing public API is via the
        // book.toml round-trip. Build a tiny TOML snippet.
        use mdbook_preprocessor::PreprocessorContext;
        use mdbook_preprocessor::config::Config as MdBookConf;
        use std::fmt::Write;
        use std::path::PathBuf;
        use std::str::FromStr;

        let mut alias_table = String::new();
        for (term, list) in aliases {
            let formatted: Vec<String> = list.iter().map(|a| format!("'{a}'")).collect();
            writeln!(&mut alias_table, "{term} = [{}]", formatted.join(", ")).unwrap();
        }
        let toml = format!("[book]\ntitle = 't'\n[preprocessor.termlink.aliases]\n{alias_table}");
        let mdb_conf = MdBookConf::from_str(&toml).unwrap();
        let ctx = PreprocessorContext::new(PathBuf::new(), mdb_conf, String::new());
        Config::from_context(&ctx).unwrap()
    }

    #[test]
    fn validate_alias_conflicts_accepts_non_conflicting_aliases() {
        let glossary = glossary_with(&["API", "REST"]);
        let mut aliases = HashMap::new();
        aliases.insert("API".to_string(), vec!["apis".to_string()]);
        let config = config_with_aliases(&aliases);
        assert!(validate_alias_conflicts(&glossary, &config).is_ok());
    }

    #[test]
    fn validate_alias_conflicts_rejects_alias_colliding_with_other_term_name() {
        // Alias "REST" attached to API conflicts with the real "REST" term.
        let glossary = glossary_with(&["API", "REST"]);
        let mut aliases = HashMap::new();
        aliases.insert("API".to_string(), vec!["REST".to_string()]);
        let config = config_with_aliases(&aliases);
        let err = validate_alias_conflicts(&glossary, &config).unwrap_err();
        assert!(
            matches!(err, TermlinkError::AliasConflict { ref alias, .. } if alias == "REST"),
            "expected AliasConflict for 'REST', got {err:?}"
        );
    }

    #[test]
    fn validate_alias_conflicts_allows_alias_matching_its_own_term() {
        // Case-insensitively equal to the term it's attached to — not a
        // conflict, just redundant.
        let glossary = glossary_with(&["API"]);
        let mut aliases = HashMap::new();
        aliases.insert("API".to_string(), vec!["api".to_string()]);
        let config = config_with_aliases(&aliases);
        assert!(validate_alias_conflicts(&glossary, &config).is_ok());
    }

    #[test]
    fn preprocessor_name_is_termlink() {
        let config = Config::default();
        let preprocessor = TermlinkPreprocessor { config };
        assert_eq!(preprocessor.name(), "termlink");
    }
}
