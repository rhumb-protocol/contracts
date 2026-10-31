#!/usr/bin/env bash
# The public files name only the specs of the public list, the ones a reader of this repository
# has: A-001, A-007, A-008, E-003 and E-007. No ADR and no other spec is named: a rule an ADR set
# is stated in the comment itself, and a fact another spec holds is stated as a fact, so that
# nothing points at a page the reader cannot open. The crates first, then every public file.
set -uo pipefail

# shellcheck source=scripts/tests/helpers.sh
. "$(cd "$(dirname "$0")" && pwd)/helpers.sh"

CRATES="$REPO_DIR/crates"

# Any ADR, and the planner's and the router's specs by name. The names are built from halves,
# so that this public file does not carry them.
adr="AD""R-[0-9]"
hits="$(LC_ALL=C grep -rnE "$adr|E-0""06|E-0""08" "$CRATES")"
if [ -z "$hits" ]; then
  pass "the crates name no ADR and neither the planner's nor the router's spec"
else
  fail "the crates name no ADR and neither the planner's nor the router's spec" "$hits"
fi

# Any other spec name off the public list: every `X-nnn` of the A, E and I series that is not
# one of the five, at the start of a line or after a character that cannot be part of a name.
others="$(LC_ALL=C grep -rnoE '(^|[^A-Za-z0-9])[AEI]-[0-9]{3}[a-z]?' "$CRATES" \
  | LC_ALL=C grep -vE '[^0-9a-z](A-001|A-007|A-008|E-003|E-007)$')"
if [ -z "$others" ]; then
  pass "the crates name no spec off the public list"
else
  fail "the crates name no spec off the public list" "$others"
fi

# The same rule for every public file: the crates, the two READMEs, the root files, the public
# workflow, the public scripts and their tests name no spec off the public list and no ADR. One
# name is allowed, in one script and its test: the series the spec sync names because
# it never copies it. That name is built from halves too.
series="E-0""00"
public=("$REPO_DIR/crates" "$REPO_DIR/README.md" "$REPO_DIR/tests/fixtures/README.md"
        "$REPO_DIR/Cargo.toml" "$REPO_DIR/Cargo.lock" "$REPO_DIR/rust-toolchain.toml"
        "$REPO_DIR/.gitignore" "$REPO_DIR/LICENSE" "$REPO_DIR/SECURITY.md"
        "$REPO_DIR/KNOWN-ISSUES.md" "$REPO_DIR/.github/workflows/ci.yml"
        "$REPO_DIR/scripts/sync-specs.sh" "$REPO_DIR/scripts/scan-secrets.sh"
        "$REPO_DIR/scripts/check-ci-gates.sh" "$REPO_DIR/scripts/tests")
for path in "${public[@]}"; do
  if [ -e "$path" ]; then
    pass "${path#"$REPO_DIR"/} was read"
  else
    fail "${path#"$REPO_DIR"/} was read" "no such path"
  fi
done
found="$(LC_ALL=C grep -rnoE '(^|[^A-Za-z0-9])[AEI]-[0-9]{3}[a-z]?' "${public[@]}" \
  | LC_ALL=C grep -vE '[^0-9a-z](A-001|A-007|A-008|E-003|E-007)$')"
# The paths relative to the root, so that the exception is anchored there and no other file of
# that name passes.
found="${found//"$REPO_DIR"\//}"
others="$(LC_ALL=C grep -vE "^scripts/(sync-specs\\.sh|tests/test-sync-specs\\.sh):[0-9]+:([^A-Za-z0-9])?${series}b?\$" <<< "$found")"
if [ -z "$others" ]; then
  pass "no public file names a spec off the public list"
else
  fail "no public file names a spec off the public list" "$others"
fi
named="$(LC_ALL=C grep -rnE -- "$adr" "${public[@]}")"
if [ -z "$named" ]; then
  pass "no public file names an ADR"
else
  fail "no public file names an ADR" "$named"
fi

# The sync's own exception is still read, so that the filter above is known to match it.
if LC_ALL=C grep -qE "${series}b?" "$REPO_DIR/scripts/sync-specs.sh"; then
  pass "scripts/sync-specs.sh still names the series it never copies"
else
  fail "scripts/sync-specs.sh still names the series it never copies"
fi

# The public specs are still named where they were, so that the greps above are known to read
# the files: the issuer adapter's events and its event test name A-001 §3.3.
for file in adapter-issuer/src/events.rs adapter-issuer/src/test/t07_event_schema.rs; do
  if grep -qF 'A-001 §3.3' "$CRATES/$file"; then
    pass "$file names A-001 §3.3"
  else
    fail "$file names A-001 §3.3"
  fi
done

# The crates call the pool's third role by the pool's own name for it, never by the name of the
# program that holds its key. That name is built from halves, so that this public file does not
# carry it.
program="watch""tower"
named="$(LC_ALL=C grep -rniF -- "$program" "$CRATES")"
if [ -z "$named" ]; then
  pass "the crates never name the program that holds the pool's pause-only key"
else
  fail "the crates never name the program that holds the pool's pause-only key" "$named"
fi

report_counts
