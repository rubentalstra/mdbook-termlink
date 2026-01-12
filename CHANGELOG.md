# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/rubentalstra/mdbook-termlink/compare/v0.0.5...HEAD

[0.0.5]: https://github.com/rubentalstra/mdbook-termlink/compare/v0.0.4...v0.0.5

[0.0.4]: https://github.com/rubentalstra/mdbook-termlink/compare/v0.0.3...v0.0.4

[0.0.3]: https://github.com/rubentalstra/mdbook-termlink/compare/v0.0.1...v0.0.3

[0.0.1]: https://github.com/rubentalstra/mdbook-termlink/releases/tag/v0.0.1
