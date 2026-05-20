# Pagina

Rust-based HTML + CSS Paged Media to PDF engine.

Servo project components (html5ever, cssparser) for parsing, custom paged layout engine, printpdf for output.

## Install

```
cargo install --path crates/pagina-cli
```

## Usage

```
pagina input.html -o output.pdf
```

External fonts:

```
pagina input.html -o output.pdf --font NotoSansJP-Regular.ttf
```

As a library:

```rust
let pdf_bytes = pagina_core::convert(&html);
```

## Supported Features

### CSS Paged Media

- `@page { size; margin }` -- Named sizes (A4, letter, ...), orientation, CSS length units
- Margin boxes -- All 16 positions (`@top-center`, `@bottom-right`, ...)
- `counter(page)` / `counter(pages)` -- Page numbering
- `string-set` / `string()` -- Running headers
- `@page :first` / `:left` / `:right` -- Page-type overrides
- `break-before` / `break-after: page` -- Explicit page breaks
- `float: footnote` -- Footnotes with automatic numbering

### CSS Properties

- `font-size`, `font-weight`, `font-style`, `font-family`
- `color` (named, hex, rgb)
- `text-align` (left, center, right, justify)
- `line-height`, `margin`, `padding`
- `border-bottom`
- `display` (block, inline, none, list-item)

### HTML Elements

- Headings (`h1`-`h6`), paragraphs, lists (`ul`/`ol`), tables, `<pre>`, `<hr>`
- Inline styling (`<strong>`, `<em>`, `<span style="...">`)
- Images (`<img src="...">` -- PNG, JPEG)

### Fonts

- 14 builtin PDF fonts (Helvetica, Courier families)
- External TTF/OTF font loading via `--font` flag
- Accurate glyph width measurement (ttf-parser)

### Style Resolution

- CSS cascade with type, class, ID selectors
- Inline `style` attribute
- UA default stylesheet
- Specificity-ordered resolution, property inheritance

## Architecture

```
HTML --> html5ever --> DOM tree
CSS  --> cssparser --> Style rules + @page
                          |
                    Style resolution (cascade, inherit)
                          |
                    Paged layout (breaks, footnotes, margin boxes)
                          |
                    PDF output (printpdf)
```

### Crates

| Crate | Description |
|---|---|
| `pagina-core` | Engine: DOM, CSS, style, layout, PDF |
| `pagina-cli` | CLI binary |

## License

MIT OR Apache-2.0
