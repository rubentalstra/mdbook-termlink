//! Typed error surface for the termlink library.
//!
//! Library functions return [`Result<T, TermlinkError>`]. The CLI in
//! `main.rs` keeps `anyhow::Result` and lets `?` widen `TermlinkError` into
//! `anyhow::Error` via the blanket `From<E: std::error::Error>` impl.

use std::path::PathBuf;

/// Result alias used throughout the library.
pub type Result<T> = std::result::Result<T, TermlinkError>;

/// Every way the termlink preprocessor can fail.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TermlinkError {
    /// The glossary file named in `book.toml` was not found in the book.
    #[error("glossary file not found: {0}")]
    GlossaryNotFound(PathBuf),

    /// `book.toml`'s `[preprocessor.termlink]` table failed to deserialize.
    #[error("failed to parse [preprocessor.termlink] configuration")]
    BadConfig(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),

    /// An alias declared in `book.toml` collides with an existing term name.
    #[error("alias '{alias}' for term '{term}' conflicts with an existing term")]
    AliasConflict {
        /// The conflicting alias as written in `book.toml`.
        alias: String,
        /// The term the alias was attached to.
        term: String,
    },

    /// `pulldown-cmark-to-cmark` failed to re-serialize the processed events.
    #[error("failed to re-serialize processed markdown")]
    MarkdownSerialize(#[from] pulldown_cmark_to_cmark::Error),
}
