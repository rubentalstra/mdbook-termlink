# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-05-15

### Added

- **Configurable display mode**: New `display-mode` config option lets you choose how glossary occurrences are rendered (issue [#9](https://github.com/rubentalstra/mdbook-termlink/issues/9) suggested by [@DocKDE](https://github.com/DocKDE))
  - `"link"` *(default — unchanged)*: `<a href title class>term</a>` — anchor with native browser tooltip
  - `"tooltip"`: `<abbr title tabindex class>term</abbr>` — tooltip only, no navigation, keyboard-focusable
  - `"both"`: `<a href class><abbr title tabindex>term</abbr></a>` — anchor wrapping a semantic abbreviation
  - Unknown values fall back to `"link"` with a warning
- **Typed error surface**: New `TermlinkError` enum (via `thiserror`) replaces `anyhow` at the library boundary. `anyhow` is still used inside `main.rs` only.

### Fixed

- **`extract_short_name` heuristic**: now uses a strict `SHORT (Long Description)` pattern match instead of a fragile length-ratio rule. Previously-rejected inputs like `"AAAA (BB)"` now correctly derive `"AAAA"` as the short form.
- **Relative glossary path for sibling chapters**: `pathdiff::diff_paths` replaces the hand-rolled `"../".repeat(depth)` calculation. Chapters sharing a directory with the glossary now produce a clean `glossary.html` href instead of an over-generated `../<dir>/glossary.html`.
- **Aliases keyed by a term's short form**: an alias map entry like `API = ["apis"]` now attaches to a glossary entry written as `"API (Application Programming Interface)"`. Previously the alias was silently ignored because the lookup used the full name. Full-name keys keep working and still take precedence when both are present.

### Changed

- **Public API tightened**. Library now exports only `TermlinkPreprocessor`, `Config`, `DisplayMode`, and `TermlinkError`. Types previously exported (`Term`, the bare `config` module) are now crate-private.
- **Module layout** restructured into `config/`, `error.rs`, `glossary/`, `linker/{matcher,render,path}`, and `preprocessor.rs`. Internal-only; no behavior change.
- **Snapshot tests** (`insta`) replace several hand-written `assert_eq!` HTML comparisons. Snapshot files live under `src/linker/snapshots/`.

### Configuration Options

- `display-mode`: How linked terms are rendered (default: `"link"`)

## [0.0.7] - 2026-05-14

### Fixed

- **Admonitions / GitHub-style alerts**: `> [!NOTE]`, `> [!TIP]`, `> [!IMPORTANT]`, `> [!WARNING]`, and `> [!CAUTION]` blockquotes are no longer corrupted by the preprocessor and now render as proper admonitions in mdBook output (issue [#6](https://github.com/rubentalstra/mdbook-termlink/issues/6) reported by [@DocKDE](https://github.com/DocKDE))
  - Root cause: the pulldown-cmark parser was built without `Options::ENABLE_GFM`, so alert markers were parsed as plain blockquote text and lost on serialization
  - Glossary term linking continues to work inside admonition bodies

## [0.0.6] - 2026-03-04

### Added

- **Split Definitions**: New `split-pattern` config option to split glossary definitions at a custom delimiter, showing only the first part in tooltips while keeping the full definition in the glossary (PR [#5](https://github.com/rubentalstra/mdbook-termlink/pull/5) by [@eloraju](https://github.com/eloraju))

### Configuration Options

- `split-pattern`: Split definitions at a delimiter for shorter tooltips (default: disabled)

## [0.0.5] - 2026-01-12

### Fixed

- **HTML Tag Parsing**: Fixed broken HTML output where `</a>` closing tags were being lost
  - Previously, the entire processed text was wrapped in a single `Event::Html`, causing mdBook's parser to mishandle mixed HTML/text content
  - Now emits separate `Event::Text` and `Event::Html` events for proper HTML structure
  - Eliminates "unexpected HTML end tag `</a>`" warnings during `mdbook build`
  - Glossary term links now render correctly without nested unclosed anchor tags

### Changed

- Replaced `replace_terms_in_text()` with `replace_terms_to_events()` for cleaner event-based output
- Internal refactoring of term replacement logic to emit split events

## [0.0.4] - 2026-01-12

### Added

- **End-to-End Testing**: Comprehensive e2e test suite that runs real `mdbook build` with the preprocessor
- 11 e2e tests covering all major features:
  - Basic term linking
  - Tooltip preview (title attributes)
  - Code block exclusion
  - Inline code exclusion
  - Link-first-only behavior
  - Nested chapter relative paths
  - Heading exclusion
  - CLI `supports html` command
  - CLI `supports pdf` rejection
  - Exclude pages functionality
  - Alias linking
- Cross-platform CI testing (Linux, macOS, Windows) with mdBook 0.5.2

### Changed

- Test suite now includes 52 tests total (35 unit + 11 e2e + 6 integration)

## [0.0.3] - 2026-01-12

### Added

- **Tooltip Preview**: Glossary definitions now appear as tooltips on hover via HTML `title` attribute
- **Exclude Pages**: Skip term linking for specified pages using glob patterns (`exclude-pages` config option)
- **Term Aliases**: Define alternative names for terms in `book.toml` (`aliases` config option)
- Alias conflict detection with clear error messages

### Configuration Options

- `exclude-pages`: List of glob patterns for pages to skip (default: `[]`)
- `aliases`: Map of term names to alternative names (default: `{}`)

## [0.0.1] - 2026-01-12

### Added

- Initial release
- Parse glossary terms from definition list markdown format
- Auto-link first occurrence of each term per page (configurable)
- Skip code blocks, inline code, existing links, and headings
- Case-insensitive term matching (configurable)
- Custom CSS class for glossary term links
- Support for terms with short forms (e.g., "API (Application Programming Interface)")
- Word boundary matching to avoid partial term matches
- Relative path calculation for nested chapters
- Comprehensive test suite with unit and integration tests

### Configuration Options

- `glossary-path`: Path to glossary file (default: `reference/glossary.md`)
- `link-first-only`: Only link first occurrence per page (default: `true`)
- `css-class`: CSS class for term links (default: `glossary-term`)
- `case-sensitive`: Case-sensitive matching (default: `false`)

[Unreleased]: https://github.com/rubentalstra/mdbook-termlink/compare/v0.1.0...HEAD

[0.1.0]: https://github.com/rubentalstra/mdbook-termlink/compare/v0.0.7...v0.1.0

[0.0.7]: https://github.com/rubentalstra/mdbook-termlink/compare/v0.0.6...v0.0.7

[0.0.6]: https://github.com/rubentalstra/mdbook-termlink/compare/v0.0.5...v0.0.6

[0.0.5]: https://github.com/rubentalstra/mdbook-termlink/compare/v0.0.4...v0.0.5

[0.0.4]: https://github.com/rubentalstra/mdbook-termlink/compare/v0.0.3...v0.0.4

[0.0.3]: https://github.com/rubentalstra/mdbook-termlink/compare/v0.0.1...v0.0.3

[0.0.1]: https://github.com/rubentalstra/mdbook-termlink/releases/tag/v0.0.1
