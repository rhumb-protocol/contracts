#!/usr/bin/env bash
# Fail when the tracked tree holds something that looks like a Stellar secret key, or an
# environment file. Keys live outside this repository; a human signs every deployment.
# The scan fails closed: an error from git is a failure, never a quiet "nothing found".
# Usage: scripts/scan-secrets.sh   (run from anywhere inside this repo)
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO"

# What the scan read and prints never forms a workflow command: the runner takes a line of the
# job's output that starts with `::`, after its leading spaces, as one, and a `##[` anywhere in a
# line as one in its older form, and the scan prints the names and the lines it found. So every
# `::` and `##[` in them is taken out, again until neither is left, since taking a marker out can
# join its neighbours into another (`####[[`, `:##[:`). Bytes, not characters, so that a line that
# is no valid text in the locale is still printed.
unmarked() {
  LC_ALL=C sed -e ':again' -e 's/##\[//g' -e 's/:://g' -e 't again'
}

status=0
out="$(mktemp)"
trap 'rm -f "$out"' EXIT

# A Stellar secret seed: S followed by 55 base32 characters. This script carries the pattern in
# two halves so that the scan never matches its own source; no file is excluded from the search,
# and files git treats as binary are searched too (--text).
seed_pattern="S[A-Z2-7]{5""5}"

set +e
git grep --text --line-number --extended-regexp "$seed_pattern" -- . >"$out" 2>&1
rc=$?
set -e
case "$rc" in
  0)
    echo "a string shaped like a Stellar secret key is tracked:"
    unmarked < "$out"
    status=1
    ;;
  1) ;;   # no match
  *)
    echo "the scan could not search the tree (git grep exit $rc):"
    unmarked < "$out"
    status=1
    ;;
esac

set +e
git ls-files >"$out" 2>&1
rc=$?
set -e
if [ "$rc" -ne 0 ]; then
  echo "the scan could not list the tracked files (git ls-files exit $rc):"
  unmarked < "$out"
  status=1
else
  # Any name that starts with .env (.env, .env.local, .envrc) or ends in .env (local.env),
  # at any depth, whatever its case: the runner's file system is case sensitive.
  envs="$(grep -i -E '(^|/)\.env|\.env$' "$out" || true)"
  if [ -n "$envs" ]; then
    echo "an environment file is tracked:"
    unmarked <<< "$envs"
    status=1
  fi
fi

if [ "$status" -eq 0 ]; then
  echo "no secrets found in the tracked tree"
fi
exit "$status"
