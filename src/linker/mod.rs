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

    // An empty `glossary_relative_path` is the orchestrator's signal that we
    // are *on* the glossary page itself. In that mode, definition-list titles
    // are a skip region so terms don't self-link to themselves, and the
    // rendered hrefs become bare `#anchor` (same-page) URLs.
    let on_glossary_page = glossary_relative_path.is_empty();

    let parser = Parser::new_ext(content, markdown_options());
    let events: Vec<Event> = parser.collect();

    let processed_events = process_events(
        events,
        &terms,
        glossary_relative_path,
        config,
        &mut linked_terms,
        on_glossary_page,
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
/// (code, links, headings, image alt-text, and on the glossary page itself
/// definition-list titles) is passed through verbatim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Context {
    Normal,
    CodeBlock,
    Link,
    Heading,
    Image,
    /// The term title in a definition list — `API` on the line before
    /// `: A set of protocols…`. Only tracked when we are processing the
    /// glossary page itself, where linking the title would cause a term to
    /// self-link in its own definition.
    DefinitionTitle,
}

/// Returns the context that *opens* on this event, if any.
///
/// `on_glossary_page` toggles whether `Tag::DefinitionListTitle` is treated as
/// a skip region — only meaningful when the chapter being processed *is* the
/// glossary file.
const fn context_opened_by(event: &Event<'_>, on_glossary_page: bool) -> Option<Context> {
    match event {
        Event::Start(Tag::CodeBlock(_)) => Some(Context::CodeBlock),
        Event::Start(Tag::Link { .. }) => Some(Context::Link),
        Event::Start(Tag::Image { .. }) => Some(Context::Image),
        Event::Start(Tag::Heading { .. }) => Some(Context::Heading),
        Event::Start(Tag::DefinitionListTitle) if on_glossary_page => {
            Some(Context::DefinitionTitle)
        }
        _ => None,
    }
}

/// Whether this event closes one of the contexts [`context_opened_by`] opens.
const fn closes_context(event: &Event<'_>, on_glossary_page: bool) -> bool {
    matches!(
        event,
        Event::End(TagEnd::CodeBlock | TagEnd::Link | TagEnd::Image | TagEnd::Heading(_))
    ) || (on_glossary_page && matches!(event, Event::End(TagEnd::DefinitionListTitle)))
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
    on_glossary_page: bool,
) -> Vec<Event<'a>> {
    let mut result = Vec::with_capacity(events.len());
    let mut context_stack: Vec<Context> = vec![Context::Normal];

    for event in events {
        if let Some(ctx) = context_opened_by(&event, on_glossary_page) {
            context_stack.push(ctx);
            result.push(event);
            continue;
        }
        if closes_context(&event, on_glossary_page) {
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

    // -----------------------------------------------------------------
    // process-glossary feature: empty `glossary_relative_path` signals
    // "this IS the glossary page". Definition-list titles become a skip
    // region, and hrefs collapse to bare `#anchor` (same-page) URLs.
    // -----------------------------------------------------------------

    #[test]
    fn glossary_page_skips_definition_titles_but_links_body_terms() {
        // "API" appears only as a title, "REST" only in the body — verify
        // the body link is emitted and the title is left alone.
        let glossary = Glossary::from_terms(vec![
            Term::with_definition("API", Some("Application Programming Interface".to_string())),
            Term::new("REST"),
        ]);
        let content = "API\n: A protocol. See also REST.\n";
        let out = add_term_links(content, &glossary, "", &default_config()).unwrap();

        assert!(
            out.contains(r##"<a href="#rest""##),
            "REST in the definition body must be linked: {out}"
        );
        assert!(
            !out.contains(r##"href="#api""##),
            "API as a definition title must not self-link: {out}"
        );
    }

    #[test]
    fn glossary_page_emits_same_page_anchor_hrefs() {
        let glossary = Glossary::from_terms(vec![Term::new("API")]);
        let out = add_term_links(
            "Some prose mentioning the API.",
            &glossary,
            "",
            &default_config(),
        )
        .unwrap();

        assert!(
            out.contains(r##"<a href="#api""##),
            "expected bare same-page href, got: {out}"
        );
        assert!(
            !out.contains("glossary.html"),
            "no file prefix should appear in same-page anchors: {out}"
        );
    }

    #[test]
    fn glossary_page_skips_short_form_inside_title() {
        // The title row "API (Application Programming Interface)" includes
        // the short form "API" as an alternation form. Both must be skipped.
        let glossary =
            Glossary::from_terms(vec![Term::new("API (Application Programming Interface)")]);
        let content = "API (Application Programming Interface)\n: The full definition here.\n";
        let out = add_term_links(content, &glossary, "", &default_config()).unwrap();

        assert!(
            !out.contains("href=\"#"),
            "no links should be emitted inside the title: {out}"
        );
    }

    #[test]
    fn non_glossary_page_still_links_definition_titles() {
        // Outside the glossary file, definition-list titles are NOT skipped —
        // a chapter that happens to use a definition list still gets its
        // titles linked. Guards against the skip leaking off the glossary
        // page.
        let glossary = Glossary::from_terms(vec![Term::new("API")]);
        let content = "API\n: Some local definition.\n";
        let out = add_term_links(content, &glossary, "glossary.html", &default_config()).unwrap();

        assert!(
            out.contains(r#"<a href="glossary.html#api""#),
            "definition-list title on a non-glossary page must still link: {out}"
        );
    }
}
