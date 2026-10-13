#!/usr/bin/env bash
# Runs the tests of the public scripts under scripts/.
#
# Each file is its own process and reports its counts on its last line; the status of this
# runner is the verdict over all of them.
#
# Usage: scripts/tests/run.sh [name ...]   (a name is a test file without test- and .sh)
set -uo pipefail

TESTS_DIR="$(cd "$(dirname "$0")" && pwd)"

files=()
if [ "$#" -gt 0 ]; then
  for name in "$@"; do files+=("$TESTS_DIR/test-$name.sh"); done
else
  for f in "$TESTS_DIR"/test-*.sh; do [ -e "$f" ] && files+=("$f"); done
fi

if [ "${#files[@]}" -eq 0 ]; then
  echo "no test files found in $TESTS_DIR"
  exit 1
fi

total_passed=0
total_failed=0
for f in "${files[@]}"; do
  if [ ! -e "$f" ]; then
    echo "$(basename "$f"): no such test file"
    total_failed=$((total_failed + 1))
    continue
  fi
  echo "$(basename "$f"):"
  out="$(bash "$f" 2>&1)"
  echo "$out" | sed '$d'
  counts="$(echo "$out" | tail -n 1)"
  p="${counts%% *}"
  q="${counts##* }"
  case "$p$q" in
    *[!0-9]*)
      echo "  the file did not report its counts; it exited early"
      total_failed=$((total_failed + 1))
      continue
      ;;
  esac
  total_passed=$((total_passed + p))
  total_failed=$((total_failed + q))
done

echo
echo "$total_passed passed, $total_failed failed"
[ "$total_failed" -eq 0 ]
