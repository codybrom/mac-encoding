#!/usr/bin/env bash
#
# Downloads the mapping files in SOURCES.lock into data/. Then checks each
# file against its checksum.
#
# The repository does not contain these files. Apple's files have the notice
# "all rights reserved" and give no permission to supply them to other
# persons. Thus the repository keeps the tables in src/tables.rs and the
# addresses in SOURCES.lock.
#
# You need this script only to write the tables again or to do the comparison
# tests. To build the crate or to use it, you need neither this script nor the
# directory data/.
#
#   ./scripts/fetch-sources.sh     # download the files and check them
#   cargo run -p generate-tables   # write src/tables.rs again
#   cargo test                     # the comparison tests now have the files
#
# Use --check to check the files in data/ but download nothing.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
lock="$root/SOURCES.lock"
data="$root/data"
check_only=false
[[ "${1:-}" == "--check" ]] && check_only=true

[[ -f "$lock" ]] || { echo "there is no SOURCES.lock at $lock" >&2; exit 1; }

# macOS has shasum. Almost all Linux systems have sha256sum.
if command -v shasum >/dev/null 2>&1; then
  digest() { shasum -a 256 "$1" | cut -d' ' -f1; }
elif command -v sha256sum >/dev/null 2>&1; then
  digest() { sha256sum "$1" | cut -d' ' -f1; }
else
  echo "this script needs shasum or sha256sum" >&2
  exit 1
fi

fetched=0
checked=0
failed=0

while read -r want path url; do
  # Do not read the comments and the empty lines.
  [[ -z "${want:-}" || "$want" == \#* ]] && continue

  dest="$data/$path"
  mkdir -p "$(dirname "$dest")"

  if [[ ! -f "$dest" ]]; then
    if $check_only; then
      echo "MISSING  $path"
      failed=$((failed + 1))
      continue
    fi
    curl -sSfL --max-time 60 -o "$dest" "$url" || {
      echo "FAILED   $path (download)" >&2
      failed=$((failed + 1))
      continue
    }
    fetched=$((fetched + 1))
  fi

  got="$(digest "$dest")"
  if [[ "$got" != "$want" ]]; then
    echo "MISMATCH $path" >&2
    echo "         expected $want" >&2
    echo "         got      $got" >&2
    echo "         Apple can change this file. Examine the change first." >&2
    echo "         Then correct SOURCES.lock and do: cargo run -p generate-tables" >&2
    failed=$((failed + 1))
  else
    checked=$((checked + 1))
  fi
done < "$lock"

echo "checked $checked file(s), downloaded $fetched, found $failed problem(s)"
[[ $failed -eq 0 ]] || exit 1
