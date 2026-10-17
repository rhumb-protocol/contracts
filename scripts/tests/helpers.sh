#!/usr/bin/env bash
# Assertions for the tests of the public scripts under scripts/ (a test of a public script
# lives here, a test of any other script beside that script).
#
# Every helper here is used by more than one test file or by the runner's contract with them;
# a check used by one test lives in that test.

# The repository, from this file's own place in it.
TESTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Used by the test files that source this one.
# shellcheck disable=SC2034
REPO_DIR="$(cd "$TESTS_DIR/../.." && pwd)"

passed=0
failed=0

# Prints a failure with its reason and remembers it; the runner exits non-zero when any test
# failed, so a test that reports and returns still fails the run.
fail() {
  failed=$((failed + 1))
  echo "  FAIL: $1"
  [ "$#" -gt 1 ] && printf '        %s\n' "${@:2}"
  return 0
}

pass() {
  passed=$((passed + 1))
  echo "  ok: $1"
}

# The last line of a test file: the counts the runner reads, and a status that is the verdict
# when the file is run on its own: a test that skips is a failure, never a pass.
report_counts() {
  echo "$passed $failed"
  [ "$failed" -eq 0 ]
}

# A command that must succeed, and a command that must fail for the reason it is meant to fail
# for: a stop is asserted on its message as well as on its status.
assert_ok() {
  local what="$1"; shift
  local out status
  out="$("$@" 2>&1)"; status=$?
  if [ "$status" -eq 0 ]; then
    pass "$what"
  else
    fail "$what" "expected success, got status $status" "$out"
  fi
}

assert_stops() {
  local what="$1" expected="$2"; shift 2
  local out status
  out="$("$@" 2>&1)"; status=$?
  if [ "$status" -eq 0 ]; then
    fail "$what" "expected a stop, the command succeeded" "$out"
  # Empty output matches no expectation, as it did when the output was piped into grep: a
  # here-string of nothing is still one empty line, which an empty pattern would match.
  elif [ -z "$out" ] || ! grep -qi -- "$expected" <<< "$out"; then
    fail "$what" "stopped, but not for '$expected'" "$out"
  else
    pass "$what"
  fi
}

# A stop that names what it read with the `::` and the `##[` taken out and prints neither: the
# runner takes a line of the job's output that starts with `::`, after its leading spaces, as a
# workflow command, and a `##[` anywhere in a line as one in its older form, so a gate never passes
# one on from what it read.
assert_stops_without_commands() {
  local what="$1" expected="$2"; shift 2
  local out status
  out="$("$@" 2>&1)"; status=$?
  if [ "$status" -eq 0 ]; then
    fail "$what" "expected a stop, the command succeeded" "$out"
  elif [ -z "$out" ] || ! grep -qF -- "$expected" <<< "$out"; then
    fail "$what" "stopped, but does not name '$expected'" "$out"
  elif grep -qF -- "::" <<< "$out"; then
    fail "$what" "stopped, but passes a :: on" "$out"
  elif grep -qF -- "##[" <<< "$out"; then
    fail "$what" "stopped, but passes a ##[ on" "$out"
  else
    pass "$what"
  fi
}

# A throwaway directory for the trees a test builds.
scratch() {
  mktemp -d "${TMPDIR:-/tmp}/rhumb-script-test.XXXXXX"
}
