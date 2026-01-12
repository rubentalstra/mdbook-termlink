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
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", path.display()))
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

    // "apis" (lowercase plural) should link to #api anchor
    // This tests that aliases work correctly
    assert!(
        html.contains("reference/glossary.html#api"),
        "Alias 'apis' should link to API term anchor"
    );

    // "RESTful" should link to #rest anchor
    assert!(
        html.contains("reference/glossary.html#rest"),
        "Alias 'RESTful' should link to REST term anchor"
    );
}
