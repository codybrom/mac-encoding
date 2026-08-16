//! This crate changes text between Apple's classic Mac OS encodings and
//! Unicode.
//!
//! Apple wrote the mappings and publishes them as mapping files. The
//! generator in `tools/generate-tables` reads those files and writes the
//! tables in `src/tables.rs`. The decode and encode operations obey the WHATWG
//! [Encoding Standard](https://encoding.spec.whatwg.org/), sections 9.1 and
//! 9.2. Thus this crate and a web browser give the same result for each
//! encoding in the standard.
//!
//! ```
//! use mac_encoding::Encoding;
//!
//! // A resource type code goes through Mac OS Roman and comes back.
//! assert_eq!(Encoding::Roman.decode(b"CODE"), "CODE");
//! assert_eq!(Encoding::Roman.encode("CODE").unwrap(), b"CODE");
//!
//! // The standard calls this encoding `macintosh`.
//! assert_eq!(Encoding::from_label("X-Mac-Roman"), Some(Encoding::Roman));
//! ```
//!
//! # The encodings in the standard
//!
//! The standard gives a name to only two of these encodings. Mac OS Roman is
//! `macintosh` and Mac OS Cyrillic is `x-mac-cyrillic`. The other 19
//! encodings have no name in the standard. Use [`Encoding::from_id`] for
//! them and [`Encoding::from_label`] for the other two.
//!
//! In the standard, `x-mac-cyrillic` also has the label `x-mac-ukrainian`.
//! This agrees with Apple. The mapping file `UKRAINE.TXT` tells us that Mac
//! OS 9 put the Ukrainian characters into Mac OS Cyrillic. Thus this crate
//! has no Ukrainian encoding, because Apple supplies no mapping file for it.
//!
//! # Two conditions that the standard does not include
//!
//! Section 9.1 permits a decoder to answer an ASCII byte with the same
//! value. This is not correct for three of these encodings. Mac OS Symbol has
//! GREEK CAPITAL LETTER ALPHA at byte `0x41`. Mac OS Dingbats and Mac OS
//! Keyboard also give other characters to many bytes in that range. Thus the
//! decoder always uses the table. For the other encodings, the table gives
//! the same result as the rule in the standard.
//!
//! Section 9 also has this rule: one byte gives one code point or none.
//! Apple's tables do not obey that rule. Mac OS Thai gives a character
//! and its position as two code points. Mac OS Devanagari, Gujarati, and
//! Gurmukhi give a two-byte code to some ligatures. The decoder and the
//! encoder both use the longest match first. Thus these mappings are correct
//! in the two directions.
//!
//! # Encodings that cannot encode a byte again correctly
//!
//! Mac OS Arabic, Farsi, and Hebrew give a direction to each mapping. Thus
//! more than one byte can have the same code point. For example, byte `0x2B`
//! and byte `0xAB` are both PLUS SIGN. Only the direction is different. The
//! decoder removes the direction and the encoder gives the lowest byte. Thus
//! a byte from the right-to-left group comes back as its left-to-right
//! equivalent.
//!
//! Mac OS Keyboard has one such condition for a different reason. Byte `0x09`
//! and byte `0x61` both decode to U+2423 OPEN BOX. Apple's comment for that
//! mapping says "duplicates mapping for 0x61, hence no round-trip".
//!
//! [`Encoding::encode_is_lossy`] tells you about all four encodings.
//! [`Encoding::is_directional`] tells you about the three with directions. To
//! encode text and then to decode it is always correct. Only the opposite
//! sequence can give a different byte.
//!
//! # Control characters in Mac OS Keyboard
//!
//! Apple's mapping files do not include the bytes `0x00` to `0x1F` and the
//! byte `0x7F`. For almost all encodings, each of these bytes gives the
//! control character with the same value. Mac OS Keyboard is different. It
//! gives key symbols to 22 of these bytes. For example, byte `0x02` is U+21E5
//! LEFTWARDS ARROW TO BAR.
//!
//! That font has no byte for the control character itself. Thus
//! [`Encoding::encode`] gives an error for those 22 control characters, and
//! [`Encoding::encode_html`] writes a character reference.
//!
//! # This crate does not need `std`
//!
//! The crate is `no_std` and uses `alloc` for [`String`] and [`Vec`]. It has
//! no features and no dependencies.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

#[cfg(test)]
extern crate std;

mod codec;
mod error;
pub mod macroman;
mod tables;

pub use error::{DecodeError, EncodeError};
pub use tables::{Encoding, ALL};

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// The table for one encoding.
///
/// The generator writes these tables. They are not part of the public API.
pub(crate) struct Table {
    id: &'static str,
    apple_name: &'static str,
    whatwg_name: Option<&'static str>,
    labels: &'static [&'static str],
    directional: bool,
    lossy: bool,
    /// True if all 256 bytes have a mapping.
    complete: bool,
    /// The text for each byte. An empty text shows a byte with no mapping.
    dec1: &'static [&'static str; 256],
    /// The two-byte codes, as `(lead, trail, text)`.
    dec2: &'static [(u8, u8, &'static str)],
    /// The mappings with one code point, in code point sequence.
    enc1: &'static [(char, &'static [u8])],
    /// The mappings with more than one code point, longest first.
    enc_seq: &'static [(&'static str, &'static [u8])],
}

impl Encoding {
    /// The permanent identifier, for example `roman` or `central-european`.
    ///
    /// Each encoding has an identifier. Only two have a name in the standard.
    /// Refer to [`Self::whatwg_name`].
    pub fn id(self) -> &'static str {
        self.table().id
    }

    /// Apple's name, for example `Mac OS Roman`.
    pub fn apple_name(self) -> &'static str {
        self.table().apple_name
    }

    /// The name in the standard, if the standard has one.
    ///
    /// The result is `Some("macintosh")` for [`Encoding::Roman`] and
    /// `Some("x-mac-cyrillic")` for [`Encoding::Cyrillic`]. For the other 19
    /// encodings the result is `None`.
    pub fn whatwg_name(self) -> Option<&'static str> {
        self.table().whatwg_name
    }

    /// The labels in the standard for this encoding.
    ///
    /// The list is empty if the standard does not have this encoding.
    pub fn labels(self) -> &'static [&'static str] {
        self.table().labels
    }

    /// Tells you if the table gives a direction to its mappings.
    ///
    /// The result is true for Mac OS Arabic, Farsi, and Hebrew. For these
    /// three encodings, the direction is also the cause of the condition in
    /// [`Self::encode_is_lossy`]. But the two conditions are not the same.
    /// Mac OS Keyboard has the second condition and not the first.
    pub fn is_directional(self) -> bool {
        self.table().directional
    }

    /// Tells you if two codes give the same text.
    ///
    /// If the result is true, one byte can decode to text that encodes to a
    /// different byte. The opposite sequence is always correct. To encode
    /// text and then to decode it always gives the first text again.
    ///
    /// The result is true for Mac OS Arabic, Farsi, Hebrew, and Keyboard. In
    /// the first three encodings, the left-to-right form and the
    /// right-to-left form of a character have the same code point. Mac OS
    /// Keyboard has one such condition at U+2423 OPEN BOX. Apple's comment
    /// for that mapping says "duplicates mapping for 0x61, hence no
    /// round-trip".
    pub fn encode_is_lossy(self) -> bool {
        self.table().lossy
    }

    /// Tells you if all 256 bytes have a mapping.
    ///
    /// If the result is true, [`Self::decode_strict`] cannot fail. Mac OS
    /// Roman is such an encoding. This is the reason that a resource type
    /// code of four bytes always decodes.
    pub fn defines_every_byte(self) -> bool {
        self.table().complete
    }

    /// Finds the encoding with this [`Self::id`]. The text must agree fully.
    pub fn from_id(id: &str) -> Option<Self> {
        ALL.iter().copied().find(|e| e.id() == id)
    }

    /// Finds the encoding with this label.
    ///
    /// This function obeys "get an encoding" in section 4.2 of the standard.
    /// It removes the ASCII space characters at the start and at the end.
    /// Then it compares the text. A capital letter and a small letter are
    /// equivalent.
    ///
    /// Only two encodings have labels. For the other 19 encodings the result
    /// is `None`. Use [`Self::from_id`] for them.
    ///
    /// ```
    /// # use mac_encoding::Encoding;
    /// assert_eq!(Encoding::from_label("  MACINTOSH\n"), Some(Encoding::Roman));
    /// assert_eq!(Encoding::from_label("x-mac-ukrainian"), Some(Encoding::Cyrillic));
    /// assert_eq!(Encoding::from_label("Mac OS Thai"), None);
    /// ```
    pub fn from_label(label: &str) -> Option<Self> {
        // Infra's ASCII whitespace is tab, newline, form feed, carriage
        // return, and space. `str::trim` follows Unicode `White_Space`, a
        // different set that also strips vertical tab and no-break space, so
        // it is not used here.
        let label = label.trim_matches(|c| matches!(c, '\t' | '\n' | '\x0C' | '\r' | ' '));
        ALL.iter()
            .copied()
            .find(|e| e.labels().iter().any(|l| l.eq_ignore_ascii_case(label)))
    }

    /// Decodes `bytes`. A byte with no mapping becomes U+FFFD.
    ///
    /// This is the "replacement" error mode in the standard. If
    /// [`Self::defines_every_byte`] is true, no byte becomes U+FFFD.
    pub fn decode(self, bytes: &[u8]) -> String {
        codec::decode(self, bytes)
    }

    /// Decodes `bytes`. The first byte with no mapping gives an error.
    ///
    /// This is the "fatal" error mode in the standard.
    pub fn decode_strict(self, bytes: &[u8]) -> Result<String, DecodeError> {
        codec::decode_strict(self, bytes)
    }

    /// Encodes `text`. The first code point with no byte gives an error.
    ///
    /// This is the "fatal" error mode in the standard. Use this function for
    /// resource data. A type code of four characters must give four bytes. If
    /// the encoder replaces a character, you find the wrong resource.
    pub fn encode(self, text: &str) -> Result<Vec<u8>, EncodeError> {
        codec::encode(self, text)
    }

    /// Encodes `text`. A code point with no byte becomes `&#NNN;`.
    ///
    /// This is the "html" error mode in the standard. HTML forms need an
    /// encoder that cannot fail. The standard gives a warning about this
    /// mode. You cannot see a difference between this result and text that
    /// contains the same characters. Use [`Self::encode`] for all data that
    /// is not form data.
    pub fn encode_html(self, text: &str) -> Vec<u8> {
        codec::encode_html(self, text)
    }
}

impl fmt::Display for Encoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.apple_name())
    }
}
