#!/usr/bin/env bash
# KNOWN-ISSUES.md in the root is the list of known issues the bounty page links to (A-008 §3.1,
# §7.1): the heading, the sentence that names the page and puts the listed issues out of the
# bounty's scope, one date line, and the issues, each on one line that ends on its reference to a
# section of a public spec. It carries no contract id and no address.
#
# The checks are one function, run on the file itself and on four forms it must refuse: an item
# without a reference, an item wrapped over two lines, a contract id and an address added.
set -uo pipefail

# shellcheck source=scripts/tests/helpers.sh
. "$(cd "$(dirname "$0")" && pwd)/helpers.sh"

FILE="$REPO_DIR/KNOWN-ISSUES.md"

# What a file needs, one fixed string per line besides the heading and the date.
required=(
  'https://rhumb.finance/security'
  'out of scope for the bug bounty'
  'A-008 §3.1'
)

# The end of an issue's line: its references in parentheses, one of them to a section of a spec,
# then the full stop; the class is written `[^)(]`, because the gate of check-ci-gates.sh reads
# an open and a close parenthesis side by side as a function definition. A contract id or an account: a strkey of 56 characters starting with C or G.
reference='[(][^)(]*[AE]-[0-9]{3} §[0-9][^)(]*[)][.]$'
strkey='(^|[^A-Z2-7])[CG][A-Z2-7]{55}([^A-Z2-7]|$)'

# Prints one line per reason the file is not in its form, and nothing for a file that is.
findings() {
  local file="$1" bad
  [ "$(head -n 1 "$file")" = '# Known issues' ] || echo "the first line is not the heading"
  for text in "${required[@]}"; do
    grep -qF -- "$text" "$file" || echo "missing: $text"
  done
  [ "$(LC_ALL=C grep -cE '^Date: [0-9]{4}-[0-9]{2}-[0-9]{2}$' "$file")" -eq 1 ] \
    || echo "not exactly one date line"
  grep -qE '^- ' "$file" || echo "lists no issue"
  bad="$(LC_ALL=C grep -nE '^- ' "$file" | LC_ALL=C grep -vE -- "$reference")"
  [ -z "$bad" ] || echo "an issue without a spec reference at its end: $bad"
  bad="$(LC_ALL=C grep -nE '^[[:space:]]+[^[:space:]]' "$file")"
  [ -z "$bad" ] || echo "an issue over more than one line: $bad"
  ! LC_ALL=C grep -qE -- "$strkey" "$file" || echo "carries a contract id or an account"
  ! grep -qF '@' "$file" || echo "carries an address"
}

if [ -f "$FILE" ]; then
  pass "KNOWN-ISSUES.md exists"
else
  fail "KNOWN-ISSUES.md exists" "no such file: $FILE"
fi

found="$([ -f "$FILE" ] && findings "$FILE")"
if [ -f "$FILE" ] && [ -z "$found" ]; then
  pass "KNOWN-ISSUES.md carries the heading, the page, one date and one referenced line per issue"
else
  fail "KNOWN-ISSUES.md carries the heading, the page, one date and one referenced line per issue" \
    "${found:-the file is missing}"
fi

dir="$(scratch)"

# A file in the form, written here, so that each refused form differs from it by one thing.
good="$dir/good.md"
cat > "$good" <<'EOF'
# Known issues

This is the list of known issues that https://rhumb.finance/security refers to. A report of an
issue listed here is out of scope for the bug bounty (A-008 §3.1, §7.1).

Date: 2026-10-31

- A pool refuses the same token on both sides (E-007 §2).
EOF
found="$(findings "$good")"
if [ -z "$found" ]; then
  pass "a file in the form is accepted"
else
  fail "a file in the form is accepted" "$found"
fi

# Each case: a line added at the end of the good file, and the reason it must be refused for.
check_refused() {
  local what="$1" line="$2" reason="$3"
  { cat "$good"; printf '%s\n' "$line"; } > "$dir/case.md"
  found="$(findings "$dir/case.md")"
  if grep -qF -- "$reason" <<< "$found"; then
    pass "$what is refused"
  else
    fail "$what is refused" "${found:-no finding}"
  fi
}

check_refused "an issue without a reference" '- A pool can be paused.' \
  'without a spec reference'
check_refused "an issue wrapped over two lines" '  continued (E-007 §2).' \
  'more than one line'
check_refused "a contract id" "- The router is C$(printf 'A%.0s' {1..55}) (E-007 §3.2)." \
  'contract id'
check_refused "an address" '- Write to someone@example.org (A-008 §3.3).' \
  'carries an address'

rm -rf "$dir"

report_counts
