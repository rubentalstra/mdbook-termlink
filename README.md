# mdbook-termlink

[![CI](https://github.com/rubentalstra/mdbook-termlink/actions/workflows/ci.yml/badge.svg)](https://github.com/rubentalstra/mdbook-termlink/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/mdbook-termlink.svg)](https://crates.io/crates/mdbook-termlink)
[![Documentation](https://docs.rs/mdbook-termlink/badge.svg)](https://docs.rs/mdbook-termlink)
[![dependency status](https://deps.rs/repo/github/rubentalstra/mdbook-termlink/status.svg)](https://deps.rs/repo/github/rubentalstra/mdbook-termlink)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

An [mdBook](https://github.com/rust-lang/mdBook) preprocessor that automatically links glossary terms throughout your
documentation.

## Features

- **Automatic Term Linking**: Parses glossary terms from Markdown definition lists and links them throughout your book
- **Smart Context Detection**: Skips code blocks, inline code, existing links, headings, and images
- **Tooltip Preview**: Displays term definitions on hover via HTML `title` attribute (configurable: link, tooltip-only, or both via `display-mode`)
- **Configurable Matching**: Case-insensitive matching with link-first-only option per page
- **Exclude Pages**: Skip specific pages from processing using glob patterns
- **Term Aliases**: Define alternative names that link to the same glossary entry
- **Short Form Support**: Automatically handles terms like "API (Application Programming Interface)"
- **Split Definitions**: Split glossary definitions at a custom delimiter, showing only the first part in tooltips while keeping the full definition in the glossary

## Installation

### From crates.io

```bash
cargo install mdbook-termlink
```

### From source

```bash
git clone https://github.com/rubentalstra/mdbook-termlink.git
cd mdbook-termlink
cargo install --path .
```

## Quick Start

### 1. Configure your `book.toml`

```toml
[preprocessor.termlink]
glossary-path = "reference/glossary.md"
```

### 2. Create a glossary file

Use Markdown definition lists in your glossary:

```markdown
# Glossary

API (Application Programming Interface)
: A set of protocols and tools for building software applications.

REST
: Representational State Transfer, an architectural style for distributed systems.

JSON
: JavaScript Object Notation, a lightweight data interchange format.

XMPP
: Extensible Messaging and Presence Protocol -- An open-standard communication technology.
```

### 3. Build your book

```bash
mdbook build
```

Terms in your chapters will automatically link to their glossary definitions with tooltip previews on hover.

## Configuration

All configuration options with their defaults:

```toml
[preprocessor.termlink]
# Path to the glossary file (relative to src directory)
glossary-path = "reference/glossary.md"

# Only link the first occurrence of each term per page
link-first-only = true

# CSS class applied to glossary term links
css-class = "glossary-term"

# Whether term matching should be case-sensitive
case-sensitive = false

# Pages to exclude from term linking (glob patterns)
exclude-pages = ["changelog.md", "appendix/*"]

# Split definitions at this pattern for shorter tooltips.
# Disabled unless set explicitly to a non-empty string.
split-pattern = ""

# How linked terms are rendered: "link", "tooltip", or "both".
# - link    : <a href title class>term</a>            (default; navigable, browser tooltip)
# - tooltip : <abbr title tabindex class>term</abbr>  (tooltip only, no navigation)
# - both    : <a href class><abbr title tabindex>term</abbr></a>
display-mode = "link"

# Alternative names for terms
[preprocessor.termlink.aliases]
API = ["apis", "api endpoints"]
REST = ["RESTful"]
```

### Options Reference

| Option            | Type    | Default                   | Description                                    |
|-------------------|---------|---------------------------|------------------------------------------------|
| `glossary-path`   | String  | `"reference/glossary.md"` | Path to glossary file relative to `src/`       |
| `link-first-only` | Boolean | `true`                    | Only link first occurrence per page            |
| `css-class`       | String  | `"glossary-term"`         | CSS class for term links                       |
| `case-sensitive`  | Boolean | `false`                   | Case-sensitive term matching                   |
| `exclude-pages`   | Array   | `[]`                      | Glob patterns for pages to skip                |
| `split-pattern`   | String  | Disabled by default       | Split definitions at pattern for short tooltips|
| `display-mode`    | String  | `"link"`                  | Render terms as `link`, `tooltip`, or `both`   |
| `aliases`         | Map     | `{}`                      | Alternative names for terms                    |


## Styling

Add custom styles for glossary links in your `book.toml`:

```toml
[output.html]
additional-css = ["custom.css"]
```

Example `custom.css`:

```css
.glossary-term {
    text-decoration: underline dotted;
    color: inherit;
}

.glossary-term:hover {
    background-color: rgba(0, 0, 0, 0.05);
}
```

## How It Works

1. **Glossary Parsing**: Parses your glossary file for definition lists (term followed by `: definition`). If `split-pattern` is defined, definitions are shortened to the first item of the splitted definition.

2. **Term Extraction**: Extracts each term with its anchor, short form (if present), and definition

3. **Content Processing**: Processes each chapter, matching terms using word boundaries while skipping protected
   contexts

4. **Link Generation**: Replaces terms with HTML markup. The shape depends on `display-mode`:

   ```html
   <!-- display-mode = "link" (default) -->
   <a href="../reference/glossary.html#api"
      title="A set of protocols and tools for building software applications."
      class="glossary-term">API</a>

   <!-- display-mode = "tooltip" -->
   <abbr title="A set of protocols and tools for building software applications."
         tabindex="0"
         class="glossary-term">API</abbr>

   <!-- display-mode = "both" -->
   <a href="../reference/glossary.html#api" class="glossary-term">
     <abbr title="A set of protocols and tools for building software applications."
           tabindex="0">API</abbr>
   </a>
   ```

## Requirements

- mdBook 0.5.0 or later
- Rust 1.88.0 or later (for building from source)

## Development

```bash
# Run all tests (unit, integration, and e2e)
cargo test

# Run only e2e tests (requires mdBook installed)
cargo test --test e2e

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings

# Check formatting
cargo fmt --all -- --check

# Build release
cargo build --release
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributors

- [Ruben Talstra](https://github.com/rubentalstra) — Creator & Maintainer
- [Juuso Elo-Rauta](https://github.com/eloraju) — Split Definitions feature ([#5](https://github.com/rubentalstra/mdbook-termlink/pull/5))

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
