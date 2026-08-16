# Changelog

This file gives the changes in each release. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). The version numbers
obey [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-08-16

No code changed in this release. The crate gives the same result as 0.1.0.

### Changed

- The README shows a crates.io badge and a docs.rs badge.

### Added

- A release workflow that publishes to crates.io with trusted publishing. The
  package does not contain this workflow.
- `CHANGELOG.md`.

## [0.1.0] - 2026-08-16

First release.

### Added

- 21 single-byte encodings from Apple's mapping files. They include Roman,
  Central European, Cyrillic, Greek, Arabic, Hebrew, and Thai. They also
  include the Indic encodings and the Symbol, Dingbats, and Keyboard font
  encodings.
- `Encoding::decode` and `Encoding::decode_strict` for the two decoder error
  modes of the WHATWG Encoding Standard.
- `Encoding::encode` and `Encoding::encode_html` for the two encoder error
  modes.
- `Encoding::from_label` for the two encodings that the standard names, and
  `Encoding::from_id` for the other 19.
- `Encoding::is_directional`, `Encoding::encode_is_lossy`, and
  `Encoding::defines_every_byte`, which give the unusual conditions in the
  tables.
- The module `macroman` for Mac OS Roman.
- Two conformance test suites. One compares each table with Apple's mapping
  file. The other compares Mac OS Roman with the index file of the standard.

### Notes

- The crate is `no_std` and uses `alloc`. It has no dependencies and no
  features. It needs Rust 1.81 or later.
- The repository does not contain Apple's mapping files. `SOURCES.lock` gives
  the address and the checksum of each file.

[Unreleased]: https://github.com/codybrom/mac-encoding/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/codybrom/mac-encoding/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/codybrom/mac-encoding/releases/tag/v0.1.0
