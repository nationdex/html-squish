//! Core zero-allocation, single-pass HTML & JSON-LD minifier.

use crate::config::SquishConfig;
use crate::simd::{find_byte_swar, find_text_special_swar, is_whitespace};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LastTag {
    None,
    Open,
    Close,
    SelfClosing,
}

/// Minifies HTML and embedded JSON-LD / JS / CSS in-place.
///
/// Returns the new length of the minified content in `buf`.
/// Since minification never increases buffer size, this is guaranteed to be zero-allocation.
#[inline]
pub fn squish_in_place_with_config(buf: &mut [u8], config: &SquishConfig) -> usize {
    let len = buf.len();
    if len == 0 {
        return 0;
    }

    let mut r = 0; // read pointer
    let mut w = 0; // write pointer

    let mut last_tag = LastTag::None;

    while r < len {
        // Fast-scan forward through plain text when possible
        let special_offset = find_text_special_swar(&buf[r..]);
        if special_offset > 0 {
            let chunk_end = r + special_offset;
            if w == r {
                w = chunk_end;
                r = chunk_end;
            } else {
                buf.copy_within(r..chunk_end, w);
                w += special_offset;
                r = chunk_end;
            }
            last_tag = LastTag::None;
            if r >= len {
                break;
            }
        }

        let b = buf[r];

        if b == b'<' {
            // Check for HTML comment <!--
            if r + 4 <= len && &buf[r..r + 4] == b"<!--" {
                let is_bang = r + 5 <= len && buf[r + 4] == b'!';
                let should_strip = if is_bang {
                    config.strip_bang_comments
                } else {
                    config.strip_comments
                };

                if should_strip {
                    // Skip past -->
                    r += 4;
                    while r + 2 < len {
                        if buf[r] == b'-' && buf[r + 1] == b'-' && buf[r + 2] == b'>' {
                            r += 3;
                            break;
                        }
                        r += 1;
                    }
                    if r + 2 >= len && r < len {
                        r = len; // unclosed comment at EOF
                    }
                    continue;
                }
            }

            // Check for CDATA <![CDATA[
            if r + 9 <= len && &buf[r..r + 9] == b"<![CDATA[" {
                buf.copy_within(r..r + 9, w);
                w += 9;
                r += 9;
                while r + 2 < len {
                    buf[w] = buf[r];
                    w += 1;
                    r += 1;
                    if buf[r - 1] == b']' && buf[r] == b']' && buf[r + 1] == b'>' {
                        buf[w] = buf[r];
                        buf[w + 1] = buf[r + 1];
                        w += 2;
                        r += 2;
                        break;
                    }
                }
                last_tag = LastTag::Close;
                continue;
            }

            // Check for <script
            if r + 7 <= len && buf[r + 1..r + 7].eq_ignore_ascii_case(b"script") {
                let next = buf[r + 7];
                if next == b'>' || next == b'/' || is_whitespace(next) {
                    process_script_tag(buf, &mut r, &mut w, len, config);
                    last_tag = LastTag::Close;
                    continue;
                }
            }

            // Check for <style
            if r + 6 <= len && buf[r + 1..r + 6].eq_ignore_ascii_case(b"style") {
                let next = buf[r + 6];
                if next == b'>' || next == b'/' || is_whitespace(next) {
                    process_style_tag(buf, &mut r, &mut w, len, config);
                    last_tag = LastTag::Close;
                    continue;
                }
            }

            // Check for <pre or <textarea
            if config.preserve_pre_and_textarea {
                if r + 4 <= len && buf[r + 1..r + 4].eq_ignore_ascii_case(b"pre") {
                    let next = buf[r + 4];
                    if next == b'>' || next == b'/' || is_whitespace(next) {
                        process_verbatim_tag(buf, &mut r, &mut w, len, b"pre");
                        last_tag = LastTag::Close;
                        continue;
                    }
                } else if r + 9 <= len && buf[r + 1..r + 9].eq_ignore_ascii_case(b"textarea") {
                    let next = buf[r + 9];
                    if next == b'>' || next == b'/' || is_whitespace(next) {
                        process_verbatim_tag(buf, &mut r, &mut w, len, b"textarea");
                        last_tag = LastTag::Close;
                        continue;
                    }
                }
            }

            // Standard HTML tag
            last_tag = process_normal_tag(buf, &mut r, &mut w, len, config);
        } else if is_whitespace(b) {
            if !config.collapse_whitespace {
                buf[w] = b;
                w += 1;
                r += 1;
                last_tag = LastTag::None;
            } else {
                // Skip all contiguous whitespace
                while r < len && is_whitespace(buf[r]) {
                    r += 1;
                }

                // If at EOF, drop trailing whitespace
                if r >= len {
                    break;
                }

                let next_is_lt = buf[r] == b'<';

                if next_is_lt {
                    let next_is_closing_tag = r + 1 < len && buf[r + 1] == b'/';

                    if next_is_closing_tag {
                        // Whitespace immediately before a closing tag </...>: drop it!
                    } else if last_tag != LastTag::None {
                        // Whitespace between tags (> \s+ <)
                        if !config.aggressive_intertag_whitespace
                            && w > 0
                            && buf[w - 1] != b' '
                        {
                            buf[w] = b' ';
                            w += 1;
                        }
                    } else {
                        // Whitespace between text and opening tag: keep 1 space
                        if w > 0 && buf[w - 1] != b' ' {
                            buf[w] = b' ';
                            w += 1;
                        }
                    }
                } else {
                    // Next is regular text character
                    if last_tag == LastTag::Open {
                        // Drop whitespace immediately after opening tag: <p>  Text -> <p>Text
                    } else if last_tag == LastTag::Close || last_tag == LastTag::SelfClosing {
                        // Keep 1 space between closing tag and text: </b>  text -> </b> text
                        if w > 0 && buf[w - 1] != b' ' {
                            buf[w] = b' ';
                            w += 1;
                        }
                    } else {
                        // Regular text-to-text whitespace
                        if w > 0 && buf[w - 1] != b' ' {
                            buf[w] = b' ';
                            w += 1;
                        }
                    }
                    last_tag = LastTag::None;
                }
            }
        } else {
            // Normal character
            buf[w] = b;
            w += 1;
            r += 1;
            last_tag = LastTag::None;
        }
    }

    w
}

/// Minifies a standard HTML tag `<...>` in-place.
#[inline]
fn process_normal_tag(
    buf: &mut [u8],
    r: &mut usize,
    w: &mut usize,
    len: usize,
    config: &SquishConfig,
) -> LastTag {
    // Write '<'
    buf[*w] = b'<';
    *w += 1;
    *r += 1;

    let mut is_closing = false;
    let mut is_self_closing = false;

    // Optional '/' for closing tag
    if *r < len && buf[*r] == b'/' {
        is_closing = true;
        buf[*w] = b'/';
        *w += 1;
        *r += 1;
    }

    let mut in_quote: Option<u8> = None;
    let mut pending_space = false;

    while *r < len {
        let b = buf[*r];

        if let Some(quote) = in_quote {
            buf[*w] = b;
            *w += 1;
            *r += 1;
            if b == quote {
                in_quote = None;
            }
            continue;
        }

        if b == b'"' || b == b'\'' {
            if pending_space {
                buf[*w] = b' ';
                *w += 1;
                pending_space = false;
            }
            in_quote = Some(b);
            buf[*w] = b;
            *w += 1;
            *r += 1;
        } else if b == b'>' {
            // Strip any pending whitespace before '>'
            buf[*w] = b'>';
            *w += 1;
            *r += 1;
            break;
        } else if b == b'/' && *r + 1 < len && buf[*r + 1] == b'>' {
            // Self-closing '/>': strip space before '/'
            is_self_closing = true;
            buf[*w] = b'/';
            buf[*w + 1] = b'>';
            *w += 2;
            *r += 2;
            break;
        } else if is_whitespace(b) {
            *r += 1;
            while *r < len && is_whitespace(buf[*r]) {
                *r += 1;
            }
            if config.normalize_attributes {
                // Only mark pending_space if not followed by '>' or '=' or '/>'
                if *r < len && buf[*r] != b'>' && buf[*r] != b'=' && buf[*r] != b'/' {
                    pending_space = true;
                }
            } else {
                pending_space = true;
            }
        } else if b == b'=' && config.normalize_attributes {
            // Remove whitespace before '='
            pending_space = false;
            buf[*w] = b'=';
            *w += 1;
            *r += 1;
            // Also skip any whitespace after '='
            while *r < len && is_whitespace(buf[*r]) {
                *r += 1;
            }
        } else {
            if pending_space {
                buf[*w] = b' ';
                *w += 1;
                pending_space = false;
            }
            buf[*w] = b;
            *w += 1;
            *r += 1;
        }
    }

    if is_self_closing {
        LastTag::SelfClosing
    } else if is_closing {
        LastTag::Close
    } else {
        LastTag::Open
    }
}

/// Handles `<script...>` tags, detecting JSON-LD (`type="application/ld+json"` or `type="application/json"`).
#[inline]
fn process_script_tag(
    buf: &mut [u8],
    r: &mut usize,
    w: &mut usize,
    len: usize,
    config: &SquishConfig,
) {
    let script_start_r = *r;

    // Scan until closing '>' of the opening tag
    let mut is_json_ld = false;
    let mut in_quote: Option<u8> = None;

    // Check opening tag content for type="application/ld+json"
    let mut tag_close_r = *r;
    while tag_close_r < len {
        let b = buf[tag_close_r];
        if let Some(q) = in_quote {
            if b == q {
                in_quote = None;
            }
        } else if b == b'"' || b == b'\'' {
            in_quote = Some(b);
        } else if b == b'>' {
            tag_close_r += 1;
            break;
        }
        tag_close_r += 1;
    }

    let open_tag = &buf[script_start_r..tag_close_r];
    if contains_ignore_case(open_tag, b"application/ld+json")
        || contains_ignore_case(open_tag, b"application/json")
    {
        is_json_ld = true;
    }

    // Process the opening `<script ...>` tag with normal tag normalization
    process_normal_tag(buf, r, w, tag_close_r, config);

    if is_json_ld && config.minify_json_ld {
        minify_json_in_script(buf, r, w, len);
    } else if config.minify_js {
        minify_js_in_script(buf, r, w, len);
    } else {
        // Copy verbatim until </script>
        copy_until_closing_tag(buf, r, w, len, b"script");
    }
}

/// Minifies JSON-LD inside `<script type="application/ld+json">`.
/// Drops all whitespace outside string literals `"..."`.
#[inline]
fn minify_json_in_script(buf: &mut [u8], r: &mut usize, w: &mut usize, len: usize) {
    let mut in_string = false;
    let mut escaped = false;

    while *r < len {
        // Check for closing </script
        if !in_string
            && *r + 8 <= len
            && buf[*r..*r + 2] == *b"</"
            && buf[*r + 2..*r + 8].eq_ignore_ascii_case(b"script")
        {
            let next = if *r + 8 < len { buf[*r + 8] } else { b'>' };
            if next == b'>' || is_whitespace(next) {
                break;
            }
        }

        let b = buf[*r];

        if in_string {
            buf[*w] = b;
            *w += 1;
            *r += 1;

            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
        } else {
            if b == b'"' {
                in_string = true;
                buf[*w] = b;
                *w += 1;
                *r += 1;
            } else if is_whitespace(b) {
                // Drop whitespace outside string
                *r += 1;
            } else {
                buf[*w] = b;
                *w += 1;
                *r += 1;
            }
        }
    }
}

/// Minifies JavaScript inside standard `<script>`.
/// Strips comments (`//` and `/* */`) and collapses whitespace.
#[inline]
fn minify_js_in_script(buf: &mut [u8], r: &mut usize, w: &mut usize, len: usize) {
    let mut in_string: Option<u8> = None;
    let mut escaped = false;

    while *r < len {
        // Check for closing </script
        if in_string.is_none()
            && *r + 8 <= len
            && buf[*r..*r + 2] == *b"</"
            && buf[*r + 2..*r + 8].eq_ignore_ascii_case(b"script")
        {
            let next = if *r + 8 < len { buf[*r + 8] } else { b'>' };
            if next == b'>' || is_whitespace(next) {
                break;
            }
        }

        let b = buf[*r];

        if let Some(quote) = in_string {
            buf[*w] = b;
            *w += 1;
            *r += 1;
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == quote {
                in_string = None;
            }
            continue;
        }

        // Check for string literal start
        if b == b'"' || b == b'\'' || b == b'`' {
            in_string = Some(b);
            buf[*w] = b;
            *w += 1;
            *r += 1;
            continue;
        }

        // Check for single line comment //
        if b == b'/' && *r + 1 < len && buf[*r + 1] == b'/' {
            *r += 2;
            while *r < len && buf[*r] != b'\n' && buf[*r] != b'\r' {
                *r += 1;
            }
            continue;
        }

        // Check for multi-line comment /*
        if b == b'/' && *r + 1 < len && buf[*r + 1] == b'*' {
            *r += 2;
            while *r + 1 < len {
                if buf[*r] == b'*' && buf[*r + 1] == b'/' {
                    *r += 2;
                    break;
                }
                *r += 1;
            }
            continue;
        }

        // Collapse whitespace in JS
        if is_whitespace(b) {
            *r += 1;
            while *r < len && is_whitespace(buf[*r]) {
                *r += 1;
            }
            // Emit a single space if surrounded by identifier/word characters
            if *w > 0 && is_js_ident_byte(buf[*w - 1]) && *r < len && is_js_ident_byte(buf[*r]) {
                buf[*w] = b' ';
                *w += 1;
            }
            continue;
        }

        buf[*w] = b;
        *w += 1;
        *r += 1;
    }
}

/// Handles `<style>` CSS minification.
#[inline]
fn process_style_tag(
    buf: &mut [u8],
    r: &mut usize,
    w: &mut usize,
    len: usize,
    config: &SquishConfig,
) {
    let mut tag_close_r = *r;
    while tag_close_r < len && buf[tag_close_r] != b'>' {
        tag_close_r += 1;
    }
    if tag_close_r < len {
        tag_close_r += 1;
    }

    process_normal_tag(buf, r, w, tag_close_r, config);

    if config.minify_css {
        minify_css_in_style(buf, r, w, len);
    } else {
        copy_until_closing_tag(buf, r, w, len, b"style");
    }
}

/// Minifies CSS inside `<style>` block.
#[inline]
fn minify_css_in_style(buf: &mut [u8], r: &mut usize, w: &mut usize, len: usize) {
    let mut in_string: Option<u8> = None;

    while *r < len {
        // Check for </style
        if in_string.is_none()
            && *r + 7 <= len
            && buf[*r..*r + 2] == *b"</"
            && buf[*r + 2..*r + 7].eq_ignore_ascii_case(b"style")
        {
            let next = if *r + 7 < len { buf[*r + 7] } else { b'>' };
            if next == b'>' || is_whitespace(next) {
                break;
            }
        }

        let b = buf[*r];

        if let Some(quote) = in_string {
            buf[*w] = b;
            *w += 1;
            *r += 1;
            if b == quote {
                in_string = None;
            }
            continue;
        }

        if b == b'"' || b == b'\'' {
            in_string = Some(b);
            buf[*w] = b;
            *w += 1;
            *r += 1;
            continue;
        }

        // CSS comment /* ... */
        if b == b'/' && *r + 1 < len && buf[*r + 1] == b'*' {
            *r += 2;
            while *r + 1 < len {
                if buf[*r] == b'*' && buf[*r + 1] == b'/' {
                    *r += 2;
                    break;
                }
                *r += 1;
            }
            continue;
        }

        if is_whitespace(b) {
            *r += 1;
            while *r < len && is_whitespace(buf[*r]) {
                *r += 1;
            }
            // Only emit space if between identifiers/tokens where required
            if *w > 0 && is_css_ident_byte(buf[*w - 1]) && *r < len && is_css_ident_byte(buf[*r]) {
                buf[*w] = b' ';
                *w += 1;
            }
            continue;
        }

        buf[*w] = b;
        *w += 1;
        *r += 1;
    }
}

/// Preserves `<pre>` and `<textarea>` content verbatim.
#[inline]
fn process_verbatim_tag(
    buf: &mut [u8],
    r: &mut usize,
    w: &mut usize,
    len: usize,
    tag_name: &[u8],
) {
    // Read opening tag until '>'
    while *r < len {
        let b = buf[*r];
        buf[*w] = b;
        *w += 1;
        *r += 1;
        if b == b'>' {
            break;
        }
    }

    // Copy verbatim until closing `</tag_name>`
    copy_until_closing_tag(buf, r, w, len, tag_name);
}

/// Copies content verbatim until closing `</{tag_name}`.
#[inline]
fn copy_until_closing_tag(
    buf: &mut [u8],
    r: &mut usize,
    w: &mut usize,
    len: usize,
    tag_name: &[u8],
) {
    let tag_len = tag_name.len();
    let close_prefix_len = 2 + tag_len; // "</" + name

    while *r < len {
        let next_lt = find_byte_swar(&buf[*r..len], b'<');
        if next_lt > 0 {
            let chunk_end = *r + next_lt;
            if *w == *r {
                *w = chunk_end;
                *r = chunk_end;
            } else {
                buf.copy_within(*r..chunk_end, *w);
                *w += next_lt;
                *r = chunk_end;
            }
            if *r >= len {
                break;
            }
        }

        if *r + close_prefix_len <= len
            && buf[*r] == b'<'
            && buf[*r + 1] == b'/'
            && buf[*r + 2..*r + 2 + tag_len].eq_ignore_ascii_case(tag_name)
        {
            let next = if *r + close_prefix_len < len {
                buf[*r + close_prefix_len]
            } else {
                b'>'
            };
            if next == b'>' || is_whitespace(next) {
                break;
            }
        }

        buf[*w] = buf[*r];
        *w += 1;
        *r += 1;
    }
}

#[inline(always)]
fn is_js_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

#[inline(always)]
fn is_css_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'%'
}

#[inline]
fn contains_ignore_case(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.len() > haystack.len() {
        return false;
    }
    haystack
        .windows(needle.len())
        .any(|w| w.eq_ignore_ascii_case(needle))
}
