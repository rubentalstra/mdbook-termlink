//! Render a single linked glossary occurrence to HTML, branching on
//! [`DisplayMode`].

use crate::config::{Config, DisplayMode};
use crate::glossary::Term;

/// Builds the HTML snippet that replaces one term occurrence in the source.
///
/// The shape depends on `config.display_mode()`:
///
/// | Mode    | Markup                                                          |
/// |---------|-----------------------------------------------------------------|
/// | Link    | `<a href title class>term</a>`                                  |
/// | Tooltip | `<abbr title tabindex class>term</abbr>`                        |
/// | Both    | `<a href class><abbr title tabindex>term</abbr></a>`            |
pub(super) fn render_term_html(
    term: &Term,
    matched_text: &str,
    glossary_path: &str,
    config: &Config,
) -> String {
    let title_attr = term
        .definition()
        .map(|d| format!(r#" title="{}""#, html_escape(d)))
        .unwrap_or_default();
    let css_class = config.css_class();
    let escaped = html_escape(matched_text);
    let anchor = term.anchor();

    match config.display_mode() {
        DisplayMode::Link => format!(
            r#"<a href="{glossary_path}#{anchor}"{title_attr} class="{css_class}">{escaped}</a>"#,
        ),
        DisplayMode::Tooltip => {
            format!(r#"<abbr{title_attr} tabindex="0" class="{css_class}">{escaped}</abbr>"#)
        }
        DisplayMode::Both => format!(
            r#"<a href="{glossary_path}#{anchor}" class="{css_class}"><abbr{title_attr} tabindex="0">{escaped}</abbr></a>"#,
        ),
    }
}

/// Escapes the four HTML metacharacters we ever emit into attribute values
/// and text nodes: `&`, `<`, `>`, `"`. Single quotes never appear in our
/// output because we always quote attributes with `"`.
fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::config_with_display_mode;

    fn term_with_definition() -> Term {
        Term::with_definition("API", Some("Application Programming Interface".to_string()))
    }

    #[test]
    fn html_escape_escapes_metacharacters() {
        assert_eq!(html_escape("a < b"), "a &lt; b");
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape(r#"say "hello""#), "say &quot;hello&quot;");
    }

    #[test]
    fn snapshot_link_mode() {
        let html = render_term_html(
            &term_with_definition(),
            "API",
            "glossary.html",
            &config_with_display_mode(DisplayMode::Link),
        );
        insta::assert_snapshot!(html);
    }

    #[test]
    fn snapshot_tooltip_mode() {
        let html = render_term_html(
            &term_with_definition(),
            "API",
            "glossary.html",
            &config_with_display_mode(DisplayMode::Tooltip),
        );
        insta::assert_snapshot!(html);
    }

    #[test]
    fn snapshot_both_mode() {
        let html = render_term_html(
            &term_with_definition(),
            "API",
            "glossary.html",
            &config_with_display_mode(DisplayMode::Both),
        );
        insta::assert_snapshot!(html);
    }

    #[test]
    fn snapshot_link_mode_without_definition() {
        let html = render_term_html(
            &Term::new("API"),
            "API",
            "glossary.html",
            &config_with_display_mode(DisplayMode::Link),
        );
        insta::assert_snapshot!(html);
    }
}
