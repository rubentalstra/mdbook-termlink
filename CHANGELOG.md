# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-01-12

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
[0.1.0]: https://github.com/rubentalstra/mdbook-termlink/releases/tag/v0.1.0
