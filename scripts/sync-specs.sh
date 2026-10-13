#!/usr/bin/env bash
# Pull the specs this repository lists in spec/LIST from the governance repo into spec/.
# Governance is the source of truth; this repo only reads from it.
# spec/LIST holds one spec's file name per line and nothing else: no empty line, no comment,
# no path. Every line is checked before the first file is copied, so a list with one bad line
# leaves spec/ as it was. A spec of the E-000 series is never copied (E-000b §3.7).
# This script adds and updates; it never deletes. A spec that governance drops is removed here
# by hand, in its own commit, with the reason in the pull request description.
# Usage: scripts/sync-specs.sh   (run from anywhere inside this repo)
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
GOV="${GOVERNANCE_DIR:-$REPO/../../governance}"
LIST="$REPO/spec/LIST"

stop() {
  echo "$1" >&2
  exit 1
}

[ -d "$GOV/specs" ] || stop "governance repo not found at $GOV (set GOVERNANCE_DIR to override)"
[ -f "$LIST" ] || stop "spec/LIST not found; it names the specs this repository copies, one per line"
[ -s "$LIST" ] || stop "spec/LIST names no spec"

names=()
number=0
while IFS= read -r line || [ -n "$line" ]; do
  number=$((number + 1))
  [ -n "$line" ] || stop "spec/LIST line $number is empty; the list holds one spec's file name per line and nothing else"
  # The C locale, so that the ranges are ASCII on every host: in some locales `[A-Z]` matches
  # lower case, and a lower-case `e-000...` would pass here and miss the E-000 refusal below.
  if ! LC_ALL=C grep -qxE '[A-Z]-[0-9]{3}[a-z]?-[a-z0-9-]+\.md' <<< "$line"; then
    stop "spec/LIST line $number, [$line], is not one spec's file name"
  fi
  case "$line" in
    E-000*) stop "spec/LIST line $number, [$line], names a spec of the E-000 series, which is never copied (E-000b §3.7)" ;;
  esac
  for seen in ${names[@]+"${names[@]}"}; do
    [ "$seen" != "$line" ] || stop "spec/LIST line $number, [$line], names a spec the list already names"
  done
  [ -f "$GOV/specs/$line" ] || stop "spec/LIST line $number, [$line], is not a file in governance's specs/"
  names+=("$line")
done < "$LIST"
# A list that is not empty and yet yields no line, such as a single NUL byte, names nothing.
[ "${#names[@]}" -gt 0 ] || stop "spec/LIST names no spec"

changed=0
for name in "${names[@]}"; do
  src="$GOV/specs/$name"
  dst="$REPO/spec/$name"
  if [ ! -e "$dst" ] || ! cmp -s "$src" "$dst"; then
    cp "$src" "$dst"
    echo "updated: spec/$name"
    changed=1
  fi
done

if [ "$changed" -eq 0 ]; then
  echo "specs up to date"
else
  echo "done; review with: git diff spec/  then commit as: docs: sync specs from governance"
fi
