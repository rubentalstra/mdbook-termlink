# mdbook-termlink

[![CI](https://github.com/rubentalstra/mdbook-termlink/actions/workflows/ci.yml/badge.svg)](https://github.com/rubentalstra/mdbook-termlink/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/mdbook-termlink.svg)](https://crates.io/crates/mdbook-termlink)
[![Documentation](https://docs.rs/mdbook-termlink/badge.svg)](https://docs.rs/mdbook-termlink)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

An [mdBook](https://github.com/rust-lang/mdBook) preprocessor that automatically links glossary terms throughout your
documentation.

## Features

- Parses glossary terms from definition list markdown format
- Auto-links first occurrence of each term per page (configurable)
- Skips code blocks, inline code, existing links, and headings
- Supports case-insensitive matching (configurable)
- Custom CSS class for styled glossary links
- Supports terms with short forms like "API (Application Programming Interface)"
- **Tooltip Preview**: Shows definition text on hover
- **Exclude Pages**: Skip specific pages using glob patterns
- **Term Aliases**: Define alternative names for terms

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

## Usage

### 1. Add to your `book.toml`

```toml
[preprocessor.termlink]
glossary-path = "reference/glossary.md"
```

### 2. Create a glossary file

Create your glossary using markdown definition lists:

```markdown
# Glossary

API (Application Programming Interface)
: A set of protocols and tools for building software applications.

REST
: Representational State Transfer, an architectural style for distributed systems.

JSON
: JavaScript Object Notation, a lightweight data interchange format.
```

### 3. Build your book

```bash
mdbook build
```

Terms in your chapters will automatically link to their glossary definitions!

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

# Alternative names for terms
[preprocessor.termlink.aliases]
API = ["apis", "api endpoints"]
REST = ["RESTful"]
```

### Configuration Options

| Option            | Type         | Default                   | Description                              |
|-------------------|--------------|---------------------------|------------------------------------------|
| `glossary-path`   | String       | `"reference/glossary.md"` | Path to glossary file relative to `src/` |
| `link-first-only` | Boolean      | `true`                    | Only link first occurrence per page      |
| `css-class`       | String       | `"glossary-term"`         | CSS class for term links                 |
| `case-sensitive`  | Boolean      | `false`                   | Case-sensitive term matching             |
| `exclude-pages`   | String Array | `[]`                      | Glob patterns for pages to skip          |
| `aliases`         | Map          | `{}`                      | Alternative names for terms              |

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

1. **Glossary Parsing**: The preprocessor parses your glossary file looking for definition lists (term followed by
   `: definition`)

2. **Term Extraction**: Each term is extracted with:
    - Full name (e.g., "API (Application Programming Interface)")
    - Anchor (e.g., "api-application-programming-interface")
    - Short form if present (e.g., "API")

3. **Content Processing**: For each chapter (except the glossary):
    - Skips code blocks, inline code, existing links, headings, and images
    - Matches terms using word boundaries to avoid partial matches
    - Creates links to the glossary with the configured CSS class

4. **Link Generation**: Terms are replaced with HTML links (with tooltip if definition exists):
   ```html
   <a href="../reference/glossary.html#api-application-programming-interface" title="A set of protocols and tools for building software applications." class="glossary-term">API</a>
   ```

## Context Awareness

The preprocessor is smart about where it adds links. It will **not** link terms inside:

- Code blocks (fenced or indented)
- Inline code (backticks)
- Existing links
- Headings (to preserve table of contents)
- Image alt text

## Requirements

- mdBook 0.5.0 or later
- Rust 1.88.0 or later (for building from source)

## Development

```bash
# Run tests
cargo test

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings

# Check formatting
cargo fmt --all -- --check

# Build release
cargo build --release
```

## Resources

- [mdBook Documentation](https://rust-lang.github.io/mdBook/)
- [mdBook Preprocessor Guide](https://rust-lang.github.io/mdBook/for_developers/preprocessors.html)
- [pulldown-cmark Definition Lists Spec](https://github.com/pulldown-cmark/pulldown-cmark/blob/main/pulldown-cmark/specs/definition_lists.txt)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
