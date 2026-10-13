#!/usr/bin/env bash
# The contract crates take no path outside themselves but the engine's Wasm in tests/fixtures/: no
# script and no pool file of a deployment is read or named in crates/pool-integration/, so that its
# tests build and pass where the crates stand alone.
set -uo pipefail

# shellcheck source=scripts/tests/helpers.sh
. "$(cd "$(dirname "$0")" && pwd)/helpers.sh"

CRATE="$REPO_DIR/crates/pool-integration"

if [ -d "$CRATE/src" ]; then
  pass "crates/pool-integration/ was read"
else
  fail "crates/pool-integration/ was read" "no src/ under $CRATE"
fi

hits="$(LC_ALL=C grep -rnE 'scripts/|pools/testnet' "$CRATE")"
if [ -z "$hits" ]; then
  pass "crates/pool-integration/ names no path under scripts/ and no pool file"
else
  fail "crates/pool-integration/ names no path under scripts/ and no pool file" "$hits"
fi

# The committed constructor values the public half of the test holds the encoder to.
for name in band-testnet-usdy.json band-testnet-usdy-sentinel.json stable-testnet-pyusd.json \
            stable-testnet-pyusd-sentinel.json rate-config-testnet-usdy.hex \
            rate-config-mainnet-shaped.hex rate-config-no-drift.hex; do
  if [ -s "$CRATE/src/test/fixtures/constructor-args/$name" ]; then
    pass "the committed value $name is there"
  else
    fail "the committed value $name is there"
  fi
done

report_counts
