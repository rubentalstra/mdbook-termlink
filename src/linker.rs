//! Term replacement logic with context tracking.

use std::collections::HashSet;
use std::path::Path;

use anyhow::Result;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use pulldown_cmark_to_cmark::cmark;
use regex::{Regex, RegexBuilder};

use crate::config::Config;
use crate::glossary::Term;

/// Adds glossary term links to chapter content.
///
/// # Errors
///
/// Returns an error if markdown reconstruction fails.
pub fn add_term_links(
    content: &str,
    terms: &[Term],
    glossary_relative_path: &str,
    config: &Config,
) -> Result<String> {
    // Build term matchers sorted by length (longest first to avoid partial matches)
    let mut sorted_terms: Vec<&Term> = terms.iter().collect();
    sorted_terms.sort_by_key(|t| std::cmp::Reverse(t.name().len()));

    // Track which terms have been linked (for link-first-only mode)
    let mut linked_terms: HashSet<String> = HashSet::new();

    // Parse content into events
    let mut options = Options::empty();
    options.insert(Options::ENABLE_DEFINITION_LIST);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(content, options);
    let events: Vec<Event> = parser.collect();

    // Process events, tracking context
    let processed_events = process_events(
        events,
        &sorted_terms,
        glossary_relative_path,
        config,
        &mut linked_terms,
    );

    // Convert back to markdown
    let mut output = String::new();
    cmark(processed_events.into_iter(), &mut output)?;

    Ok(output)
}

/// Context tracking for what kind of element we're inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Context {
    Normal,
    CodeBlock,
    Link,
    Heading,
    Image,
}

/// Processes parser events and adds term links where appropriate.
fn process_events<'a>(
    events: Vec<Event<'a>>,
    terms: &[&Term],
    glossary_path: &str,
    config: &Config,
    linked_terms: &mut HashSet<String>,
) -> Vec<Event<'a>> {
    let mut result = Vec::with_capacity(events.len());
    let mut context_stack: Vec<Context> = vec![Context::Normal];

    for event in events {
        match &event {
            // Track context changes
            Event::Start(Tag::CodeBlock(_)) => {
                context_stack.push(Context::CodeBlock);
                result.push(event);
            }
            Event::Start(Tag::Link { .. }) => {
                context_stack.push(Context::Link);
                result.push(event);
            }
            Event::Start(Tag::Image { .. }) => {
                context_stack.push(Context::Image);
                result.push(event);
            }
            Event::Start(Tag::Heading { .. }) => {
                context_stack.push(Context::Heading);
                result.push(event);
            }
            Event::End(TagEnd::CodeBlock | TagEnd::Link | TagEnd::Image | TagEnd::Heading(_)) => {
                context_stack.pop();
                result.push(event);
            }
            Event::Code(_) => {
                // Inline code - pass through unchanged
                result.push(event);
            }

            // Process text in safe contexts
            Event::Text(text) => {
                let current_context = context_stack.last().copied().unwrap_or(Context::Normal);

                if current_context == Context::Normal {
                    // Safe to process - replace terms with links
                    let processed =
                        replace_terms_in_text(text, terms, glossary_path, config, linked_terms);
                    result.push(Event::Html(processed.into()));
                } else {
                    // Inside code/link/heading - pass through unchanged
                    result.push(event);
                }
            }

            // Pass through all other events
            _ => {
                result.push(event);
            }
        }
    }

    result
}

/// Replaces term occurrences in a text string with HTML links.
fn replace_terms_in_text(
    text: &str,
    terms: &[&Term],
    glossary_path: &str,
    config: &Config,
    linked_terms: &mut HashSet<String>,
) -> String {
    let mut result = text.to_string();

    for term in terms {
        // Skip if already linked and link-first-only is enabled
        if config.link_first_only() && linked_terms.contains(term.anchor()) {
            continue;
        }

        // Build regex for this term
        let Some(regex) = build_term_regex(term, config.case_sensitive()) else {
            continue;
        };

        if let Some(mat) = regex.find(&result) {
            // Build replacement link with optional title attribute for tooltip
            let matched_text = &result[mat.start()..mat.end()];
            let title_attr = term
                .definition()
                .map(|d| format!(r#" title="{}""#, html_escape(d)))
                .unwrap_or_default();
            let link = format!(
                r#"<a href="{}#{}"{} class="{}">{}</a>"#,
                glossary_path,
                term.anchor(),
                title_attr,
                config.css_class(),
                html_escape(matched_text),
            );

            // Replace (first occurrence only if link-first-only)
            if config.link_first_only() {
                result = format!("{}{}{}", &result[..mat.start()], link, &result[mat.end()..]);
            } else {
                // Replace all occurrences - need to capture title_attr for closure
                let title_attr_clone = title_attr.clone();
                result = regex
                    .replace_all(&result, |caps: &regex::Captures| {
                        format!(
                            r#"<a href="{}#{}"{} class="{}">{}</a>"#,
                            glossary_path,
                            term.anchor(),
                            title_attr_clone,
                            config.css_class(),
                            html_escape(&caps[0]),
                        )
                    })
                    .to_string();
            }
            linked_terms.insert(term.anchor().to_string());
        }
    }

    result
}

/// Builds a regex pattern for matching a term.
fn build_term_regex(term: &Term, case_sensitive: bool) -> Option<Regex> {
    // Get all forms to match
    let forms: Vec<&str> = term.searchable_forms();

    // Escape and join with alternation
    let pattern_parts: Vec<String> = forms.iter().map(|f| regex::escape(f)).collect();

    // Word boundary pattern
    let pattern = format!(r"\b({})\b", pattern_parts.join("|"));

    RegexBuilder::new(&pattern)
        .case_insensitive(!case_sensitive)
        .build()
        .ok()
}

/// Calculates the relative path from a chapter to the glossary.
#[must_use]
pub fn calculate_relative_path(from_chapter: &Path, to_glossary: &Path) -> String {
    // Count directory depth of the chapter
    let depth = from_chapter.parent().map_or(0, |p| p.components().count());

    // Build relative path
    let prefix = "../".repeat(depth);
    format!("{}{}", prefix, to_glossary.display())
}

/// Escapes HTML special characters.
fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> Config {
        Config::default()
    }

    #[test]
    fn test_calculate_relative_path_same_dir() {
        assert_eq!(
            calculate_relative_path(Path::new("intro.md"), Path::new("glossary.html")),
            "glossary.html"
        );
    }

    #[test]
    fn test_calculate_relative_path_one_level() {
        assert_eq!(
            calculate_relative_path(Path::new("chapter/intro.md"), Path::new("glossary.html")),
            "../glossary.html"
        );
    }

    #[test]
    fn test_calculate_relative_path_two_levels() {
        assert_eq!(
            calculate_relative_path(
                Path::new("part1/chapter1/intro.md"),
                Path::new("glossary.html")
            ),
            "../../glossary.html"
        );
    }

    #[test]
    fn test_build_term_regex_case_insensitive() {
        let term = Term::new("XPT");
        let regex = build_term_regex(&term, false).unwrap();

        assert!(regex.is_match("The XPT format"));
        assert!(regex.is_match("The xpt format"));
        assert!(regex.is_match("The Xpt format"));
    }

    #[test]
    fn test_build_term_regex_case_sensitive() {
        let term = Term::new("XPT");
        let regex = build_term_regex(&term, true).unwrap();

        assert!(regex.is_match("The XPT format"));
        assert!(!regex.is_match("The xpt format"));
    }

    #[test]
    fn test_build_term_regex_word_boundary() {
        let term = Term::new("API");
        let regex = build_term_regex(&term, false).unwrap();

        assert!(regex.is_match("The API is"));
        assert!(!regex.is_match("The APIs are")); // Word boundary prevents partial match
    }

    #[test]
    fn test_build_term_regex_with_short_name() {
        let term = Term::new("API (Application Programming Interface)");
        let regex = build_term_regex(&term, false).unwrap();

        assert!(regex.is_match("Use the API"));
        assert!(regex.is_match("API (Application Programming Interface) is"));
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("a < b"), "a &lt; b");
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape(r#"say "hello""#), "say &quot;hello&quot;");
    }

    #[test]
    fn test_replace_terms_link_first_only() {
        let term = Term::new("XPT");
        let terms: Vec<&Term> = vec![&term];
        let config = default_config();
        let mut linked = HashSet::new();

        let result = replace_terms_in_text(
            "XPT is great. XPT is used.",
            &terms,
            "g.html",
            &config,
            &mut linked,
        );

        // Should only link first occurrence
        assert!(result.contains(r#"<a href="g.html#xpt""#));
        assert_eq!(result.matches("glossary-term").count(), 1);
    }

    #[test]
    fn test_replace_terms_with_tooltip() {
        let term =
            Term::with_definition("API", Some("Application Programming Interface".to_string()));
        let terms: Vec<&Term> = vec![&term];
        let config = default_config();
        let mut linked = HashSet::new();

        let result = replace_terms_in_text(
            "Use the API for data access.",
            &terms,
            "glossary.html",
            &config,
            &mut linked,
        );

        // Should include title attribute with definition
        assert!(result.contains(r#"title="Application Programming Interface""#));
        assert!(result.contains(r#"<a href="glossary.html#api""#));
        assert!(result.contains("class=\"glossary-term\""));
    }

    #[test]
    fn test_replace_terms_without_tooltip() {
        let term = Term::new("API"); // No definition
        let terms: Vec<&Term> = vec![&term];
        let config = default_config();
        let mut linked = HashSet::new();

        let result = replace_terms_in_text(
            "Use the API for data access.",
            &terms,
            "glossary.html",
            &config,
            &mut linked,
        );

        // Should NOT include title attribute
        assert!(!result.contains("title="));
        assert!(result.contains(r#"<a href="glossary.html#api""#));
    }

    #[test]
    fn test_replace_terms_with_aliases() {
        let term = Term::new("REST").with_aliases(vec!["RESTful".to_string()]);
        let terms: Vec<&Term> = vec![&term];
        let config = default_config();
        let mut linked = HashSet::new();

        let result = replace_terms_in_text(
            "This is a RESTful service.",
            &terms,
            "glossary.html",
            &config,
            &mut linked,
        );

        // Should link the alias
        assert!(result.contains(r#"<a href="glossary.html#rest""#));
        assert!(result.contains("RESTful</a>"));
    }
}
