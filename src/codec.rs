//! The decode and encode loops. All encodings use them.
//!
//! These loops obey the single-byte decoder in section 9.1 of the standard
//! and the single-byte encoder in section 9.2. There are two differences,
//! because Apple's tables need them. A comment gives each difference at the
//! correct location. [`crate::Encoding`] tells you which encodings have these
//! conditions.

use alloc::string::{String, ToString as _};
use alloc::vec::Vec;

use crate::{DecodeError, EncodeError, Encoding, Table};

/// U+FFFD. The "replacement" error mode in section 4.1 writes this character.
const REPLACEMENT: char = '\u{FFFD}';

/// Finds a pair of bytes in the two-byte codes.
///
/// Only Mac OS Devanagari, Gujarati, and Gurmukhi have two-byte codes. These
/// codes are for some ligature forms and halant forms. Apple writes such a
/// code as `0xE8+0xE9`. The list is in sequence, but it has ten entries or
/// fewer. Thus a simple search is faster than a binary search.
fn lookup2(table: &Table, lead: u8, trail: u8) -> Option<&'static str> {
    table
        .dec2
        .iter()
        .find(|(l, t, _)| *l == lead && *t == trail)
        .map(|(_, _, text)| *text)
}

/// Decodes `bytes`. The `fatal` flag controls the bytes with no mapping.
///
/// This function does not do step 2 of section 9.1. That step says "if byte
/// is an ASCII byte, then return a code point whose value is byte". Mac OS
/// Dingbats, Symbol, and Keyboard have other characters in the range
/// `0x20..=0x7E`. For example, Mac OS Symbol has GREEK CAPITAL LETTER ALPHA
/// at byte `0x41`. Step 2 gives the wrong character for these three
/// encodings. The table gives the same result as step 2 for all the other
/// encodings.
fn decode_inner(encoding: Encoding, bytes: &[u8], fatal: bool) -> Result<String, DecodeError> {
    let table = encoding.table();
    let mut out = String::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        // Examine the longest match first. If not, the decoder reads a
        // two-byte code as two one-byte codes. A table with no two-byte
        // codes does not do this step.
        if !table.dec2.is_empty() && i + 1 < bytes.len() {
            if let Some(text) = lookup2(table, bytes[i], bytes[i + 1]) {
                out.push_str(text);
                i += 2;
                continue;
            }
        }

        let text = table.dec1[bytes[i] as usize];
        if text.is_empty() {
            // Step 4 of section 9.1: the index has no code point for this byte.
            if fatal {
                return Err(DecodeError {
                    encoding,
                    byte: bytes[i],
                    index: i,
                });
            }
            out.push(REPLACEMENT);
        } else {
            out.push_str(text);
        }
        i += 1;
    }

    Ok(out)
}

pub(crate) fn decode(encoding: Encoding, bytes: &[u8]) -> String {
    decode_inner(encoding, bytes, false).expect("the error branch needs fatal")
}

pub(crate) fn decode_strict(encoding: Encoding, bytes: &[u8]) -> Result<String, DecodeError> {
    decode_inner(encoding, bytes, true)
}

/// Finds the bytes for the longest mapping at the start of `rest`.
///
/// The result has the bytes and the number of bytes of `rest` that they
/// replace. The function examines the mappings with more than one code point
/// first. These are in sequence from the longest to the shortest. Thus the
/// longest mapping wins. For example, in Mac OS Symbol, `®` with U+F87F
/// after it is one byte. A `®` alone is a different byte.
fn encode_prefix(table: &Table, rest: &str) -> Option<(&'static [u8], usize)> {
    for (text, code) in table.enc_seq {
        if rest.starts_with(text) {
            return Some((code, text.len()));
        }
    }

    let c = rest.chars().next()?;
    table
        .enc1
        .binary_search_by_key(&c, |(k, _)| *k)
        .ok()
        .map(|i| (table.enc1[i].1, c.len_utf8()))
}

/// Encodes `text` in the "fatal" error mode of section 4.1.
///
/// This function does not do step 2 of section 9.2. The cause is the same as
/// for [`decode_inner`] and step 2 of the decoder.
///
/// If more than one code gives the same text, the lowest code wins. Refer to
/// [`Encoding::encode_is_lossy`] for the encodings with this condition.
pub(crate) fn encode(encoding: Encoding, text: &str) -> Result<Vec<u8>, EncodeError> {
    let table = encoding.table();
    let mut out = Vec::with_capacity(text.len());
    let mut i = 0;

    while i < text.len() {
        match encode_prefix(table, &text[i..]) {
            Some((code, consumed)) => {
                out.extend_from_slice(code);
                i += consumed;
            }
            None => {
                // Step 4 of section 9.2: return error with codePoint.
                let code_point = text[i..]
                    .chars()
                    .next()
                    .expect("index is on a char boundary");
                return Err(EncodeError {
                    encoding,
                    code_point,
                    index: i,
                });
            }
        }
    }

    Ok(out)
}

/// Encodes `text` in the "html" error mode of section 4.1.
///
/// A code point with no byte becomes `&#`, then its value in base ten, then
/// `;`.
pub(crate) fn encode_html(encoding: Encoding, text: &str) -> Vec<u8> {
    let table = encoding.table();
    let mut out = Vec::with_capacity(text.len());
    let mut i = 0;

    while i < text.len() {
        match encode_prefix(table, &text[i..]) {
            Some((code, consumed)) => {
                out.extend_from_slice(code);
                i += consumed;
            }
            None => {
                let c = text[i..]
                    .chars()
                    .next()
                    .expect("index is on a char boundary");
                out.extend_from_slice(b"&#");
                // For a u32, `to_string` gives the shortest base-ten form.
                out.extend_from_slice((c as u32).to_string().as_bytes());
                out.push(b';');
                i += c.len_utf8();
            }
        }
    }

    out
}
