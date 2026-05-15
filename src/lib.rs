//! # mdbook-termlink
//!
//! An [mdBook](https://github.com/rust-lang/mdBook) preprocessor that
//! automatically links every glossary term throughout the book.
//!
//! ## Quick start
//!
//! Add to your `book.toml`:
//!
//! ```toml
//! [preprocessor.termlink]
//! glossary-path = "reference/glossary.md"
//! display-mode  = "link"     # or "tooltip", or "both"
//! ```
//!
//! Write the glossary as a Markdown definition list:
//!
//! ```markdown
//! API (Application Programming Interface)
//! : A set of protocols for building software.
//!
//! REST
//! : Representational State Transfer.
//! ```
//!
//! Build the book with `mdbook build`. Every chapter will have its terms
//! linked into the glossary.
//!
//! ## Library entry points
//!
//! The public API is intentionally narrow:
//!
//! - [`TermlinkPreprocessor`] — the `Preprocessor` trait implementation.
//! - [`Config`] — parsed `book.toml` settings, including [`DisplayMode`].
//! - [`TermlinkError`] — the typed error returned by every fallible operation.

mod config;
mod error;
mod glossary;
mod linker;
mod preprocessor;

#[cfg(test)]
mod test_support;

pub use config::{Config, DisplayMode};
pub use error::TermlinkError;
pub use preprocessor::TermlinkPreprocessor;
