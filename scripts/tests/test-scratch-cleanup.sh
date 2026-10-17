#!/usr/bin/env bash
# Every test file removes the trees it made with scratch(), each by its name: a run of the suite
# leaves nothing of its own in the system's temporary directory, so that what is found there is
# never a test's to clean up by a pattern.
#
# Each file that calls scratch() is run here with a temporary directory of its own, made here and
# removed by its name, and holds that directory empty of scratch trees once it ends. Whether its
# cases pass is the file's own verdict, not this one's.
set -uo pipefail

# shellcheck source=scripts/tests/helpers.sh
. "$(cd "$(dirname "$0")" && pwd)/helpers.sh"

self="$(basename "$0")"
for file in "$TESTS_DIR"/test-*.sh; do
  name="$(basename "$file")"
  [ "$name" != "$self" ] || continue
  grep -qE '\$\(scratch\)' "$file" || continue
  tmp="$(scratch)"
  TMPDIR="$tmp" bash "$file" > /dev/null 2>&1
  left="$(find "$tmp" -mindepth 1 -maxdepth 1 -name 'rhumb-script-test.*' -exec basename {} \;)"
  rm -rf "$tmp"
  if [ -z "$left" ]; then
    pass "$name removes every scratch tree it made"
  else
    fail "$name removes every scratch tree it made" "$(wc -l <<< "$left" | tr -d ' ') left behind"
  fi
done

report_counts
