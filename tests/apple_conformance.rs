//! Compares each encoding with Apple's mapping file.
//!
//! The parser in this file is independent of the generator in
//! `tools/generate-tables`. This is on purpose. The generator can lose a
//! line, divide a `+` sequence incorrectly, or put the bytes of a code in the
//! wrong sequence. If it does, these tests do not agree with it. Two parsers
//! that agree with each other give more confidence than one.
//!
//! The repository does not contain Apple's mapping files. Thus these tests
//! need `./scripts/fetch-sources.sh` first. Without the directory `data/`,
//! the tests write a notice and stop. They do not fail. If `data/` has only
//! some of the files, the test `sources_are_complete_when_present` fails.

use std::path::PathBuf;

use mac_encoding::Encoding;

/// Each encoding and its mapping file.
const FILES: &[(Encoding, &str)] = &[
    (Encoding::Arabic, "ARABIC"),
    (Encoding::Celtic, "CELTIC"),
    (Encoding::CentralEuropean, "CENTEURO"),
    (Encoding::Croatian, "CROATIAN"),
    (Encoding::Cyrillic, "CYRILLIC"),
    (Encoding::Devanagari, "DEVANAGA"),
    (Encoding::Dingbats, "DINGBATS"),
    (Encoding::Farsi, "FARSI"),
    (Encoding::Gaelic, "GAELIC"),
    (Encoding::Greek, "GREEK"),
    (Encoding::Gujarati, "GUJARATI"),
    (Encoding::Gurmukhi, "GURMUKHI"),
    (Encoding::Hebrew, "HEBREW"),
    (Encoding::Icelandic, "ICELAND"),
    (Encoding::Inuit, "INUIT"),
    (Encoding::Keyboard, "KEYBOARD"),
    (Encoding::Roman, "ROMAN"),
    (Encoding::Romanian, "ROMANIAN"),
    (Encoding::Symbol, "SYMBOL"),
    (Encoding::Thai, "THAI"),
    (Encoding::Turkish, "TURKISH"),
];

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/apple")
}

/// Gives all the mapping files, or `None` if you did not download them.
///
/// The result is `None` if the directory `data/` does not exist. This is the
/// usual condition after a new clone. But if `data/` exists and a file is not
/// in it, this function fails. That condition shows a fault.
fn sources() -> Option<Vec<(Encoding, &'static str, String)>> {
    if !data_dir().is_dir() {
        eprintln!(
            "Apple comparison tests stopped: {} does not exist. \
             Do ./scripts/fetch-sources.sh to make these tests run.",
            data_dir().display()
        );
        return None;
    }

    Some(
        FILES
            .iter()
            .map(|(encoding, stem)| {
                let path = data_dir().join(format!("{stem}.TXT"));
                let source = std::fs::read_to_string(&path).unwrap_or_else(|e| {
                    panic!(
                        "the directory data/ exists, but this program cannot read {}: {e}\n\
                         Do ./scripts/fetch-sources.sh again.",
                        path.display()
                    )
                });
                (*encoding, *stem, source)
            })
            .collect(),
    )
}

struct Mapping {
    code: Vec<u8>,
    text: String,
    directional: bool,
}

fn parse(source: &str) -> Vec<Mapping> {
    let mut out = Vec::new();
    for line in source.lines() {
        let line = line.split('#').next().unwrap().trim();
        if line.is_empty() {
            continue;
        }
        let mut columns = line.split_whitespace();
        let code_column = columns.next().unwrap();
        let Some(mut mapping_column) = columns.next() else {
            continue;
        };

        let mut directional = false;
        if mapping_column.starts_with('<') {
            directional = true;
            match mapping_column.split_once('+') {
                Some((_, rest)) => mapping_column = rest,
                None => continue,
            }
        }

        let mut code = Vec::new();
        for token in code_column.split('+') {
            let hex = token.strip_prefix("0x").unwrap();
            // Apple writes a two-byte code as `0x8140` or as `0x81+0x40`.
            for pair in hex.as_bytes().chunks(2) {
                code.push(u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap());
            }
        }

        let text: String = mapping_column
            .split('+')
            .map(|t| {
                let hex = t.strip_prefix("0x").unwrap();
                char::from_u32(u32::from_str_radix(hex, 16).unwrap()).unwrap()
            })
            .collect();

        out.push(Mapping {
            code,
            text,
            directional,
        });
    }
    out
}

#[test]
fn sources_are_complete_when_present() {
    let Some(files) = sources() else { return };
    assert_eq!(files.len(), mac_encoding::ALL.len());
    for (encoding, name, source) in &files {
        assert!(
            !parse(source).is_empty(),
            "{name}: parsed no mappings for {encoding}"
        );
    }
}

#[test]
fn every_encoding_has_a_source_file() {
    // This test needs only the list, not the mapping files. Thus it always
    // runs.
    assert_eq!(FILES.len(), mac_encoding::ALL.len());
    for encoding in mac_encoding::ALL.iter().copied() {
        assert!(
            FILES.iter().any(|(e, _)| *e == encoding),
            "{encoding} has no source file"
        );
    }
}

#[test]
fn every_mapping_decodes_to_what_apple_says() {
    let Some(files) = sources() else { return };
    for (encoding, name, source) in &files {
        for m in parse(source) {
            assert_eq!(
                encoding.decode(&m.code),
                m.text,
                "{name}: {:02X?} must decode to {:?}",
                m.code,
                m.text
            );
        }
    }
}

#[test]
fn decoding_is_never_lossy() {
    // Each byte with a mapping must decode to a character. It must not
    // decode to a replacement character.
    let Some(files) = sources() else { return };
    for (encoding, name, source) in &files {
        for m in parse(source) {
            assert!(
                !encoding.decode(&m.code).contains('\u{FFFD}'),
                "{name}: {:02X?} decoded to a replacement character",
                m.code
            );
        }
    }
}

#[test]
fn text_always_round_trips() {
    // If more than one code gives the same text, the encoder can give a
    // different byte. But that byte must decode to the same text. Programs
    // depend on this condition. It is true for all the encodings.
    let Some(files) = sources() else { return };
    for (encoding, name, source) in &files {
        for m in parse(source) {
            let Ok(reencoded) = encoding.encode(&m.text) else {
                panic!("{name}: {:?} has no encoding", m.text);
            };
            assert_eq!(
                encoding.decode(&reencoded),
                m.text,
                "{name}: {:?} did not survive a round trip",
                m.text
            );
        }
    }
}

#[test]
fn bytes_round_trip_unless_the_table_collides() {
    // A byte is different after the two operations only if another code
    // gives the same text. This test finds those codes in the mapping file.
    // Thus an encoding cannot get this condition unless `encode_is_lossy`
    // also changes.
    let Some(files) = sources() else { return };
    for (encoding, name, source) in &files {
        let mappings = parse(source);
        let mut collides = std::collections::HashMap::<&str, usize>::new();
        for m in &mappings {
            *collides.entry(m.text.as_str()).or_default() += 1;
        }

        let mut saw_collision = false;
        for m in &mappings {
            let reencoded = encoding.encode(&m.text).unwrap();
            if collides[m.text.as_str()] > 1 {
                saw_collision = true;
                // The lowest code wins. Thus only that code is the same
                // after the two operations.
                let lowest = mappings
                    .iter()
                    .filter(|o| o.text == m.text)
                    .map(|o| o.code.clone())
                    .min()
                    .unwrap();
                assert_eq!(
                    reencoded, lowest,
                    "{name}: {:?} chose the wrong code",
                    m.text
                );
            } else {
                assert_eq!(
                    reencoded, m.code,
                    "{name}: {:?} re-encoded to a different byte",
                    m.text
                );
            }
        }

        assert_eq!(
            encoding.encode_is_lossy(),
            saw_collision,
            "{name}: encode_is_lossy disagrees with the file"
        );
    }
}

#[test]
fn directional_tables_are_flagged() {
    let Some(files) = sources() else { return };
    for (encoding, name, source) in &files {
        assert_eq!(
            encoding.is_directional(),
            parse(source).iter().any(|m| m.directional),
            "{name}: is_directional disagrees with the file"
        );
    }
}

#[test]
fn omitted_controls_decode_as_themselves() {
    // Apple does not include the bytes 0x00 to 0x1F and the byte 0x7F. The
    // mapping files say that this obeys "the conventions of the standard UTC
    // mapping tables". These bytes are the C0 control characters and DELETE.
    let Some(files) = sources() else { return };
    for (encoding, name, source) in &files {
        let stated: Vec<u8> = parse(source)
            .into_iter()
            .filter(|m| m.code.len() == 1)
            .map(|m| m.code[0])
            .collect();
        for byte in (0x00..=0x1Fu8).chain(std::iter::once(0x7F)) {
            if stated.contains(&byte) {
                continue;
            }
            assert_eq!(
                encoding.decode(&[byte]),
                (byte as char).to_string(),
                "{name}: control {byte:#04X} is not identity"
            );
        }
    }
}

#[test]
fn synthesized_controls_round_trip() {
    // The generator adds the control characters that Apple does not include.
    // It must add them to the encoder only where it also adds them to the
    // decoder. Mac OS Keyboard gives key symbols to 22 of these bytes. For
    // those bytes, the control character has no code at all.
    let Some(files) = sources() else { return };
    for (encoding, name, source) in &files {
        let stated: std::collections::HashMap<u8, String> = parse(source)
            .into_iter()
            .filter(|m| m.code.len() == 1)
            .map(|m| (m.code[0], m.text))
            .collect();
        for byte in (0x00..=0x1Fu8).chain(std::iter::once(0x7F)) {
            let control = (byte as char).to_string();
            // If the mapping file says nothing about the byte, the identity
            // rule applies. The rule also applies if the file gives the byte
            // to the same control character.
            let identity = stated.get(&byte).map_or(true, |text| *text == control);
            if identity {
                assert_eq!(
                    encoding.decode(&[byte]),
                    control,
                    "{name}: control {byte:#04X} does not decode to itself"
                );
                assert_eq!(
                    encoding.encode(&control),
                    Ok(vec![byte]),
                    "{name}: control {byte:#04X} does not encode to itself"
                );
            } else {
                // The mapping file gives this byte to another character. Thus
                // the encoder must not give this byte for the control
                // character.
                assert_ne!(
                    encoding.encode(&control).ok(),
                    Some(vec![byte]),
                    "{name}: control {byte:#04X} encodes to a byte that shows {:?}",
                    stated[&byte]
                );
            }
        }
    }
}

// The tests after this line examine the tables only. Thus they run with the
// mapping files and without them.

#[test]
fn every_scalar_survives_encode_and_then_decode() {
    // The strongest condition that this crate gives: if text encodes, the
    // bytes decode to that same text. The test examines all 1,112,064 scalar
    // values for each encoding, and not only the text in the mapping files.
    // A defect in Mac OS Keyboard was outside those files.
    let mut buffer = [0u8; 4];
    for encoding in mac_encoding::ALL.iter().copied() {
        for value in 0..=0x10FFFFu32 {
            let Some(scalar) = char::from_u32(value) else {
                continue;
            };
            let text = scalar.encode_utf8(&mut buffer);
            if let Ok(bytes) = encoding.encode(text) {
                assert_eq!(
                    encoding.decode(&bytes),
                    *text,
                    "{encoding}: U+{value:04X} encoded to {bytes:02X?}, which shows other text"
                );
            }
        }
    }
}

#[test]
fn keyboard_has_no_code_for_22_control_characters() {
    // Mac OS Keyboard is a font of key symbols. Byte 0x02 is U+21E5 LEFTWARDS
    // ARROW TO BAR. Thus the font has no byte for U+0002. Before the
    // correction, U+0002 encoded to byte 0x02 and that byte decoded to
    // U+21E5.
    let without_a_code: Vec<u32> = (0x00..=0x1Fu32)
        .chain(std::iter::once(0x7F))
        .filter(|v| {
            let c = char::from_u32(*v).unwrap();
            Encoding::Keyboard.encode(&c.to_string()).is_err()
        })
        .collect();
    assert_eq!(
        without_a_code,
        vec![
            0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x09, 0x0A, 0x0B, 0x0C, 0x0F, 0x10, 0x11, 0x12,
            0x13, 0x14, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C
        ]
    );

    assert_eq!(Encoding::Keyboard.decode(&[0x02]), "\u{21E5}");
    assert!(Encoding::Keyboard.encode("\u{2}").is_err());
    // The "html" error mode cannot fail. It writes a character reference.
    assert_eq!(Encoding::Keyboard.encode_html("\u{2}"), b"&#2;");

    // The other encodings keep the control characters.
    assert_eq!(Encoding::Roman.encode("\u{2}").unwrap(), vec![0x02]);
    assert_eq!(Encoding::Symbol.encode("\u{2}").unwrap(), vec![0x02]);
}

#[test]
fn exactly_four_encodings_are_lossy() {
    // Three encodings have this condition because of their directions. Mac
    // OS Keyboard has it because of U+2423. If a different encoding is in
    // this list, there is a fault.
    let lossy: Vec<_> = mac_encoding::ALL
        .iter()
        .copied()
        .filter(|e| e.encode_is_lossy())
        .collect();
    assert_eq!(
        lossy,
        vec![
            Encoding::Arabic,
            Encoding::Farsi,
            Encoding::Hebrew,
            Encoding::Keyboard
        ]
    );

    // Apple's comment in KEYBOARD.TXT says: "duplicates mapping for 0x61,
    // hence no round-trip".
    assert_eq!(Encoding::Keyboard.decode(&[0x09]), "\u{2423}");
    assert_eq!(Encoding::Keyboard.decode(&[0x61]), "\u{2423}");
    assert_eq!(Encoding::Keyboard.encode("\u{2423}").unwrap(), vec![0x09]);

    // The two conditions are not the same.
    assert!(Encoding::Keyboard.encode_is_lossy());
    assert!(!Encoding::Keyboard.is_directional());
}

#[test]
fn only_the_bidi_tables_are_directional() {
    let flagged: Vec<_> = mac_encoding::ALL
        .iter()
        .copied()
        .filter(|e| e.is_directional())
        .collect();
    assert_eq!(
        flagged,
        vec![Encoding::Arabic, Encoding::Farsi, Encoding::Hebrew]
    );
}

#[test]
fn ascii_is_not_assumed_to_be_transparent() {
    // Three encodings give other characters to the printable ASCII bytes. If
    // a later change puts the ASCII rule of the standard into the decoder,
    // this test fails.
    assert_eq!(Encoding::Symbol.decode(b"A"), "\u{0391}"); // GREEK CAPITAL ALPHA
    assert_ne!(Encoding::Dingbats.decode(b"a"), "a");
    assert_ne!(Encoding::Keyboard.decode(b"a"), "a");

    let reassigns =
        |e: Encoding| (0x20..=0x7Eu8).any(|b| e.decode(&[b]) != (b as char).to_string());
    let found: Vec<_> = mac_encoding::ALL
        .iter()
        .copied()
        .filter(|e| reassigns(*e))
        .collect();
    assert_eq!(
        found,
        vec![Encoding::Dingbats, Encoding::Keyboard, Encoding::Symbol]
    );
}

#[test]
fn two_byte_codes_win_over_one_byte_codes() {
    // Only the Indic tables have two-byte codes. In Mac OS Devanagari, the
    // code 0xE8+0xE9 is VIRAMA with ZWJ. Two one-byte codes give different
    // text.
    assert_eq!(
        Encoding::Devanagari.decode(&[0xE8, 0xE9]),
        "\u{094D}\u{200D}"
    );
    assert_ne!(
        Encoding::Devanagari.decode(&[0xE8, 0xE9]),
        format!(
            "{}{}",
            Encoding::Devanagari.decode(&[0xE8]),
            Encoding::Devanagari.decode(&[0xE9])
        )
    );
    assert_eq!(
        Encoding::Devanagari.encode("\u{094D}\u{200D}").unwrap(),
        vec![0xE8, 0xE9]
    );
}

#[test]
fn scalar_sequences_win_over_single_scalars() {
    // In Mac OS Symbol, byte 0xE2 is REGISTERED SIGN with Apple's variant
    // selector U+F87F. A REGISTERED SIGN alone is a different byte.
    assert_eq!(Encoding::Symbol.decode(&[0xE2]), "\u{00AE}\u{F87F}");
    assert_eq!(
        Encoding::Symbol.encode("\u{00AE}\u{F87F}").unwrap(),
        vec![0xE2]
    );
    assert_ne!(Encoding::Symbol.encode("\u{00AE}").unwrap(), vec![0xE2]);
}

#[test]
fn defines_every_byte_matches_the_tables() {
    // Almost all tables have a mapping for all 256 bytes. The font encodings
    // do not.
    for encoding in mac_encoding::ALL.iter().copied() {
        let complete = (0..=255u8).all(|b| encoding.decode_strict(&[b]).is_ok());
        assert_eq!(encoding.defines_every_byte(), complete, "{encoding}");
    }
    assert!(Encoding::Roman.defines_every_byte());
    assert!(!Encoding::Symbol.defines_every_byte());
}

#[test]
fn identifiers_are_unique_and_resolvable() {
    let mut ids: Vec<_> = mac_encoding::ALL.iter().map(|e| e.id()).collect();
    ids.sort_unstable();
    let count = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), count, "two encodings share an id");

    for encoding in mac_encoding::ALL.iter().copied() {
        assert_eq!(Encoding::from_id(encoding.id()), Some(encoding));
    }
    assert_eq!(
        Encoding::from_id("central-european"),
        Some(Encoding::CentralEuropean)
    );
    assert_eq!(Encoding::from_id("nonesuch"), None);
}
