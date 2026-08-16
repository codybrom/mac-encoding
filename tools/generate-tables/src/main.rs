//! Writes `src/tables.rs` again from Apple's mapping files.
//!
//! ```text
//! ./scripts/fetch-sources.sh        # downloads data/, checks the checksums
//! cargo run -p generate-tables      # rewrites src/tables.rs
//! cargo fmt && cargo test           # conformance tests now have data to check
//! ```
//!
//! This is a tool for development. It is not part of the build. The file
//! `src/tables.rs` is in the repository. Thus the crate builds without the
//! directory `data/` and without a build script. Use this tool only after
//! Apple changes a mapping file. Then commit the new `src/tables.rs` and the
//! new `SOURCES.lock` together.
//!
//! The repository does not contain Apple's mapping files. Each file has the
//! notice "all rights reserved" and gives no permission to supply the file to
//! other persons. Thus the repository keeps only the mappings. A mapping
//! tells you which byte shows which character. `SOURCES.lock` gives the
//! address and the checksum of each mapping file. Thus you can check each
//! file, but the repository does not supply it.
//!
//! # The format of a mapping file
//!
//! Each line has this form: `<code>\t<mapping>\t# comment`. A `#` character
//! starts a comment at any position. A `+` character joins the items in the
//! first column and in the second column:
//!
//! ```text
//! 0xB9        0x03C0                  # GREEK SMALL LETTER PI
//! 0xE8+0xE9   0x094D+0x200D           # VIRAMA + ZWJ, a two-byte code
//! 0xE2        0x00AE+0xF87F           # REGISTERED SIGN, sans serif variant
//! 0x2B        <LR>+0x002B             # PLUS SIGN, left-right context
//! 0x81                                # no mapping at all
//! ```
//!
//! A `<LR>` tag or an `<RL>` tag gives the direction of a character. Mac OS
//! Arabic, Farsi, and Hebrew have these tags. The tags are the reason that
//! more than one byte in those three tables has the same code point. This
//! tool removes the tag and keeps only the fact that the table has tags.
//! `Encoding::is_directional` gives you that fact.
//!
//! Apple does not include the bytes `0x00` to `0x1F` and the byte `0x7F`. The
//! mapping files say that this obeys "the conventions of the standard UTC
//! mapping tables". These bytes are the C0 control characters and DELETE.
//! Each of them gives the character with the same value. Thus this tool adds
//! them.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

/// The identity of one encoding: its mapping file and its names.
struct Spec {
    /// The name of the mapping file in `data/apple/`, without `.TXT`.
    stem: &'static str,
    /// The name of the Rust enum item to write.
    variant: &'static str,
    /// Apple's name for the encoding.
    apple_name: &'static str,
    /// The name in the standard, if the standard has one.
    whatwg_name: Option<&'static str>,
    /// The labels in the standard for this encoding.
    labels: &'static [&'static str],
}

/// The encodings to write, in Apple's sequence.
///
/// The standard has only two of them. Mac OS Roman is `macintosh` and Mac OS
/// Cyrillic is `x-mac-cyrillic`. The label `x-mac-ukrainian` also gives
/// `x-mac-cyrillic`, which agrees with Apple. The mapping file `UKRAINE.TXT`
/// tells us that Mac OS 9 put the Ukrainian characters into Mac OS Cyrillic.
/// The other 19 encodings have no name in the standard. You can find them by
/// their identifier only.
const ENCODINGS: &[Spec] = &[
    spec("ARABIC", "Arabic", "Mac OS Arabic"),
    spec("CELTIC", "Celtic", "Mac OS Celtic"),
    spec("CENTEURO", "CentralEuropean", "Mac OS Central European"),
    spec("CROATIAN", "Croatian", "Mac OS Croatian"),
    Spec {
        whatwg_name: Some("x-mac-cyrillic"),
        labels: &["x-mac-cyrillic", "x-mac-ukrainian"],
        ..spec("CYRILLIC", "Cyrillic", "Mac OS Cyrillic")
    },
    spec("DEVANAGA", "Devanagari", "Mac OS Devanagari"),
    spec("DINGBATS", "Dingbats", "Mac OS Dingbats"),
    spec("FARSI", "Farsi", "Mac OS Farsi"),
    spec("GAELIC", "Gaelic", "Mac OS Gaelic"),
    spec("GREEK", "Greek", "Mac OS Greek"),
    spec("GUJARATI", "Gujarati", "Mac OS Gujarati"),
    spec("GURMUKHI", "Gurmukhi", "Mac OS Gurmukhi"),
    spec("HEBREW", "Hebrew", "Mac OS Hebrew"),
    spec("ICELAND", "Icelandic", "Mac OS Icelandic"),
    spec("INUIT", "Inuit", "Mac OS Inuit"),
    spec("KEYBOARD", "Keyboard", "Mac OS Keyboard"),
    Spec {
        whatwg_name: Some("macintosh"),
        labels: &["csmacintosh", "mac", "macintosh", "x-mac-roman"],
        ..spec("ROMAN", "Roman", "Mac OS Roman")
    },
    spec("ROMANIAN", "Romanian", "Mac OS Romanian"),
    spec("SYMBOL", "Symbol", "Mac OS Symbol"),
    spec("THAI", "Thai", "Mac OS Thai"),
    spec("TURKISH", "Turkish", "Mac OS Turkish"),
];

/// Makes an encoding that has no name in the standard.
const fn spec(stem: &'static str, variant: &'static str, apple_name: &'static str) -> Spec {
    Spec {
        stem,
        variant,
        apple_name,
        whatwg_name: None,
        labels: &[],
    }
}

/// One line after the parser reads it: the bytes and the code points.
struct Mapping {
    code: Vec<u8>,
    text: String,
    /// True if the mapping had a `<LR>` tag or an `<RL>` tag.
    directional: bool,
}

fn parse(source: &str, file: &str) -> Vec<Mapping> {
    let mut out = Vec::new();
    for (lineno, raw) in source.lines().enumerate() {
        let line = raw.split('#').next().unwrap().trim();
        if line.is_empty() {
            continue;
        }
        let mut columns = line.split_whitespace();
        let code_column = columns.next().unwrap();
        // A line with no second column shows a byte with no mapping.
        let Some(mut mapping_column) = columns.next() else {
            continue;
        };

        let mut directional = false;
        if mapping_column.starts_with('<') {
            directional = true;
            match mapping_column.split_once('+') {
                Some((_tag, rest)) => mapping_column = rest,
                // A tag with no code point after it is not a mapping.
                None => continue,
            }
        }

        let code: Vec<u8> = code_column
            .split('+')
            .map(|b| {
                let hex = b.strip_prefix("0x").unwrap_or_else(|| {
                    panic!("{file}:{}: byte {b:?} is not 0x-prefixed", lineno + 1)
                });
                // A CJK mapping file writes a two-byte code as `0x8140`.
                let value = u32::from_str_radix(hex, 16)
                    .unwrap_or_else(|e| panic!("{file}:{}: byte {b:?}: {e}", lineno + 1));
                assert!(
                    hex.len() % 2 == 0,
                    "{file}:{}: odd-width code {b:?}",
                    lineno + 1
                );
                (0..hex.len() / 2)
                    .rev()
                    .map(move |i| (value >> (i * 8)) as u8)
                    .collect::<Vec<u8>>()
            })
            .fold(Vec::new(), |mut acc, bytes| {
                acc.extend(bytes);
                acc
            });

        let text: String = mapping_column
            .split('+')
            .map(|c| {
                let hex = c.strip_prefix("0x").unwrap_or_else(|| {
                    panic!("{file}:{}: scalar {c:?} is not 0x-prefixed", lineno + 1)
                });
                let value = u32::from_str_radix(hex, 16)
                    .unwrap_or_else(|e| panic!("{file}:{}: scalar {c:?}: {e}", lineno + 1));
                char::from_u32(value).unwrap_or_else(|| {
                    panic!("{file}:{}: U+{value:04X} is not a scalar", lineno + 1)
                })
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

fn main() {
    // This crate lives at <root>/tools/generate-tables.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the generator must sit two directories below the repository root");
    let mut generated = String::new();
    generated.push_str(
        "//! The tables for all the encodings.\n\
         //!\n\
         //! @generated. The command `cargo run -p generate-tables` writes this\n\
         //! file from Apple's mapping files. Do not change this file. Change\n\
         //! the generator, then write this file again.\n\
         //!\n\
         //! Each table tells you, for one encoding, which byte shows which\n\
         //! character. `SOURCES.lock` gives the address and the checksum of\n\
         //! each mapping file.\n\n\
         use crate::Table;\n\n",
    );

    let mut variants = Vec::new();

    for Spec {
        stem,
        variant,
        apple_name,
        whatwg_name,
        labels,
    } in ENCODINGS
    {
        let path = root.join("data/apple").join(format!("{stem}.TXT"));
        let source = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}\nDo ./scripts/fetch-sources.sh first.",
                path.display()
            )
        });
        let mappings = parse(&source, stem);

        // For the decoder: one byte, or two bytes for some Indic ligature
        // codes. An empty text shows a byte with no mapping. A mapping cannot
        // give an empty text, because each mapping has one code point or
        // more.
        let mut dec1 = vec![String::new(); 256];
        let mut dec2: BTreeMap<(u8, u8), String> = BTreeMap::new();

        // For the encoder: the opposite direction. The bytes are in sequence
        // from the lowest. Thus the lowest byte wins if more than one byte
        // gives the same text. This is how the tables with directions give
        // one byte again.
        let mut enc: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        let mut directional = false;
        // The text that more than one code gives. The encoder cannot know
        // which code made this text. Thus those bytes are not correct after
        // the two operations.
        let mut collisions: BTreeMap<String, usize> = BTreeMap::new();

        for m in &mappings {
            directional |= m.directional;
            *collisions.entry(m.text.clone()).or_default() += 1;
            match m.code[..] {
                [b] => {
                    if dec1[b as usize].is_empty() {
                        dec1[b as usize] = m.text.clone();
                    }
                }
                [lead, trail] => {
                    dec2.entry((lead, trail)).or_insert_with(|| m.text.clone());
                }
                ref other => panic!("{stem}: unsupported {}-byte code", other.len()),
            }
            enc.entry(m.text.clone()).or_insert_with(|| m.code.clone());
        }

        // Add the C0 control characters and DELETE that Apple does not
        // include. Add them only where the mapping file gives no mapping. The
        // mapping file must always win.
        //
        // Mac OS Keyboard gives key symbols to 22 of these bytes. For
        // example, byte 0x02 is U+21E5 LEFTWARDS ARROW TO BAR. That font has
        // no byte for the control character itself. Thus the encoder must not
        // get an entry for it. Without this rule, the text U+0002 encodes to
        // byte 0x02, and that byte decodes to U+21E5.
        for byte in (0x00..=0x1Fu8).chain(std::iter::once(0x7F)) {
            if !dec1[byte as usize].is_empty() {
                continue;
            }
            let control = char::from(byte).to_string();
            dec1[byte as usize] = control.clone();
            enc.entry(control).or_insert_with(|| vec![byte]);
        }

        // A mapping with one code point goes into a list for a binary
        // search. Only a few mappings have more than one code point. They go
        // into a short list, from the longest to the shortest. The encoder
        // examines that list first at each position. Thus the longest match
        // wins.
        let mut enc1: Vec<(char, Vec<u8>)> = Vec::new();
        let mut enc_seq: Vec<(String, Vec<u8>)> = Vec::new();
        for (text, code) in &enc {
            let mut chars = text.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => enc1.push((c, code.clone())),
                _ => enc_seq.push((text.clone(), code.clone())),
            }
        }
        enc1.sort_by_key(|(c, _)| *c);
        enc_seq.sort_by(|a, b| {
            b.0.chars()
                .count()
                .cmp(&a.0.chars().count())
                .then_with(|| a.0.cmp(&b.0))
        });

        writeln!(generated, "pub(crate) static {stem}: Table = Table {{").unwrap();
        writeln!(generated, "    id: {:?},", variant_id(variant)).unwrap();
        writeln!(generated, "    apple_name: {apple_name:?},").unwrap();
        match whatwg_name {
            Some(n) => writeln!(generated, "    whatwg_name: Some({n:?}),").unwrap(),
            None => writeln!(generated, "    whatwg_name: None,").unwrap(),
        }
        writeln!(generated, "    labels: &{labels:?},").unwrap();
        writeln!(generated, "    directional: {directional},").unwrap();
        let lossy = collisions.values().any(|n| *n > 1);
        writeln!(generated, "    lossy: {lossy},").unwrap();
        // `Encoding::defines_every_byte` reads this value. The generator
        // knows it already. Thus the crate does not examine 256 entries at
        // each call.
        let complete = dec1.iter().all(|text| !text.is_empty());
        writeln!(generated, "    complete: {complete},").unwrap();

        generated.push_str("    dec1: &[\n");
        for (byte, text) in dec1.iter().enumerate() {
            writeln!(generated, "        {text:?}, // 0x{byte:02X}").unwrap();
        }
        generated.push_str("    ],\n");

        generated.push_str("    dec2: &[\n");
        for ((lead, trail), text) in &dec2 {
            writeln!(
                generated,
                "        (0x{lead:02X}, 0x{trail:02X}, {text:?}),"
            )
            .unwrap();
        }
        generated.push_str("    ],\n");

        generated.push_str("    enc1: &[\n");
        for (c, code) in &enc1 {
            writeln!(generated, "        ({:?}, &{code:?}),", c).unwrap();
        }
        generated.push_str("    ],\n");

        generated.push_str("    enc_seq: &[\n");
        for (text, code) in &enc_seq {
            writeln!(generated, "        ({text:?}, &{code:?}),").unwrap();
        }
        generated.push_str("    ],\n};\n\n");

        variants.push((*variant, *stem));
    }

    generated.push_str("impl Encoding {\n    /// Gives the table for this encoding.\n");
    generated.push_str("    pub(crate) fn table(self) -> &'static Table {\n        match self {\n");
    for (variant, stem) in &variants {
        writeln!(generated, "            Encoding::{variant} => &{stem},").unwrap();
    }
    generated.push_str("        }\n    }\n}\n\n");

    generated.push_str("/// All the encodings in this crate, in Apple's sequence.\npub const ALL: &[Encoding] = &[\n");
    for (variant, _) in &variants {
        writeln!(generated, "    Encoding::{variant},").unwrap();
    }
    generated.push_str("];\n\n");

    generated.push_str("/// The encodings. The generator writes this list and the tables together,\n/// so the two always agree.\n");
    generated.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]\n#[non_exhaustive]\npub enum Encoding {\n");
    for (variant, _) in &variants {
        writeln!(generated, "    {variant},").unwrap();
    }
    generated.push_str("}\n");

    let out = root.join("src/tables.rs");
    std::fs::write(&out, generated).unwrap();
    println!("wrote {}", out.display());
    println!("now do `cargo fmt` and `cargo test` to check the result");
}

/// `CentralEuropean` -> `central-european`.
fn variant_id(variant: &str) -> String {
    let mut id = String::new();
    for (i, c) in variant.char_indices() {
        if c.is_uppercase() && i != 0 {
            id.push('-');
        }
        id.extend(c.to_lowercase());
    }
    id
}
