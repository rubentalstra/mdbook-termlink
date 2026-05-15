//! Compute the relative URL from a chapter to the glossary.

use std::path::{Path, PathBuf};

/// Returns the relative URL a chapter should use to link to the glossary.
///
/// Uses [`pathdiff::diff_paths`], which gets sibling chapters right — the
/// previous hand-rolled `"../".repeat(depth)` over-generated leading `..`
/// segments when the two paths shared a common ancestor.
///
/// Paths are normalized to forward slashes so the output is a valid URL on
/// every platform (Windows path separators are not valid in HTML hrefs).
#[must_use]
pub fn calculate_relative_path(from_chapter: &Path, to_glossary: &Path) -> String {
    // `diff_paths` wants both paths interpreted relative to the same root.
    // mdBook always feeds chapter paths that are relative to `src/`, so the
    // origin for `diff_paths` is the chapter's parent directory.
    let from_dir = from_chapter
        .parent()
        .map_or_else(PathBuf::new, Path::to_path_buf);

    let rel =
        pathdiff::diff_paths(to_glossary, &from_dir).unwrap_or_else(|| to_glossary.to_path_buf());
    to_url_string(&rel)
}

/// Joins a `Path`'s components with `/`, regardless of host OS.
fn to_url_string(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_directory_yields_bare_filename() {
        assert_eq!(
            calculate_relative_path(Path::new("intro.md"), Path::new("glossary.html")),
            "glossary.html"
        );
    }

    #[test]
    fn one_level_deep_prepends_one_dotdot() {
        assert_eq!(
            calculate_relative_path(Path::new("chapter/intro.md"), Path::new("glossary.html")),
            "../glossary.html"
        );
    }

    #[test]
    fn two_levels_deep_prepends_two_dotdots() {
        assert_eq!(
            calculate_relative_path(
                Path::new("part1/chapter1/intro.md"),
                Path::new("glossary.html")
            ),
            "../../glossary.html"
        );
    }

    #[test]
    fn sibling_chapter_resolves_without_extra_dotdot() {
        // Both paths share `reference/`. The legacy implementation produced
        // `"../reference/glossary.html"`; the correct answer is just
        // `"glossary.html"`.
        assert_eq!(
            calculate_relative_path(
                Path::new("reference/intro.md"),
                Path::new("reference/glossary.html")
            ),
            "glossary.html"
        );
    }

    #[test]
    fn deeper_sibling_walks_only_far_enough_up() {
        // From `guide/api/intro.md` to `reference/glossary.html`:
        // up twice, then into `reference`.
        assert_eq!(
            calculate_relative_path(
                Path::new("guide/api/intro.md"),
                Path::new("reference/glossary.html")
            ),
            "../../reference/glossary.html"
        );
    }
}
