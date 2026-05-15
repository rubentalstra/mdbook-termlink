//! End-to-end tests that run mdBook with the preprocessor.
//!
//! These tests require mdBook to be installed: `cargo install mdbook`

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

/// Cached book build result - only build once per test run
static BOOK_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Get path to the built preprocessor binary
fn preprocessor_binary() -> PathBuf {
    // In tests, the binary is in target/debug/ or target/release/
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");

    // Check debug first (matches default cargo build/test), then release
    let debug_path = path.join("debug").join(binary_name());
    if debug_path.exists() {
        return debug_path;
    }

    path.join("release").join(binary_name())
}

const fn binary_name() -> &'static str {
    if cfg!(windows) {
        "mdbook-termlink.exe"
    } else {
        "mdbook-termlink"
    }
}

/// Get path to test fixtures
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// Build the test book using mdBook (cached - only runs once)
fn get_book_dir() -> &'static PathBuf {
    BOOK_DIR.get_or_init(|| {
        let fixtures = fixtures_dir();
        let book_dir = fixtures.join("book");

        // Clean previous build
        if book_dir.exists() {
            fs::remove_dir_all(&book_dir).expect("Failed to clean book directory");
        }

        // Ensure preprocessor is in PATH
        let binary = preprocessor_binary();
        assert!(
            binary.exists(),
            "Preprocessor binary not found at {}. Run `cargo build` first.",
            binary.display()
        );

        let bin_dir = binary.parent().unwrap();
        let path_env = env::var("PATH").unwrap_or_default();
        let new_path = format!(
            "{}{}{}",
            bin_dir.display(),
            if cfg!(windows) { ";" } else { ":" },
            path_env
        );

        // Run mdbook build
        let output = Command::new("mdbook")
            .arg("build")
            .current_dir(&fixtures)
            .env("PATH", &new_path)
            .output()
            .expect("Failed to run mdbook. Is mdBook installed? Run: cargo install mdbook");

        assert!(
            output.status.success(),
            "mdbook build failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        book_dir
    })
}

/// Read HTML file content
fn read_html(relative_path: &str) -> String {
    let book_dir = get_book_dir();
    let path = book_dir.join(relative_path);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read {}: {e}", path.display()))
}

// =============================================================================
// Test 1: Basic Term Linking
// =============================================================================

#[test]
fn test_e2e_basic_term_linking() {
    let html = read_html("chapter1.html");

    // Verify glossary-term class is present
    assert!(
        html.contains(r#"class="glossary-term""#),
        "Expected glossary-term class in chapter1.html"
    );

    // Verify link points to correct glossary anchor
    assert!(
        html.contains("reference/glossary.html#api"),
        "Expected link to glossary API anchor"
    );
}

// =============================================================================
// Test 2: Tooltip Preview
// =============================================================================

#[test]
fn test_e2e_tooltip_preview() {
    let html = read_html("chapter1.html");

    // Verify title attribute exists (for tooltip)
    assert!(
        html.contains(r"title="),
        "Expected title attribute for tooltip preview"
    );

    // Definition text should be in the title (from glossary)
    // "A set of protocols and tools for building software applications."
    assert!(
        html.contains("protocols") && html.contains("title="),
        "Expected definition text in title attribute"
    );
}

// =============================================================================
// Test 3: Code Block Exclusion
// =============================================================================

#[test]
fn test_e2e_code_block_exclusion() {
    let html = read_html("chapter_with_code.html");

    // The file has "API" in code blocks - these should NOT be linked
    // Find all <pre><code> sections and verify no glossary-term inside
    for (start, _) in html.match_indices("<pre>") {
        if let Some(end) = html[start..].find("</pre>") {
            let code_block = &html[start..start + end];
            assert!(
                !code_block.contains("glossary-term"),
                "Terms inside code blocks should NOT be linked:\n{code_block}"
            );
        }
    }
}

// =============================================================================
// Test 4: Inline Code Exclusion
// =============================================================================

#[test]
fn test_e2e_inline_code_exclusion() {
    let html = read_html("chapter_with_code.html");

    // Inline code like `REST` should not be linked
    // The pattern <code>REST</code> should NOT have glossary-term
    assert!(
        !html.contains(r"<code><a"),
        "Inline code should not contain glossary links"
    );
}

// =============================================================================
// Test 5: Link-First-Only
// =============================================================================

#[test]
fn test_e2e_link_first_only() {
    let html = read_html("chapter1.html");

    // Count occurrences of links to API anchor
    // With link-first-only=true, should be exactly 1
    let api_link_count = html.matches("reference/glossary.html#api").count();

    assert_eq!(
        api_link_count, 1,
        "Expected exactly 1 API link (link-first-only=true), found {api_link_count}"
    );
}

// =============================================================================
// Test 6: Nested Chapter Relative Paths
// =============================================================================

#[test]
fn test_e2e_nested_chapter_relative_paths() {
    let html = read_html("nested/chapter2.html");

    // Nested chapter (one level deep) should use "../reference/glossary.html"
    assert!(
        html.contains("../reference/glossary.html#"),
        "Nested chapter should have correct relative path to glossary"
    );
}

// =============================================================================
// Test 7: Heading Exclusion
// =============================================================================

#[test]
fn test_e2e_heading_exclusion() {
    let html = read_html("chapter1.html");

    // Headings contain terms like "REST" but should NOT be linked
    // This preserves the table of contents
    // Look for <h2> tags and verify no glossary-term inside
    for (start, _) in html.match_indices("<h2") {
        if let Some(end) = html[start..].find("</h2>") {
            let heading = &html[start..start + end + 5];
            assert!(
                !heading.contains("glossary-term"),
                "Terms in headings should NOT be linked: {heading}"
            );
        }
    }
}

// =============================================================================
// Test 8: CLI Support Check (html)
// =============================================================================

#[test]
fn test_e2e_cli_supports_html() {
    let binary = preprocessor_binary();

    let output = Command::new(&binary)
        .args(["supports", "html"])
        .output()
        .expect("Failed to run preprocessor binary");

    assert!(
        output.status.success(),
        "Preprocessor should support html renderer (exit 0)"
    );
}

// =============================================================================
// Test 9: CLI Support Check (pdf - rejected)
// =============================================================================

#[test]
fn test_e2e_cli_rejects_pdf() {
    let binary = preprocessor_binary();

    let output = Command::new(&binary)
        .args(["supports", "pdf"])
        .output()
        .expect("Failed to run preprocessor binary");

    assert!(
        !output.status.success(),
        "Preprocessor should reject pdf renderer (exit non-zero)"
    );
}

// =============================================================================
// Test 10: Exclude Pages
// =============================================================================

#[test]
fn test_e2e_exclude_pages() {
    let html = read_html("excluded.html");

    // Excluded page should NOT have any glossary links
    // even though it contains terms like "API" and "REST"
    assert!(
        !html.contains("glossary-term"),
        "Excluded page should not have any glossary links"
    );
}

// =============================================================================
// Test 11: Alias Linking
// =============================================================================

#[test]
fn test_e2e_alias_linking() {
    let html = read_html("chapter_with_aliases.html");

    // The `apis` alias must be wrapped in a glossary link (not merely appear
    // as the URL fragment of some other link). This catches the historical
    // bug where short-name alias keys silently failed for parenthesized
    // glossary entries.
    assert!(
        html.contains(r">apis</a>"),
        "Alias 'apis' should be wrapped in a glossary link.\nHTML:\n{html}"
    );

    // "RESTful" should link to #rest anchor.
    assert!(
        html.contains(r">RESTful</a>"),
        "Alias 'RESTful' should be wrapped in a glossary link.\nHTML:\n{html}"
    );

    // The chapter contains an `API endpoints` substring too, which is another
    // configured alias. `link-first-only` means only the first match wins —
    // verify exactly one glossary link points at the API anchor.
    let api_link_count = html
        .matches("glossary.html#api-application-programming-interface")
        .count();
    assert_eq!(
        api_link_count, 1,
        "Expected exactly one link to the API anchor (link-first-only=true), found {api_link_count}"
    );
}

// =============================================================================
// Test 12: Admonitions (GitHub-style alerts) - issue #6
// =============================================================================

#[test]
fn test_e2e_admonitions_render_with_alert_markup() {
    let html = read_html("chapter_with_admonitions.html");

    // mdBook emits alerts as a <blockquote> carrying a kind-specific class.
    // Current mdBook (0.5.x) uses `blockquote-tag-{kind}` (e.g.
    // `blockquote-tag blockquote-tag-note`). We accept either that naming or
    // the GitHub-style `markdown-alert-{kind}` as a tolerant forward-compat
    // hedge — what matters is that the alert kind survives the preprocessor.
    let lower = html.to_lowercase();
    for kind in ["note", "tip", "important", "warning", "caution"] {
        let blockquote_tag = format!("blockquote-tag-{kind}");
        let markdown_alert = format!("markdown-alert-{kind}");
        assert!(
            lower.contains(&blockquote_tag) || lower.contains(&markdown_alert),
            "Expected an admonition class for `[!{}]` (looking for \
             `{blockquote_tag}` or `{markdown_alert}`) in \
             chapter_with_admonitions.html. If this fails, the preprocessor \
             likely corrupted the `[!{}]` marker.",
            kind.to_uppercase(),
            kind.to_uppercase()
        );
    }

    // Confirm the literal `[!NOTE]` marker text did NOT leak through to the
    // rendered HTML — if it did, mdBook treated it as plain blockquote text
    // instead of an alert.
    assert!(
        !html.contains("[!NOTE]") && !html.contains("[!WARNING]"),
        "Found a literal `[!KIND]` marker in rendered HTML — alert was not \
         recognized by mdBook (preprocessor likely broke the marker):\n{html}"
    );
}

#[test]
fn test_e2e_termlink_inside_admonition_body() {
    let html = read_html("chapter_with_admonitions.html");

    // Termlinks must still be injected inside alert bodies — admonitions are
    // not a "skip" context, only the marker line needs preservation.
    assert!(
        html.contains(r#"class="glossary-term""#),
        "Expected at least one glossary-term link inside an admonition body \
         in chapter_with_admonitions.html:\n{html}"
    );
    assert!(
        html.contains("reference/glossary.html#api"),
        "Expected the API termlink inside the [!NOTE] body to resolve to the \
         glossary anchor"
    );
}

// =============================================================================
// Test 13: Split Pattern (Definition Truncation)
// =============================================================================

#[test]
fn test_e2e_split_pattern() {
    let html = read_html("chapter1.html");

    // XMPP term has a definition that uses " -- " delimiter

    // Verify the XMPP link exists
    assert!(
        html.contains("reference/glossary.html#xmpp"),
        "Expected link to XMPP glossary term"
    );

    // Verify the tooltip contains ONLY the first part (before " -- ")
    assert!(
        html.contains(r#"title="Extensible Messaging and Presence Protocol""#),
        "Expected first part of XMPP definition in tooltip (before ' -- ')"
    );

    // Verify the tooltip does NOT contain the second part (after " -- ")
    assert!(
        !html.contains("open-standard") && !html.contains("decentralized communication"),
        "Tooltip should not contain text after the ' -- ' delimiter"
    );
}
