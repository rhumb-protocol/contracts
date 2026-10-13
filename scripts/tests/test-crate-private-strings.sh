#!/usr/bin/env bash
# The crates name nothing of the operational side: not the folder that holds a deployment's tables,
# not a deploy script, and the host program that posts the issuer adapters' rates only where it is
# the issuer adapter's role of that name: in adapter-issuer and in pool-integration, whose tests
# drive that role. The strings are built from halves, so that this public file does not carry them.
set -uo pipefail

# shellcheck source=scripts/tests/helpers.sh
. "$(cd "$(dirname "$0")" && pwd)/helpers.sh"

CRATES="$REPO_DIR/crates"

# grep exits 1 when it finds nothing and 2 when it could not read; only 0 and 1 are a verdict.
read_ok() {
  [ "$1" -le 1 ] || fail "grep read every file under crates/" "status $1"
}

# The role is still named where it is kept, so that the greps below are known to read the files.
if LC_ALL=C grep -rqF 'set_keeper' "$CRATES/adapter-issuer/src"; then
  pass "crates/adapter-issuer still names its role"
else
  fail "crates/adapter-issuer still names its role" "no set_keeper under $CRATES/adapter-issuer/src"
fi

tables="chain""-config"
hits="$(LC_ALL=C grep -rniF -- "$tables" "$CRATES")"
read_ok "$?"
if [ -z "$hits" ]; then
  pass "the crates never name the folder of a deployment's tables"
else
  fail "the crates never name the folder of a deployment's tables" "$hits"
fi

hits="$(LC_ALL=C grep -rniE -- 'deploy-[a-z0-9-]+\.sh' "$CRATES")"
read_ok "$?"
if [ -z "$hits" ]; then
  pass "the crates never name a deploy script"
else
  fail "the crates never name a deploy script" "$hits"
fi

program="kee""per"
hits="$(LC_ALL=C grep -rniF --exclude-dir=adapter-issuer --exclude-dir=pool-integration \
  -- "$program" "$CRATES")"
read_ok "$?"
if [ -z "$hits" ]; then
  pass "the host program is named only in adapter-issuer and pool-integration"
else
  fail "the host program is named only in adapter-issuer and pool-integration" "$hits"
fi

report_counts
