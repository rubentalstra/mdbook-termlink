//! # mdbook-termlink
//!
//! An mdBook preprocessor that automatically links glossary terms throughout documentation.
//!
//! ## Features
//!
//! - Parses glossary terms from definition list markdown
//! - Auto-links first occurrence of each term per page
//! - Configurable via `book.toml`
//! - Skips code blocks, inline code, existing links, and headings
//! - Supports case-insensitive matching
//! - Custom CSS class for styled links
//!
//! ## Usage
//!
//! Add to your `book.toml`:
//!
//! ```toml
//! [preprocessor.termlink]
//! glossary-path = "reference/glossary.md"
//! link-first-only = true
//! css-class = "glossary-term"
//! case-sensitive = false
//! ```
//!
//! ## Glossary Format
//!
//! Use standard markdown definition lists:
//!
//! ```markdown
//! API (Application Programming Interface)
//! : A set of protocols for building software.
//!
//! REST
//! : Representational State Transfer.
//! ```

pub mod config;
mod glossary;
mod linker;

pub use config::Config;
pub use glossary::Term;

use anyhow::{Context, Result};
use mdbook_preprocessor::book::{Book, BookItem};
use mdbook_preprocessor::{Preprocessor, PreprocessorContext};

/// mdBook preprocessor that auto-links glossary terms throughout documentation.
#[derive(Debug)]
pub struct TermlinkPreprocessor {
    config: Config,
}

impl TermlinkPreprocessor {
    /// Creates a new preprocessor instance from the given context.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration in `book.toml` is invalid.
    pub fn new(ctx: &PreprocessorContext) -> Result<Self> {
        let config = Config::from_context(ctx)?;
        Ok(Self { config })
    }
}

impl Preprocessor for TermlinkPreprocessor {
    fn name(&self) -> &'static str {
        "termlink"
    }

    fn run(&self, _ctx: &PreprocessorContext, mut book: Book) -> Result<Book> {
        // 1. Extract terms from glossary
        let terms = glossary::extract_terms(&book, &self.config)
            .context("Failed to extract glossary terms")?;

        if terms.is_empty() {
            log::warn!(
                "No glossary terms found in {}",
                self.config.glossary_path().display()
            );
            return Ok(book);
        }

        log::info!("Found {} glossary terms", terms.len());

        // 2. Calculate glossary HTML path for linking
        let glossary_html_path = glossary::get_glossary_html_path(self.config.glossary_path());

        // 3. Process each chapter
        book.for_each_mut(|item| {
            if let BookItem::Chapter(chapter) = item {
                // Skip draft chapters and the glossary itself
                let Some(chapter_path) = chapter.path.as_ref() else {
                    return;
                };

                if self.config.is_glossary_path(chapter_path) {
                    log::debug!("Skipping glossary file: {}", chapter_path.display());
                    return;
                }

                // Calculate relative path from chapter to glossary
                let relative_glossary =
                    linker::calculate_relative_path(chapter_path, &glossary_html_path);

                // Add term links
                match linker::add_term_links(
                    &chapter.content,
                    &terms,
                    &relative_glossary,
                    &self.config,
                ) {
                    Ok(new_content) => {
                        chapter.content = new_content;
                    }
                    Err(e) => {
                        log::error!("Failed to process chapter {}: {e}", chapter_path.display());
                    }
                }
            }
        });

        Ok(book)
    }
}
