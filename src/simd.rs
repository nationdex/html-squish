//! High-performance SIMD & SWAR (SIMD Within A Register) fast scanners.
//!
//! Provides vector scanning for delimiters and whitespace without external dependencies,
//! fully compatible with `no_std`.

#[inline(always)]
pub fn is_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

#[inline(always)]
pub fn is_ascii_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0C)
}

/// Fast-scan forward to find the next byte matching any in `[b'<', b' ', b'\t', b'\r', b'\n']`.
/// Uses 64-bit SWAR (SIMD within a register) to inspect 8 bytes at a time.
#[inline]
pub fn find_text_special_swar(bytes: &[u8]) -> usize {
    let mut i = 0;
    let len = bytes.len();

    // Process 8-byte words
    while i + 8 <= len {
        let chunk = u64::from_ne_bytes(bytes[i..i + 8].try_into().unwrap());

        // Check for '<' (0x3C)
        let has_lt = has_byte(chunk, b'<');
        // Check for ' ' (0x20)
        let has_space = has_byte(chunk, b' ');
        // Check for '\n' (0x0A)
        let has_nl = has_byte(chunk, b'\n');
        // Check for '\r' (0x0D)
        let has_cr = has_byte(chunk, b'\r');
        // Check for '\t' (0x09)
        let has_tab = has_byte(chunk, b'\t');

        if has_lt | has_space | has_nl | has_cr | has_tab {
            // Find exact index within the 8 bytes
            for offset in 0..8 {
                let b = bytes[i + offset];
                if b == b'<' || is_whitespace(b) {
                    return i + offset;
                }
            }
        }
        i += 8;
    }

    // Scalar tail
    while i < len {
        let b = bytes[i];
        if b == b'<' || is_whitespace(b) {
            return i;
        }
        i += 1;
    }

    len
}

/// Fast-scan forward to find the next byte matching any in `[b'>', b'/', b'=', b'"', b'\'', b' ', b'\t', b'\r', b'\n']`.
#[inline]
pub fn find_tag_special_swar(bytes: &[u8]) -> usize {
    let mut i = 0;
    let len = bytes.len();

    while i + 8 <= len {
        let chunk = u64::from_ne_bytes(bytes[i..i + 8].try_into().unwrap());

        let has_gt = has_byte(chunk, b'>');
        let has_slash = has_byte(chunk, b'/');
        let has_eq = has_byte(chunk, b'=');
        let has_quote = has_byte(chunk, b'"') | has_byte(chunk, b'\'');
        let has_ws = has_byte(chunk, b' ') | has_byte(chunk, b'\n') | has_byte(chunk, b'\r') | has_byte(chunk, b'\t');

        if has_gt | has_slash | has_eq | has_quote | has_ws {
            for offset in 0..8 {
                let b = bytes[i + offset];
                if b == b'>' || b == b'/' || b == b'=' || b == b'"' || b == b'\'' || is_whitespace(b) {
                    return i + offset;
                }
            }
        }
        i += 8;
    }

    while i < len {
        let b = bytes[i];
        if b == b'>' || b == b'/' || b == b'=' || b == b'"' || b == b'\'' || is_whitespace(b) {
            return i;
        }
        i += 1;
    }

    len
}

/// Fast-scan forward to find the next occurrence of a specific delimiter byte.
#[inline]
pub fn find_byte_swar(bytes: &[u8], target: u8) -> usize {
    let mut i = 0;
    let len = bytes.len();

    while i + 8 <= len {
        let chunk = u64::from_ne_bytes(bytes[i..i + 8].try_into().unwrap());
        if has_byte(chunk, target) {
            for offset in 0..8 {
                if bytes[i + offset] == target {
                    return i + offset;
                }
            }
        }
        i += 8;
    }

    while i < len {
        if bytes[i] == target {
            return i;
        }
        i += 1;
    }

    len
}

/// SWAR byte match: returns true if the 64-bit word contains the byte `b`.
/// Alan Mycroft's 0-byte detection algorithm extended to arbitrary bytes:
/// ((chunk ^ repeated_b) - 0x0101010101010101) & ~(chunk ^ repeated_b) & 0x8080808080808080 != 0
#[inline(always)]
fn has_byte(chunk: u64, b: u8) -> bool {
    let repeated = u64::from_ne_bytes([b; 8]);
    let x = chunk ^ repeated;
    ((x.wrapping_sub(0x0101010101010101)) & !x & 0x8080808080808080) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_byte() {
        let val = u64::from_ne_bytes(*b"abcdefg<");
        assert!(has_byte(val, b'<'));
        assert!(!has_byte(val, b'z'));
    }

    #[test]
    fn test_find_text_special() {
        let text = b"Lorem ipsum dolor sit amet <div class='test'>";
        let idx = find_text_special_swar(text);
        assert_eq!(idx, 5); // first space after "Lorem"

        let text_no_ws = b"abcdefghijk<tag>";
        let idx2 = find_text_special_swar(text_no_ws);
        assert_eq!(idx2, 11); // '<'
    }

    #[test]
    fn test_find_byte() {
        let s = b"abcdefgh12345678>";
        let idx = find_byte_swar(s, b'>');
        assert_eq!(idx, 16);
    }
}
