//! Walk parsed-markdown events for one chapter and inject glossary term links.

mod matcher;
mod path;
mod render;

pub use path::calculate_relative_path;

use std::collections::HashSet;

use pulldown_cmark::{CowStr, Event, Options, Parser, Tag, TagEnd};
use pulldown_cmark_to_cmark::cmark;

use crate::config::Config;
use crate::error::Result;
use crate::glossary::{Glossary, Term};

/// Adds glossary term links to a chapter's markdown content and returns the
/// result.
///
/// # Errors
///
/// Returns [`crate::error::TermlinkError::MarkdownSerialize`] if the processed
/// event stream cannot be reserialized.
pub fn add_term_links(
    content: &str,
    glossary: &Glossary,
    glossary_relative_path: &str,
    config: &Config,
) -> Result<String> {
    let terms: Vec<&Term> = glossary.iter().collect();
    let mut linked_terms: HashSet<String> = HashSet::new();

    let parser = Parser::new_ext(content, markdown_options());
    let events: Vec<Event> = parser.collect();

    let processed_events = process_events(
        events,
        &terms,
        glossary_relative_path,
        config,
        &mut linked_terms,
    );

    let mut output = String::new();
    cmark(processed_events.into_iter(), &mut output)?;
    Ok(output)
}

fn markdown_options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_DEFINITION_LIST);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    // GFM enables blockquote alerts (`> [!NOTE]` etc.) so they round-trip
    // through parse → serialize as `Tag::BlockQuote(Some(kind))` instead of
    // being flattened to plain blockquote text. See issue #6.
    options.insert(Options::ENABLE_GFM);
    options
}

/// The kind of element currently surrounding the cursor in the event stream.
///
/// Only [`Context::Normal`] is safe to inject links into. Everything else
/// (code, links, headings, image alt-text) is passed through verbatim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Context {
    Normal,
    CodeBlock,
    Link,
    Heading,
    Image,
}

/// Returns the context that *opens* on this event, if any.
const fn context_opened_by(event: &Event<'_>) -> Option<Context> {
    match event {
        Event::Start(Tag::CodeBlock(_)) => Some(Context::CodeBlock),
        Event::Start(Tag::Link { .. }) => Some(Context::Link),
        Event::Start(Tag::Image { .. }) => Some(Context::Image),
        Event::Start(Tag::Heading { .. }) => Some(Context::Heading),
        _ => None,
    }
}

/// Whether this event closes one of the contexts [`context_opened_by`] opens.
const fn closes_context(event: &Event<'_>) -> bool {
    matches!(
        event,
        Event::End(TagEnd::CodeBlock | TagEnd::Link | TagEnd::Image | TagEnd::Heading(_))
    )
}

/// Walks the parser events, pushing/popping context as we cross protected
/// regions, and rewrites text in safe regions into a sequence of text + html
/// events that include glossary links.
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
        if let Some(ctx) = context_opened_by(&event) {
            context_stack.push(ctx);
            result.push(event);
            continue;
        }
        if closes_context(&event) {
            context_stack.pop();
            result.push(event);
            continue;
        }

        if let Event::Text(text) = &event {
            let current = context_stack.last().copied().unwrap_or(Context::Normal);
            if current == Context::Normal {
                // Safe region — rewrite text into text + html events.
                result.extend(replace_terms_to_events(
                    text,
                    terms,
                    glossary_path,
                    config,
                    linked_terms,
                ));
                continue;
            }
        }

        // Inline code, protected text inside code/link/heading, or any other
        // event we don't transform: pass through unchanged.
        result.push(event);
    }

    result
}

/// Replaces term occurrences in `text` with HTML link events.
///
/// Returns a sequence of [`Event::Text`] and [`Event::Html`] events so that
/// HTML stays in its own event — wrapping mixed content in a single
/// `Event::Html` would confuse mdBook's renderer (see commit history for #4).
fn replace_terms_to_events(
    text: &str,
    terms: &[&Term],
    glossary_path: &str,
    config: &Config,
    linked_terms: &mut HashSet<String>,
) -> Vec<Event<'static>> {
    let mut matches: Vec<(usize, usize, String)> = Vec::new();

    for term in terms {
        if config.link_first_only() && linked_terms.contains(term.anchor()) {
            continue;
        }
        let Some(regex) = matcher::build_term_regex(term, config.case_sensitive()) else {
            continue;
        };
        if let Some(mat) = regex.find(text) {
            let matched_text = &text[mat.start()..mat.end()];
            let html = render::render_term_html(term, matched_text, glossary_path, config);
            matches.push((mat.start(), mat.end(), html));
            linked_terms.insert(term.anchor().to_string());
        }
    }

    matches.sort_by_key(|(start, _, _)| *start);

    let mut events = Vec::new();
    let mut last_end = 0;

    for (start, end, link) in matches {
        if start < last_end {
            // Overlapping match — skip.
            continue;
        }
        if start > last_end {
            events.push(Event::Text(CowStr::from(text[last_end..start].to_string())));
        }
        events.push(Event::Html(CowStr::from(link)));
        last_end = end;
    }

    if last_end < text.len() {
        events.push(Event::Text(CowStr::from(text[last_end..].to_string())));
    }
    if events.is_empty() {
        events.push(Event::Text(CowStr::from(text.to_string())));
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DisplayMode;
    use crate::test_support::{config_with_display_mode, sample_glossary};

    fn default_config() -> Config {
        Config::default()
    }

    #[test]
    fn snapshot_full_chapter_link_mode() {
        let input = "\
The API is great. We also use REST for transport.

```rust
let api = ApiClient::new();
```

Inline `API` should not link. Visit [the API docs](docs.html).

> [!NOTE]
> The API handles auth, while XPT is a legacy format.
";
        let output = add_term_links(
            input,
            &sample_glossary(),
            "glossary.html",
            &config_with_display_mode(DisplayMode::Link),
        )
        .unwrap();
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_full_chapter_tooltip_mode() {
        let input = "The API and REST are linked in tooltip mode.";
        let output = add_term_links(
            input,
            &sample_glossary(),
            "glossary.html",
            &config_with_display_mode(DisplayMode::Tooltip),
        )
        .unwrap();
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_full_chapter_both_mode() {
        let input = "The API and REST are linked in both mode.";
        let output = add_term_links(
            input,
            &sample_glossary(),
            "glossary.html",
            &config_with_display_mode(DisplayMode::Both),
        )
        .unwrap();
        insta::assert_snapshot!(output);
    }

    #[test]
    fn link_first_only_links_only_the_first_occurrence() {
        let term = Term::new("XPT");
        let terms = vec![&term];
        let config = default_config();
        let mut linked = HashSet::new();

        let events = replace_terms_to_events(
            "XPT is great. XPT is used.",
            &terms,
            "g.html",
            &config,
            &mut linked,
        );
        let rendered = events_to_string(&events);

        assert!(rendered.contains(r#"<a href="g.html#xpt""#));
        assert_eq!(rendered.matches("glossary-term").count(), 1);
    }

    #[test]
    fn alias_matches_and_links_to_canonical_anchor() {
        let term = Term::new("REST").with_aliases(vec!["RESTful".to_string()]);
        let terms = vec![&term];
        let mut linked = HashSet::new();

        let events = replace_terms_to_events(
            "This is a RESTful service.",
            &terms,
            "glossary.html",
            &default_config(),
            &mut linked,
        );
        let rendered = events_to_string(&events);

        assert!(rendered.contains(r#"<a href="glossary.html#rest""#));
        assert!(rendered.contains("RESTful</a>"));
    }

    #[test]
    fn admonition_marker_is_preserved_for_every_kind() {
        let term = Term::new("API");
        let glossary = Glossary::from_terms(vec![term]);
        let config = default_config();

        for kind in ["NOTE", "TIP", "IMPORTANT", "WARNING", "CAUTION"] {
            let input = format!("> [!{kind}]\n> Use the API carefully.\n");
            let out = add_term_links(&input, &glossary, "glossary.html", &config)
                .unwrap_or_else(|e| panic!("add_term_links failed for {kind}: {e}"));

            assert!(
                out.contains(&format!("[!{kind}]")),
                "alert marker [!{kind}] lost in output:\n{out}"
            );
            assert!(
                out.contains(r#"<a href="glossary.html#api""#),
                "termlink missing inside [!{kind}] body:\n{out}"
            );
        }
    }

    #[test]
    fn term_overlapping_admonition_marker_text_does_not_corrupt_marker() {
        let glossary = Glossary::from_terms(vec![Term::new("NOTE")]);
        let out = add_term_links(
            "> [!NOTE]\n> Read this NOTE.\n",
            &glossary,
            "g.html",
            &default_config(),
        )
        .unwrap();
        assert!(out.contains("[!NOTE]"), "alert marker dropped: {out}");
    }

    fn events_to_string(events: &[Event]) -> String {
        events
            .iter()
            .map(|e| match e {
                Event::Text(s) | Event::Html(s) => s.to_string(),
                _ => String::new(),
            })
            .collect()
    }
}
