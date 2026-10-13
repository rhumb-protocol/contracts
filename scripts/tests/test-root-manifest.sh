#!/usr/bin/env bash
# The root workspace names only the crates: the host program that posts the issuer adapters' rates
# is a workspace of its own, so the root manifest needs no entry for it and names it nowhere, not
# in a member list, an exclude list or a comment. Its name is built from halves, so that this
# public file does not carry it.
set -uo pipefail

# shellcheck source=scripts/tests/helpers.sh
. "$(cd "$(dirname "$0")" && pwd)/helpers.sh"

MANIFEST="$REPO_DIR/Cargo.toml"

if grep -qF '[workspace]' "$MANIFEST"; then
  pass "Cargo.toml was read and holds the root workspace"
else
  fail "Cargo.toml was read and holds the root workspace" "no [workspace] in $MANIFEST"
fi

if grep -qF 'members = ["crates/*"]' "$MANIFEST"; then
  pass "the root workspace's members are the crates"
else
  fail "the root workspace's members are the crates"
fi

program="kee""per"
named="$(LC_ALL=C grep -niF -- "$program" "$MANIFEST")"
if [ -z "$named" ]; then
  pass "Cargo.toml never names the host program"
else
  fail "Cargo.toml never names the host program" "$named"
fi

report_counts
