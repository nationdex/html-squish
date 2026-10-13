//! # html-squish / maud-strip
//!
//! A blazing-fast, zero-allocation, SIMD-accelerated HTML and JSON-LD minifier for Rust.
//! Designed for high-throughput SSR web servers (such as MindVote and Maud-based stacks).
//!
//! ## Key Capabilities
//! - **Zero Runtime Allocations**: Minifies in-place (`squish_in_place(&mut [u8]) -> usize`),
//!   using the input buffer as the output buffer in a single linear pass ($O(N)$ time, $O(1)$ memory).
//! - **Deep JSON-LD Minification**: Automatically strips whitespace from `<script type="application/ld+json">`
//!   SEO fragments, slicing payloads without breaking JSON syntax.
//! - **Embedded Script & Style Minification**: Strips comments (`//`, `/* */`) and collapses whitespace
//!   in inline `<script>` and `<style>` blocks.
//! - **Pre & Textarea Preservation**: Preserves literal indentation and whitespace inside `<pre>` and `<textarea>`.
//! - **SIMD / SWAR Acceleration**: 64-bit vector scans over text segments to leap over uninteresting bytes at gigabytes per second.
//! - **`no_std` Compatible**: Operates directly on raw slices without requiring the standard library or an allocator.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod config;
pub mod minifier;
pub mod simd;

pub use config::SquishConfig;
pub use minifier::squish_in_place_with_config;

/// Errors returned by slice or streaming minification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SquishError {
    /// The destination buffer is smaller than the required minified output.
    BufferTooSmall,
}

impl core::fmt::Display for SquishError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooSmall => write!(f, "destination buffer too small for minified output"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SquishError {}

/// Minifies HTML and embedded JSON-LD in-place using default configuration.
///
/// Returns the length of the minified content in `buf`. The minified slice is `&buf[..len]`.
///
/// # Example
/// ```
/// use html_squish::squish_in_place;
///
/// let mut html = *b"  <div>  <p>Hello   World</p>  </div>  ";
/// let len = squish_in_place(&mut html);
/// assert_eq!(&html[..len], b"<div><p>Hello World</p></div>");
/// ```
#[inline]
pub fn squish_in_place(buf: &mut [u8]) -> usize {
    squish_in_place_with_config(buf, &SquishConfig::default())
}

/// Minifies an in-place mutable string slice using default configuration.
///
/// Since valid HTML minification only collapses ASCII whitespace and tags,
/// UTF-8 validity is strictly preserved.
///
/// # Example
/// ```
/// use html_squish::squish_in_place_str;
///
/// let mut s = String::from("  <h1>  MindVote  </h1>  ");
/// let minified = squish_in_place_str(&mut s);
/// assert_eq!(minified, "<h1>MindVote</h1>");
/// ```
#[inline]
pub fn squish_in_place_str(s: &mut str) -> &mut str {
    squish_in_place_str_with_config(s, &SquishConfig::default())
}

/// Minifies an in-place mutable string slice with custom configuration.
#[inline]
pub fn squish_in_place_str_with_config<'a>(s: &'a mut str, config: &SquishConfig) -> &'a mut str {
    // Safety: squish_in_place only removes ASCII whitespace, comments, and redundant delimiters.
    // Multi-byte UTF-8 sequences are copied verbatim and never split.
    let bytes = unsafe { s.as_bytes_mut() };
    let new_len = squish_in_place_with_config(bytes, config);
    &mut s[..new_len]
}

/// Minifies from an immutable input buffer into a separate pre-allocated output buffer.
///
/// Guarantees zero heap allocation. Returns `Ok(bytes_written)` or `Err(SquishError::BufferTooSmall)`.
#[inline]
pub fn squish_slice(
    input: &[u8],
    output: &mut [u8],
    config: &SquishConfig,
) -> Result<usize, SquishError> {
    if output.len() < input.len() {
        // Output must be at least input length for zero-alloc in-place copy
        return Err(SquishError::BufferTooSmall);
    }
    output[..input.len()].copy_from_slice(input);
    Ok(squish_in_place_with_config(&mut output[..input.len()], config))
}

/// Minifies an HTML string and returns an allocated `String`.
/// Available when `alloc` is available.
#[inline]
pub fn squish(input: &str) -> alloc::string::String {
    squish_with_config(input, &SquishConfig::default())
}

/// Minifies an HTML string with custom configuration and returns an allocated `String`.
#[inline]
pub fn squish_with_config(input: &str, config: &SquishConfig) -> alloc::string::String {
    let mut vec = alloc::vec::Vec::from(input.as_bytes());
    let len = squish_in_place_with_config(&mut vec, config);
    vec.truncate(len);
    unsafe { alloc::string::String::from_utf8_unchecked(vec) }
}
