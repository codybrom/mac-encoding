//! The errors. Each error gives the item that caused it.
//!
//! Section 4.1 of the standard says that a handler returns "error optionally
//! with a code point". The encoder always gives a code point. Refer to
//! section 9.2, "return error with codePoint". Thus [`EncodeError`] has one.
//! The decoder gives no item, but the byte is the only useful data. Thus
//! [`DecodeError`] has the byte.
//!
//! Each error also gives a position. The standard does not ask for this. But
//! a program that cannot encode a resource must tell you which part of the
//! text caused the error.

use core::fmt;

use crate::Encoding;

/// The encoding has no byte for this character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncodeError {
    /// The encoding with no byte for [`Self::code_point`].
    pub encoding: Encoding,
    /// The first code point with no mapping.
    pub code_point: char,
    /// The position of [`Self::code_point`] in the text, counted in bytes.
    pub index: usize,
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} has no byte for U+{:04X} ({:?}) at index {}",
            self.encoding.apple_name(),
            self.code_point as u32,
            self.code_point,
            self.index
        )
    }
}

impl core::error::Error for EncodeError {}

/// The encoding has no character for this byte.
///
/// Almost all of Apple's tables have a mapping for all 256 bytes. Thus almost
/// all encodings do not give this error. [`Encoding::defines_every_byte`]
/// tells you which encodings have a mapping for all bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeError {
    /// The encoding with no code point for [`Self::byte`].
    pub encoding: Encoding,
    /// The first byte with no mapping.
    pub byte: u8,
    /// The position of [`Self::byte`] in the bytes.
    pub index: usize,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} has no character for byte {:#04X} at index {}",
            self.encoding.apple_name(),
            self.byte,
            self.index
        )
    }
}

impl core::error::Error for DecodeError {}
