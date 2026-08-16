//! Mac OS Roman, for classic Mac OS resource text.
//!
//! This module is a short front for [`Encoding::Roman`]. Persons who work
//! with resources use this encoding much more than the other 20 encodings.
//! The module operates on resource type codes and on string resources. For
//! all other work, use [`Encoding`].
//!
//! # The source of the mapping
//!
//! The mapping is Apple's `ROMAN.TXT`. It agrees fully with
//! `index-macintosh.txt` in the standard, which gives the `macintosh`
//! encoding. The tests in `tests/whatwg_conformance.rs` compare the two.
//!
//! # Two bytes that need an explanation
//!
//! Byte `0xDB` is `€` U+20AC. Before Mac OS 8.5, it was `¤` U+00A4. Apple
//! Technote TN1140 gives this change. Apple's table and the standard both
//! have the new value. Thus this crate has it also. Text from before Mac OS
//! 8.5 with a currency sign decodes to a euro sign. The bytes alone do not
//! show you which of the two characters the text had.
//!
//! Byte `0xF0` is the Apple logo. Unicode has no character for it. The
//! mapping uses U+F8FF, which is a private use character. Thus it has a
//! meaning only for programs that use Apple's rule. The mapping file
//! `CORPCHAR.TXT` is the register of these characters.
//!
//! # A different mapping with the same name
//!
//! RFC 1345 gives a different mapping. The IANA `macintosh` registration
//! (MIBenum 2027) refers to RFC 1345. Both show The Unicode Standard 1.0 of
//! 1991, which is earlier than the two changes above. That mapping has `¤` at
//! byte `0xDB`. It also has no character for bytes `0xF0`, `0xF6`, and
//! `0xF7`. Text that you decode with those tables is different.

use alloc::string::String;
use alloc::vec::Vec;

use crate::{EncodeError, Encoding};

/// Decodes Mac OS Roman bytes.
///
/// The encoding has a mapping for all 256 bytes. Thus this function never
/// writes a replacement character. [`Encoding::defines_every_byte`] gives the
/// same data for all encodings. The test `every_byte_is_defined` checks it.
pub fn decode(bytes: &[u8]) -> String {
    Encoding::Roman.decode(bytes)
}

/// Encodes text to Mac OS Roman. The first character with no byte gives an
/// error.
///
/// This function does not replace a character that has no byte. A resource
/// type code of four characters must give four bytes. If the encoder replaces
/// a character, you find the wrong resource.
pub fn encode(text: &str) -> Result<Vec<u8>, EncodeError> {
    Encoding::Roman.encode(text)
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn ascii_round_trips() {
        assert_eq!(decode(b"CODE"), "CODE");
        assert_eq!(encode("CODE").unwrap(), b"CODE");
    }

    #[test]
    fn pi_is_0xb9_not_a_utf8_construction() {
        // Do not make a host file name for `π` from the Mac OS Roman byte
        // 0xB9. That gives a different path, or a path that does not exist.
        // The mapping is always 0xB9 to U+03C0. A file name must be correct
        // UTF-8.
        assert_eq!(decode(&[0xB9]), "π");
        assert_eq!(encode("π").unwrap(), vec![0xB9]);
    }

    #[test]
    fn bullet_is_0xa5_and_asterisk_is_0x2a() {
        // These two bytes cause all of the LearnOOP 5/6 difference.
        assert_eq!(decode(&[0xA5]), "•");
        assert_eq!(decode(&[0x2A]), "*");
    }

    #[test]
    fn every_high_byte_decodes_and_re_encodes() {
        for byte in 0x80u8..=0xFF {
            let decoded = decode(&[byte]);
            assert_eq!(
                encode(&decoded),
                Ok(vec![byte]),
                "byte {byte:#04x} did not round-trip"
            );
        }
    }

    #[test]
    fn every_byte_is_defined() {
        assert!(Encoding::Roman.defines_every_byte());
        assert!(Encoding::Roman
            .decode_strict(&(0..=255).collect::<Vec<u8>>())
            .is_ok());
    }

    #[test]
    fn apple_logo_is_private_use() {
        // U+F8FF has a meaning only in Apple's corporate zone. But it must
        // go through the two operations correctly. If not, the resource names
        // that contain it are not correct.
        assert_eq!(decode(&[0xF0]), "\u{F8FF}");
        assert_eq!(encode("\u{F8FF}").unwrap(), vec![0xF0]);
    }

    #[test]
    fn byte_0xdb_is_the_euro_not_the_currency_sign() {
        // Mac OS 8.5 replaced ¤ with €. Apple's table and the standard both
        // have this change. RFC 1345 is earlier than the change.
        assert_eq!(decode(&[0xDB]), "€");
        assert_eq!(
            encode("¤"),
            Err(EncodeError {
                encoding: Encoding::Roman,
                code_point: '¤',
                index: 0,
            })
        );
    }

    #[test]
    fn encode_error_locates_the_character() {
        // The position uses bytes. Thus `π` moves the position by two.
        let err = encode("πx→").unwrap_err();
        assert_eq!(err.code_point, '→');
        assert_eq!(err.index, 3);
    }
}
