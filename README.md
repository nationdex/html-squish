# html-squish

Zero-allocation, SIMD-accelerated HTML and embedded JSON-LD minifier for Rust.

Designed for high-throughput SSR web engines (such as MindVote and Maud-based architectures) where every microsecond and byte count.

## Features

- **Zero Runtime Allocations**: Minifies in-place (`squish_in_place(&mut [u8]) -> usize`), reusing existing memory ($O(N)$ single pass, $O(1)$ space).
- **Embedded JSON-LD Minification**: Automatically discovers and strips redundant whitespace inside `<script type="application/ld+json">` without breaking JSON strings.
- **Embedded Script & Style Minifier**: Strips single-line (`//`), multi-line (`/* */`) comments and collapses whitespace in inline `<script>` and `<style>` blocks.
- **Tag & Attribute Normalization**: Removes superfluous spaces around attribute assignments (`class = "btn"` $\to$ `class="btn"`) and collapses inter-element whitespace.
- **Literal Preservation**: Verbatim preservation of literal content inside `<pre>` and `<textarea>` tags.
- **SIMD / SWAR Vectorization**: 64-bit vector scans over text chunks to leap over uninteresting bytes at gigabytes per second.
- **`no_std` Support**: Direct slice operations with `#![no_std]`, zero heap requirements.

## Quick Start

### In-Place Slice (Zero-Alloc)
```rust
use html_squish::squish_in_place;

let mut html = *b"  <div>  <p>  MindVote  </p>  </div>  ";
let len = squish_in_place(&mut html);
assert_eq!(&html[..len], b"<div><p>MindVote</p></div>");
```

### In-Place String Slice
```rust
use html_squish::squish_in_place_str;

let mut html = String::from(r#"
    <script type="application/ld+json">
    {
        "@context": "https://schema.org",
        "name": "MindVote"
    }
    </script>
"#);

let minified = squish_in_place_str(&mut html);
// Result: <script type="application/ld+json">{"@context":"https://schema.org","name":"MindVote"}</script>
```

### Maud Integration
```rust
use html_squish::squish_in_place_str;
use maud::html;

let mut markup = html! {
    h1 { "MindVote" }
    p { "Prediction market" }
}.into_string();

let squished = squish_in_place_str(&mut markup);
```

## Benchmarks

Single-pass in-place minification throughput on representative HTML documents containing metadata, JSON-LD, CSS, and JS blocks:

```text
html-squish/squish_in_place:      321.7 MiB/s (~9.5 µs per payload)
html-squish/squish_aggressive:    261.8 MiB/s (~11.7 µs per payload)
```

