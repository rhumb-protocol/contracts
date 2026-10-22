#!/usr/bin/env bash
# SECURITY.md in the root carries the short form of A-008 §4 test 1: the reporting route of
# A-008 §3.3, its five promises in short and the link to the page that holds the rules of
# record. It does not carry the text of A-008 §7.1, and no placeholder in angle brackets.
#
# The checks are one function, run on the file itself and on two forms it must refuse: the
# text of §7.1 as the spec in this tree holds it, and the short form with a placeholder added.
set -uo pipefail

# shellcheck source=scripts/tests/helpers.sh
. "$(cd "$(dirname "$0")" && pwd)/helpers.sh"

FILE="$REPO_DIR/SECURITY.md"
SPEC="$REPO_DIR/spec/A-008-bug-bounty.md"

# What a file needs, one fixed string per line: the route, then the five promises, then the link.
required=(
  'security@rhumb.finance'
  "GitHub's private"
  'vulnerability reporting'
  'Never in a public issue'
  'within 48 hours'
  'within 7 days'
  "the guardian's pause"
  'after 90 days'
  'No legal action'
  'https://rhumb.finance/security'
)

# The lead-ins of the paragraphs of §7.1 and its interim sentences: any one of them means the
# text of record was copied in.
refused=(
  '**Rhumb bug bounty.**'
  '**In scope:**'
  '**Out of scope:**'
  '**Awards:**'
  '**Report:**'
  '**Safe harbour:**'
  'No contract is in scope yet'
  'the list of known issues is published here'
)

# Prints one line per reason the file is not the short form, and nothing for a file that is.
findings() {
  local file="$1" text
  for text in "${required[@]}"; do
    grep -qF -- "$text" "$file" || echo "missing: $text"
  done
  for text in "${refused[@]}"; do
    ! grep -qF -- "$text" "$file" || echo "carries the text of record: $text"
  done
  ! LC_ALL=C grep -qE '<[a-z][a-z ]*>' "$file" || echo "carries a placeholder in angle brackets"
}

if [ -f "$FILE" ]; then
  pass "SECURITY.md exists"
else
  fail "SECURITY.md exists" "no such file: $FILE"
fi

found="$([ -f "$FILE" ] && findings "$FILE")"
if [ -f "$FILE" ] && [ -z "$found" ]; then
  pass "SECURITY.md carries the route, the five promises and the link, and no text of record"
else
  fail "SECURITY.md carries the route, the five promises and the link, and no text of record" \
    "${found:-the file is missing}"
fi

dir="$(scratch)"

# §7.1 from the spec, its quote markers taken off: the form the file must not take.
awk '/^### 7\.1 /{f=1; next} /^### 7\.2 /{f=0} f && /^>/' "$SPEC" | sed -E 's/^> ?//' \
  > "$dir/record.md"
if grep -qF '**Safe harbour:**' "$dir/record.md"; then
  pass "the text of A-008 §7.1 was read from the spec"
else
  fail "the text of A-008 §7.1 was read from the spec" "nothing taken from $SPEC"
fi
found="$(findings "$dir/record.md")"
if grep -qF 'carries the text of record' <<< "$found"; then
  pass "the text of A-008 §7.1 is refused"
else
  fail "the text of A-008 §7.1 is refused" "${found:-no finding}"
fi

# The short form with a placeholder of §7.1 added: refused for that alone.
{ [ -f "$FILE" ] && cat "$FILE"; echo '<contract ids>'; } > "$dir/placeholder.md"
found="$(findings "$dir/placeholder.md")"
if grep -qF 'carries a placeholder in angle brackets' <<< "$found"; then
  pass "a placeholder in angle brackets is refused"
else
  fail "a placeholder in angle brackets is refused" "${found:-no finding}"
fi

rm -rf "$dir"

report_counts
