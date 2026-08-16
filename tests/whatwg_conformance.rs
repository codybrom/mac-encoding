//! Compares Mac OS Roman with the index file of the standard.
//!
//! The generator writes `src/tables.rs` from Apple's `ROMAN.TXT`. It does not
//! read <https://encoding.spec.whatwg.org/index-macintosh.txt>. Thus these
//! tests compare two tables that have different sources. They do not compare
//! the generator with itself.
//!
//! The repository does not contain the index file. Do
//! `./scripts/fetch-sources.sh` to make these tests run. Without the file,
//! they write a notice and stop. The tests after them examine the tables
//! only, and they always run.

use std::path::PathBuf;

use mac_encoding::Encoding;

/// The issue of the index file that this crate agrees with.
///
/// Each index file of the standard has a checksum. Thus a change to the file
/// makes this test fail. Without this test, such a change is difficult to
/// find.
const EXPECTED_IDENTIFIER: &str =
    "f2c6a4f6406b3e86a50a5dba4d2b7dd48e2e33c0d82aefe764535c934ec11764";

fn index_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/whatwg/index-macintosh.txt")
}

/// Gives the index file, or `None` if you did not download it.
fn index_source() -> Option<String> {
    match std::fs::read_to_string(index_path()) {
        Ok(source) => Some(source),
        Err(_) => {
            eprintln!(
                "index comparison tests stopped: {} does not exist. \
                 Do ./scripts/fetch-sources.sh to make these tests run.",
                index_path().display()
            );
            None
        }
    }
}

/// Gives `(pointer, code point)` for each line of the index file.
fn index(source: &str) -> Vec<(u8, char)> {
    source
        .lines()
        .filter_map(|line| {
            let line = line.split('#').next().unwrap().trim();
            if line.is_empty() {
                return None;
            }
            let mut columns = line.split_whitespace();
            let pointer: u8 = columns.next()?.parse().unwrap();
            let hex = columns.next()?.strip_prefix("0x").unwrap();
            let scalar = char::from_u32(u32::from_str_radix(hex, 16).unwrap()).unwrap();
            Some((pointer, scalar))
        })
        .collect()
}

#[test]
fn index_file_is_the_revision_we_targeted() {
    let Some(source) = index_source() else { return };
    assert!(
        source.contains(EXPECTED_IDENTIFIER),
        "the file data/whatwg/index-macintosh.txt is a different issue. \
         Examine the table again before you change EXPECTED_IDENTIFIER"
    );
}

#[test]
fn index_covers_every_high_byte() {
    let Some(source) = index_source() else { return };
    let index = index(&source);
    assert_eq!(index.len(), 128);
    for (i, (pointer, _)) in index.iter().enumerate() {
        assert_eq!(*pointer as usize, i, "index is not densely ordered");
    }
}

#[test]
fn every_pointer_decodes_to_the_indexed_code_point() {
    let Some(source) = index_source() else { return };
    for (pointer, scalar) in index(&source) {
        let byte = pointer + 0x80;
        assert_eq!(
            Encoding::Roman.decode(&[byte]),
            scalar.to_string(),
            "byte {byte:#04X} disagrees with the standard's index"
        );
    }
}

#[test]
fn every_indexed_code_point_encodes_back_to_its_byte() {
    let Some(source) = index_source() else { return };
    for (pointer, scalar) in index(&source) {
        let byte = pointer + 0x80;
        assert_eq!(
            Encoding::Roman.encode(&scalar.to_string()),
            Ok(vec![byte]),
            "U+{:04X} does not encode back to {byte:#04X}",
            scalar as u32
        );
    }
}

#[test]
fn the_index_is_injective() {
    // Section 9.2 uses the first pointer for a code point. This rule is
    // important only if an index has a code point more than one time. This
    // index does not. Thus the encoder is the opposite of the decoder.
    let Some(source) = index_source() else { return };
    let mut seen: Vec<char> = index(&source).into_iter().map(|(_, c)| c).collect();
    seen.sort_unstable();
    let count = seen.len();
    seen.dedup();
    assert_eq!(
        seen.len(),
        count,
        "the index maps two bytes to one code point"
    );
}

#[test]
fn ascii_bytes_are_identity() {
    // Section 9.1 answers an ASCII byte with the same value. This crate uses
    // the table to get that result. Three other encodings of Apple are not
    // correct with the rule of the standard. For Mac OS Roman, the table and
    // the rule agree.
    for byte in 0x00..=0x7Fu8 {
        assert_eq!(Encoding::Roman.decode(&[byte]), (byte as char).to_string());
    }
}

#[test]
fn the_standard_names_this_encoding_macintosh() {
    assert_eq!(Encoding::Roman.whatwg_name(), Some("macintosh"));
    assert_eq!(
        Encoding::Roman.labels(),
        &["csmacintosh", "mac", "macintosh", "x-mac-roman"]
    );
}

#[test]
fn get_an_encoding_trims_only_ascii_whitespace() {
    // Section 4.2 removes the ASCII space characters at the start and at the
    // end. They are tab, new line, form feed, carriage return, and space.
    for padded in [
        "macintosh",
        " macintosh ",
        "\tmacintosh\n",
        "\x0Cmacintosh\r",
    ] {
        assert_eq!(
            Encoding::from_label(padded),
            Some(Encoding::Roman),
            "{padded:?}"
        );
    }
    // Vertical tab and no-break space are not ASCII space characters. Thus a
    // label with them does not agree. The function `str::trim` removes both
    // of them, and thus this crate does not use it.
    for padded in ["\x0Bmacintosh", "\u{A0}macintosh"] {
        assert_eq!(Encoding::from_label(padded), None, "{padded:?}");
    }
}

#[test]
fn labels_are_ascii_case_insensitive() {
    for label in [
        "MACINTOSH",
        "MacIntosh",
        "X-MAC-ROMAN",
        "CSMacintosh",
        "MAC",
    ] {
        assert_eq!(
            Encoding::from_label(label),
            Some(Encoding::Roman),
            "{label}"
        );
    }
}

#[test]
fn cyrillic_answers_to_the_ukrainian_label() {
    // Apple's UKRAINE.TXT tells us that Mac OS 9 put the Ukrainian
    // characters into Mac OS Cyrillic. The labels in the standard agree.
    assert_eq!(
        Encoding::from_label("x-mac-ukrainian"),
        Some(Encoding::Cyrillic)
    );
    assert_eq!(Encoding::Cyrillic.whatwg_name(), Some("x-mac-cyrillic"));
}

#[test]
fn encodings_outside_the_standard_have_no_labels() {
    for encoding in mac_encoding::ALL.iter().copied() {
        if encoding.whatwg_name().is_none() {
            assert!(
                encoding.labels().is_empty(),
                "{encoding} has labels but no name in the standard"
            );
        }
    }
}

#[test]
fn html_error_mode_emits_a_decimal_character_reference() {
    // Section 4.1: write `&`, then `#`, then the shortest base-ten form,
    // then `;`.
    assert_eq!(Encoding::Roman.encode_html("a→b"), b"a&#8594;b".to_vec());
    // This example is in the standard. You cannot see a difference between
    // the result and text that contains the same characters.
    assert_eq!(
        Encoding::Roman.encode_html("\u{1F4A9}"),
        Encoding::Roman.encode_html("&#128169;")
    );
}
