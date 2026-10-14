#!/usr/bin/env bash
# The public scripts parse and pass shellcheck in ci.yml: `scripts/tests/`,
# `scripts/sync-specs.sh`, `scripts/scan-secrets.sh`, `scripts/check-ci-gates.sh` and
# `scripts/check-checkouts.sh`, with the shellcheck image pinned by tag and digest.
set -uo pipefail

# shellcheck source=scripts/tests/helpers.sh
. "$(cd "$(dirname "$0")" && pwd)/helpers.sh"

CI="$REPO_DIR/.github/workflows/ci.yml"

# The steps of the check job, read whole before they are searched.
check="$(awk '
  $0 == "  check:" { inside = 1; next }
  inside && /^  [a-z]/ { inside = 0 }
  inside { print }
' "$CI")"

# The step that lints, from its name up to the next step.
step="$(awk '
  /^      - name: The public scripts parse and pass shellcheck$/ { on = 1; print; next }
  on && /^      - / { on = 0 }
  on { print }
' <<< "$check")"

if [ -n "$step" ]; then
  pass "the check job has the step [The public scripts parse and pass shellcheck]"
else
  fail "the check job has the step [The public scripts parse and pass shellcheck]"
fi

image='koalaman/shellcheck:v0.10.0@sha256:2097951f02e735b613f4a34de20c40f937a6c8f18ecb170612c88c34517221fb'
if grep -qF -- "$image" <<< "$step"; then
  pass "the step runs shellcheck pinned by tag and digest"
else
  fail "the step runs shellcheck pinned by tag and digest" "expected $image"
fi
if grep -qF -- '--severity=style -x' <<< "$step"; then
  pass "the step runs shellcheck at style severity, following sources"
else
  fail "the step runs shellcheck at style severity, following sources"
fi
if grep -qF 'bash -n' <<< "$step"; then
  pass "the step parses every file with bash -n"
else
  fail "the step parses every file with bash -n"
fi

# The five names, each in the bash -n loop and in the shellcheck call, and nothing else.
targets='scripts/tests/*.sh scripts/sync-specs.sh scripts/scan-secrets.sh scripts/check-ci-gates.sh scripts/check-checkouts.sh'
count="$(grep -cF -- "$targets" <<< "$step")"
if [ "$count" = "2" ]; then
  pass "the step lints [$targets] in its bash -n loop and its shellcheck call"
else
  fail "the step lints [$targets] in its bash -n loop and its shellcheck call" "found it $count times"
fi
others="$(grep -oE 'scripts/[A-Za-z0-9._/*-]+' <<< "$step" | sort -u \
  | grep -vxE 'scripts/tests/\*\.sh|scripts/sync-specs\.sh|scripts/scan-secrets\.sh|scripts/check-ci-gates\.sh|scripts/check-checkouts\.sh')"
if [ -z "$others" ]; then
  pass "the step lints no other path"
else
  fail "the step lints no other path" "$others"
fi

for file in "$REPO_DIR"/scripts/sync-specs.sh "$REPO_DIR"/scripts/scan-secrets.sh \
            "$REPO_DIR"/scripts/check-ci-gates.sh "$REPO_DIR"/scripts/check-checkouts.sh; do
  if [ -f "$file" ]; then
    pass "$(basename "$file") is there to lint"
  else
    fail "$(basename "$file") is there to lint"
  fi
done

report_counts
