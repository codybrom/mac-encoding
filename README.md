# mac-encoding

[![crates.io](https://img.shields.io/crates/v/mac-encoding.svg)](https://crates.io/crates/mac-encoding)
[![docs.rs](https://docs.rs/mac-encoding/badge.svg)](https://docs.rs/mac-encoding)
[![CI](https://github.com/codybrom/mac-encoding/actions/workflows/ci.yml/badge.svg)](https://github.com/codybrom/mac-encoding/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

This crate translates text between classic Mac OS encodings and Unicode.

The crate has 21 single-byte encodings. They include Roman, Central European,
Cyrillic, Greek, Arabic, Hebrew, and Thai. They also include the Indic
encodings and the Symbol, Dingbats, and Keyboard font encodings. The decode
and encode operations obey the WHATWG
[Encoding Standard](https://encoding.spec.whatwg.org/).

The crate has no dependencies, no features, and no build script. It is
`no_std` and uses `alloc`. It needs Rust 1.81 or later.

```rust
use mac_encoding::Encoding;

assert_eq!(Encoding::Roman.decode(b"CODE"), "CODE");
assert_eq!(Encoding::Roman.encode("CODE").unwrap(), b"CODE");

// The standard calls Mac OS Roman `macintosh`.
assert_eq!(Encoding::from_label("X-Mac-Roman"), Some(Encoding::Roman));
```

Mac OS Roman is the encoding that most persons need. The module
[`macroman`](src/macroman.rs) is a short front for it.

```rust
use mac_encoding::macroman;

assert_eq!(macroman::decode(b"CODE"), "CODE");
```

## The source of the tables

**This repository does not contain the mapping files.** Apple's published mapping files are available at <https://www.unicode.org/Public/MAPPINGS/VENDORS/APPLE/>.

A mapping tells you which byte shows which character. A generator in this
crate writes the mappings into `src/tables.rs`.

[`SOURCES.lock`](SOURCES.lock) contains the address and known SHA-256 checksum of
each mapping file as of the last time that the tables were written.

You do not need the mapping files to build the crate or to use it. The file
`src/tables.rs` is in the repository and the crate builds from it when cloned.

### How to refresh the tables

You only need to do these steps if Apple changes a mapping file.

1. Download the mapping files into `data/`:

   ```sh
   ./scripts/fetch-sources.sh
   ```

   The script checks each file against its checksum in `SOURCES.lock`.

2. Write the tables again:

   ```sh
   cargo run -p generate-tables
   ```

3. Format the result and do the tests:

   ```sh
   cargo fmt && cargo test
   ```

To check the files in `data/` but not download them, use
`./scripts/fetch-sources.sh --check`.

A checksum that does not agree implies a source file has changed or moved.
Examine the change first. Then correct `SOURCES.lock` and do step 2 again.

## Tests

The command `cargo test` runs from a new clone. Most of the tests read only
`src/tables.rs`, which is in the repository.

Two test files also read the files in `data/`. `tests/apple_conformance.rs`
compares each table with its mapping file. `tests/whatwg_conformance.rs`
compares Mac OS Roman with the index file of the standard. Each of the two
files has its own parser, and neither parser uses the generator. If the
generator or one parser has a fault, the two results disagree.

These two files need the directory `data/`. Without it, they write a notice
and stop. To run them, download the mapping files first:

```sh
./scripts/fetch-sources.sh
```

The workflow `.github/workflows/ci.yml` downloads the files also. Its five
jobs do these operations:

- run the tests on the stable release and on Rust 1.81
- check the format with `cargo fmt` and the code with `cargo clippy`
- build the crate for a target that has no `std`
- write the tables again and compare the result with `src/tables.rs`
- package the crate with `cargo publish --dry-run`

## Unusual conditions in the tables

- **Three encodings give other characters to the printable ASCII bytes.**
  They are Mac OS Symbol, Dingbats, and Keyboard. Mac OS Symbol has GREEK
  CAPITAL LETTER ALPHA at byte `0x41`. The standard says that an ASCII byte
  decodes to itself. This rule is not correct for these three encodings. Thus
  the decoder always uses the table.

- **Byte `0xDB` in Mac OS Roman is `€`, not `¤`.** Mac OS 8.5 replaced the
  currency sign with the euro sign. Apple Technote TN1140 gives this change.
  RFC 1345 and the IANA `macintosh` registration show the mapping of 1991.
  Thus they do not agree with this crate.

- **Byte `0xF0` is the Apple logo.** Unicode has no character for it. The
  mapping uses U+F8FF, which is a private use character.

- **Four encodings cannot encode a byte again correctly.** They are Arabic,
  Farsi, Hebrew, and Keyboard. The first three give a direction to each
  mapping. Thus the left-to-right form and the right-to-left form of a
  character have the same code point. Keyboard has one such condition at
  U+2423. Apple's comment for that mapping says "hence no round-trip". The
  function `Encoding::encode_is_lossy` tells you which encodings do this. To
  encode text and then to decode it is always correct. Only the opposite
  sequence can give a different byte.

- **Mac OS Keyboard has no code for 22 control characters.** Apple's mapping
  files do not include the bytes `0x00` to `0x1F` and the byte `0x7F`. For all
  the other encodings, each of these bytes gives the control character with
  the same value. Mac OS Keyboard gives key symbols to 22 of them. For
  example, byte `0x02` is U+21E5 LEFTWARDS ARROW TO BAR. Thus `encode` gives
  an error for those 22 control characters.

## Encodings this crate does not yet have

The crate does not have the four double-byte CJK encodings. They are Japanese,
Simplified Chinese, Traditional Chinese, and Korean. Together they have about
38,000 more mappings. They also need a decoder for lead bytes and trail bytes.

The crate also has no Mac OS Ukrainian encoding because Apple/Unicode do not
supply a true mapping file for it. The file `UKRAINE.TXT` tells us that Mac OS
9 put the Ukrainian characters into Mac OS Cyrillic. This is the reason that
`x-mac-cyrillic` in the standard also has the label `x-mac-ukrainian`.

## License

You can use this crate under the MIT license or under the Apache License 2.0.
The two files are [LICENSE-MIT](LICENSE-MIT) and
[LICENSE-APACHE](LICENSE-APACHE).

The license covers the code in this repository. It does not cover Apple's
mapping files, which this repository does not contain. Refer to "The source of
the tables" above.
