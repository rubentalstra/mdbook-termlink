//! Definition-list parser. Extracts [`Term`]s from glossary markdown using
//! `pulldown-cmark`.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use super::term::Term;

/// Parses every definition-list term out of a glossary file's markdown.
///
/// `split_pattern`, when provided, truncates each definition at its first
/// occurrence — useful for short tooltips while the glossary page itself keeps
/// the full text.
pub fn parse_definition_lists(content: &str, split_pattern: Option<&str>) -> Vec<Term> {
    let mut terms = Vec::new();

    let mut options = Options::empty();
    options.insert(Options::ENABLE_DEFINITION_LIST);
    let parser = Parser::new_ext(content, options);

    let mut in_definition_list = false;
    let mut in_title = false;
    let mut in_definition = false;
    let mut current_title_text = String::new();
    let mut current_definition_text = String::new();
    let mut pending_title: Option<String> = None;

    let flush_pending =
        |pending: &mut Option<String>, definition_text: &mut String, terms: &mut Vec<Term>| {
            if let Some(title) = pending.take()
                && !title.is_empty()
            {
                let definition = if definition_text.trim().is_empty() {
                    None
                } else {
                    truncate_at(definition_text.trim(), split_pattern)
                };
                terms.push(Term::with_definition(title, definition));
                definition_text.clear();
            }
        };

    for event in parser {
        match event {
            Event::Start(Tag::DefinitionList) => {
                in_definition_list = true;
            }
            Event::End(TagEnd::DefinitionList) => {
                in_definition_list = false;
                if let Some(title) = pending_title.take()
                    && !title.is_empty()
                {
                    terms.push(Term::new(title));
                }
            }
            Event::Start(Tag::DefinitionListTitle) if in_definition_list => {
                flush_pending(&mut pending_title, &mut current_definition_text, &mut terms);
                in_title = true;
                current_title_text.clear();
                current_definition_text.clear();
            }
            Event::End(TagEnd::DefinitionListTitle) if in_title => {
                pending_title = Some(current_title_text.trim().to_string());
                in_title = false;
            }
            Event::Start(Tag::DefinitionListDefinition) if in_definition_list => {
                in_definition = true;
            }
            Event::End(TagEnd::DefinitionListDefinition) if in_definition => {
                in_definition = false;
                flush_pending(&mut pending_title, &mut current_definition_text, &mut terms);
            }
            Event::Text(text) | Event::Code(text) => {
                if in_title {
                    current_title_text.push_str(&text);
                } else if in_definition {
                    current_definition_text.push_str(&text);
                }
            }
            _ => {}
        }
    }

    terms
}

/// Truncates the definition at the first occurrence of `split_pattern`.
/// When the pattern is absent, the definition is kept whole.
fn truncate_at(definition: &str, split_pattern: Option<&str>) -> Option<String> {
    split_pattern.map_or_else(
        || Some(definition.to_string()),
        |p| definition.split(p).next().map(|d| d.trim().to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_definition_list() {
        let content = r"
# Glossary

API (Application Programming Interface)
: A set of protocols for building software.

REST
: Representational State Transfer.

XPT
: SAS Transport file format.
";
        let terms = parse_definition_lists(content, None);

        assert_eq!(terms.len(), 3);
        assert_eq!(terms[0].name(), "API (Application Programming Interface)");
        assert_eq!(terms[0].short_name(), Some("API"));
        assert_eq!(terms[0].anchor(), "api-application-programming-interface");
        assert_eq!(
            terms[0].definition(),
            Some("A set of protocols for building software.")
        );
        assert_eq!(terms[1].name(), "REST");
        assert_eq!(terms[2].definition(), Some("SAS Transport file format."));
    }

    #[test]
    fn parses_empty_content_to_no_terms() {
        assert!(parse_definition_lists("# Just a heading\n\nBody text.", None).is_empty());
    }

    #[test]
    fn split_pattern_truncates_definition() {
        let split = truncate_at(
            "Extensible Messaging and Presence Protocol -- An open-standard XML technology",
            Some(" -- "),
        );
        assert_eq!(
            split,
            Some("Extensible Messaging and Presence Protocol".to_string())
        );
    }

    #[test]
    fn split_pattern_none_keeps_definition() {
        let original =
            "Extensible Messaging and Presence Protocol -- An open-standard XML technology";
        assert_eq!(truncate_at(original, None), Some(original.to_string()));
    }
}
