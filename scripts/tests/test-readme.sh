#!/usr/bin/env bash
# README.md in the root tells a reader what they are looking at before they read the code: that
# the contracts are unaudited and every pool runs in a capped beta, in one paragraph that names
# KNOWN-ISSUES.md as the list those two head; and, under its heading on reporting a
# vulnerability, that the route, the promises and the link to the rules are in SECURITY.md. It
# does not say that the rules of the bounty still follow: they are published.
#
# The checks are one function, run on the file itself and on five forms it must refuse: the
# notice without the list, the two halves of the notice in separate paragraphs, the reporting
# section without SECURITY.md, SECURITY.md named outside that section, and the lapsed sentence.
set -uo pipefail

# shellcheck source=scripts/tests/helpers.sh
. "$(cd "$(dirname "$0")" && pwd)/helpers.sh"

FILE="$REPO_DIR/README.md"

heading='## Reporting a vulnerability'
lapsed='the full rules follow with the bug bounty'

# The paragraph of the file that carries a word, paragraphs being separated by blank lines.
paragraph_with() {
  awk -v word="$1" 'BEGIN { RS = "" } tolower($0) ~ word { print; exit }' "$2"
}

# The lines under the heading, up to the next heading of any level.
section() {
  awk -v heading="$1" '
    $0 == heading { on = 1; next }
    on && /^#/ { exit }
    on { print }
  ' "$2"
}

# Prints one line per reason the file is not in its form, and nothing for a file that is.
findings() {
  local file="$1" notice route
  notice="$(paragraph_with 'unaudited' "$file")"
  [ -n "$notice" ] || echo "missing: unaudited"
  grep -qi 'capped beta' <<< "$notice" || echo "the paragraph that says unaudited does not say capped beta"
  grep -qF 'KNOWN-ISSUES.md' <<< "$notice" || echo "the paragraph that says unaudited does not name KNOWN-ISSUES.md"
  grep -qxF -- "$heading" "$file" || echo "missing: $heading"
  route="$(section "$heading" "$file")"
  grep -qF 'SECURITY.md' <<< "$route" || echo "the section $heading does not name SECURITY.md"
  ! grep -qF -- "$lapsed" "$file" || echo "carries the lapsed sentence: $lapsed"
}

if [ -f "$FILE" ]; then
  pass "README.md exists"
else
  fail "README.md exists" "no such file: $FILE"
fi

found="$([ -f "$FILE" ] && findings "$FILE")"
if [ -f "$FILE" ] && [ -z "$found" ]; then
  pass "README.md carries the beta notice with its list and the route to SECURITY.md"
else
  fail "README.md carries the beta notice with its list and the route to SECURITY.md" \
    "${found:-the file is missing}"
fi

dir="$(scratch)"

# A file in the form, written here, so that each refused form differs from it by one thing.
good="$dir/good.md"
cat > "$good" <<'EOF'
# contracts

The rate adapters.
The contracts are unaudited and every pool runs in a capped beta; the list of known issues is
`KNOWN-ISSUES.md`.

## Layout
- `crates/` — the crates.

## Reporting a vulnerability
Report a vulnerability privately, never in a public issue; the route and the promises are in
`SECURITY.md`.

## Licence
MIT, see LICENSE.
EOF
found="$(findings "$good")"
if [ -z "$found" ]; then
  pass "a file in the form is accepted"
else
  fail "a file in the form is accepted" "$found"
fi

# Each case: the good file with one piece of text replaced by another, fixed strings on the
# first line that carries the first (awk, not sed: a newline in the replacement is spelled the
# same way on every platform), and the reason it must be refused for.
check_refused() {
  local what="$1" from="$2" to="$3" reason="$4"
  awk -v from="$from" -v to="$to" '
    !done { i = index($0, from) }
    !done && i { $0 = substr($0, 1, i - 1) to substr($0, i + length(from)); done = 1 }
    { print }
  ' "$good" > "$dir/case.md"
  if cmp -s "$good" "$dir/case.md"; then
    fail "$what is refused" "the case is the good file: nothing carries '$from'"
    return 0
  fi
  found="$(findings "$dir/case.md")"
  if grep -qF -- "$reason" <<< "$found"; then
    pass "$what is refused"
  else
    fail "$what is refused" "${found:-no finding}"
  fi
}

check_refused "the notice without the list" 'KNOWN-ISSUES.md' 'the list' \
  'does not name KNOWN-ISSUES.md'
check_refused "the two halves of the notice in separate paragraphs" \
  'unaudited and every' 'unaudited.\n\nEvery' \
  'does not say capped beta'
check_refused "the notice missing" 'unaudited' 'not yet audited' \
  'missing: unaudited'
check_refused "the reporting section without SECURITY.md" 'SECURITY.md' 'the policy' \
  'does not name SECURITY.md'
check_refused "SECURITY.md named only outside its section" \
  'Report a vulnerability privately' 'Report it privately.\n\n## Policy\nIt is' \
  'does not name SECURITY.md'
check_refused "the lapsed sentence" 'the promises are' \
  'the full rules follow with the bug bounty, and the promises are' \
  'carries the lapsed sentence'

rm -rf "$dir"

report_counts
