#!/usr/bin/env bash
# scripts/sync-specs.sh reads its list from spec/LIST: it copies exactly the files the list names
# from governance, adds and updates, never deletes, and exits 1 on a name that is not a file in
# governance.
#
# Every case builds its own governance folder and its own copy of the script, so that neither
# the real governance nor this repository's spec/ is read or written.
set -uo pipefail

# shellcheck source=scripts/tests/helpers.sh
. "$(cd "$(dirname "$0")" && pwd)/helpers.sh"

SYNC="$REPO_DIR/scripts/sync-specs.sh"

# A tree in the shape the script works in: <dir>/repo with the script and spec/, <dir>/gov with
# specs/ holding three listed specs of three series, one spec the list does not name, and a
# spec of the E-000 series. spec/LIST is written by each case.
make_sync_tree() {
  local dir="$1" name
  mkdir -p "$dir/repo/scripts" "$dir/repo/spec" "$dir/gov/specs"
  cp "$SYNC" "$dir/repo/scripts/sync-specs.sh"
  for name in A-001-layout.md E-003-rate-provider.md X-001-corpus.md X-099-unlisted.md \
              E-000b-recipe.md; do
    printf '# %s\n\nthe text of %s\n' "$name" "$name" > "$dir/gov/specs/$name"
  done
}

# The three names a list of the default case holds, one per line.
listed=$'A-001-layout.md\nE-003-rate-provider.md\nX-001-corpus.md'

sync() {
  GOVERNANCE_DIR="$1/gov" bash "$1/repo/scripts/sync-specs.sh"
}

# The files of a spec/ folder, one per line, sorted.
files_of() {
  (cd "$1" && find . -type f | sed 's|^\./||' | sort)
}

# --- The list is read, and exactly its files are copied -----------------------------------------

dir="$(scratch)"
make_sync_tree "$dir"
printf '%s\n' "$listed" > "$dir/repo/spec/LIST"
out="$(sync "$dir" 2>&1)"; status=$?
if [ "$status" -eq 0 ]; then
  pass "a list of three names syncs"
else
  fail "a list of three names syncs" "status $status" "$out"
fi
expected=$'A-001-layout.md\nE-003-rate-provider.md\nLIST\nX-001-corpus.md'
found="$(files_of "$dir/repo/spec")"
if [ "$found" = "$expected" ]; then
  pass "spec/ holds the three listed specs and nothing the list does not name"
else
  fail "spec/ holds the three listed specs and nothing the list does not name" "found:" "$found"
fi
for name in A-001-layout.md E-003-rate-provider.md X-001-corpus.md; do
  if cmp -s "$dir/gov/specs/$name" "$dir/repo/spec/$name"; then
    pass "$name is copied unchanged"
  else
    fail "$name is copied unchanged"
  fi
  if grep -qxF "updated: spec/$name" <<< "$out"; then
    pass "the run reports [updated: spec/$name]"
  else
    fail "the run reports [updated: spec/$name]" "$out"
  fi
done

# --- A second run changes nothing and says so ----------------------------------------------------
#
# The copies are dated into the past and a reference file a minute after them; a run that
# rewrote any of them leaves it dated now, newer than the reference, on any file system's clock
# resolution.
for name in A-001-layout.md E-003-rate-provider.md X-001-corpus.md LIST; do
  touch -t 200001010000 "$dir/repo/spec/$name"
done
touch -t 200001010001 "$dir/reference"
out="$(sync "$dir" 2>&1)"; status=$?
if [ "$status" -eq 0 ] && [ "$out" = "specs up to date" ]; then
  pass "a second run prints [specs up to date] and exits 0"
else
  fail "a second run prints [specs up to date] and exits 0" "status $status" "$out"
fi
rewritten="$(find "$dir/repo/spec" -type f -newer "$dir/reference")"
if [ -z "$rewritten" ]; then
  pass "a second run writes no file"
else
  fail "a second run writes no file" "$rewritten"
fi

# --- An update in governance is copied, and only that one ---------------------------------------

printf 'an amendment\n' >> "$dir/gov/specs/E-003-rate-provider.md"
out="$(sync "$dir" 2>&1)"; status=$?
if [ "$status" -eq 0 ] && cmp -s "$dir/gov/specs/E-003-rate-provider.md" "$dir/repo/spec/E-003-rate-provider.md"; then
  pass "a changed spec in governance is copied again"
else
  fail "a changed spec in governance is copied again" "status $status" "$out"
fi
if grep -qxF "updated: spec/E-003-rate-provider.md" <<< "$out" \
   && ! grep -qF "updated: spec/A-001-layout.md" <<< "$out" \
   && ! grep -qF "updated: spec/X-001-corpus.md" <<< "$out"; then
  pass "only the changed spec is reported"
else
  fail "only the changed spec is reported" "$out"
fi
rm -rf "$dir"

# --- Nothing is ever deleted ---------------------------------------------------------------------

dir="$(scratch)"
make_sync_tree "$dir"
printf '%s\n' "$listed" > "$dir/repo/spec/LIST"
printf 'a spec governance has dropped\n' > "$dir/repo/spec/X-050-dropped.md"
out="$(sync "$dir" 2>&1)"; status=$?
if [ "$status" -eq 0 ] && [ -f "$dir/repo/spec/X-050-dropped.md" ]; then
  pass "a spec in spec/ that the list does not name stays"
else
  fail "a spec in spec/ that the list does not name stays" "status $status" "$out"
fi
rm -rf "$dir"

# --- What the script refuses, and that it then copies nothing -----------------------------------
#
# Every line is checked before the first file is copied, so a list with one bad line leaves
# spec/ exactly as it was.
refuses() {
  local what="$1" expected="$2" list="$3" dir out status found
  dir="$(scratch)"
  make_sync_tree "$dir"
  printf '%s\n' "$list" > "$dir/repo/spec/LIST"
  out="$(sync "$dir" 2>&1)"; status=$?
  if [ "$status" -ne 1 ]; then
    fail "$what exits 1" "status $status" "$out"
  elif ! grep -qiF -- "$expected" <<< "$out"; then
    fail "$what exits 1" "it stopped, but not for '$expected'" "$out"
  else
    pass "$what exits 1"
  fi
  found="$(files_of "$dir/repo/spec")"
  if [ "$found" = "LIST" ]; then
    pass "$what copies nothing"
  else
    fail "$what copies nothing" "found:" "$found"
  fi
  rm -rf "$dir"
}

refuses "a name that is not a file in governance" "X-404-missing.md" \
  "$listed"$'\nX-404-missing.md'
refuses "a spec of the E-000 series" "E-000b-recipe.md" \
  "$listed"$'\nE-000b-recipe.md'
refuses "a path instead of a name" "../outside.md" \
  "$listed"$'\n../outside.md'
refuses "a name in a folder" "specs/A-001-layout.md" \
  "$listed"$'\nspecs/A-001-layout.md'
refuses "a name that is not a Markdown spec" "A-001-layout.txt" \
  "$listed"$'\nA-001-layout.txt'
refuses "a name listed twice" "A-001-layout.md" \
  "$listed"$'\nA-001-layout.md'
refuses "an empty line" "line 4" \
  "$listed"$'\n\nX-099-unlisted.md'
refuses "a comment line" "# the specs" \
  $'# the specs\n'"$listed"
refuses "a name with a space after it" "A-001-layout.md " \
  $'A-001-layout.md \nE-003-rate-provider.md'

# A name in a locale whose ranges are not ASCII: under ISO 8859-1 on some systems `[A-Z]` matches
# lower case, and a line `e-000b-recipe.md` would then pass the name check, miss the E-000
# refusal and, on a case-insensitive file system, copy the E-000 spec. The check runs in the C
# locale; where this locale is not installed the run falls back to C and the assertion holds.
dir="$(scratch)"
make_sync_tree "$dir"
printf '%s\n' 'e-000b-recipe.md' > "$dir/repo/spec/LIST"
out="$(LC_ALL=en_US.ISO8859-1 GOVERNANCE_DIR="$dir/gov" bash "$dir/repo/scripts/sync-specs.sh" 2>&1)"; status=$?
if [ "$status" -eq 1 ] && grep -qF 'e-000b-recipe.md' <<< "$out" && [ "$(files_of "$dir/repo/spec")" = "LIST" ]; then
  pass "a lower-case name is refused in any locale and nothing is copied"
else
  fail "a lower-case name is refused in any locale and nothing is copied" "status $status" "$out"
fi
rm -rf "$dir"

# A list whose last line has no newline: the last name is synced like the others, never
# skipped by a read that stops at the end of the file.
dir="$(scratch)"
make_sync_tree "$dir"
printf '%s' "$listed" > "$dir/repo/spec/LIST"
out="$(sync "$dir" 2>&1)"; status=$?
expected=$'A-001-layout.md\nE-003-rate-provider.md\nLIST\nX-001-corpus.md'
if [ "$status" -eq 0 ] && [ "$(files_of "$dir/repo/spec")" = "$expected" ]; then
  pass "a list without a trailing newline syncs its last name too"
else
  fail "a list without a trailing newline syncs its last name too" "status $status" "$out" "$(files_of "$dir/repo/spec")"
fi
rm -rf "$dir"

# A list that is not empty and yet names nothing, a single NUL byte, is a stop as an empty list
# is, not a quiet "up to date".
dir="$(scratch)"
make_sync_tree "$dir"
printf '\0' > "$dir/repo/spec/LIST"
out="$(sync "$dir" 2>&1)"; status=$?
if [ "$status" -eq 1 ] && grep -qF "spec/LIST names no spec" <<< "$out" && [ "$(files_of "$dir/repo/spec")" = "LIST" ]; then
  pass "a list that names nothing exits 1 and copies nothing"
else
  fail "a list that names nothing exits 1 and copies nothing" "status $status" "$out"
fi
rm -rf "$dir"

# An empty list syncs nothing and says so rather than passing quietly.
dir="$(scratch)"
make_sync_tree "$dir"
: > "$dir/repo/spec/LIST"
out="$(sync "$dir" 2>&1)"; status=$?
if [ "$status" -eq 1 ] && grep -qiF "spec/LIST" <<< "$out"; then
  pass "an empty spec/LIST exits 1"
else
  fail "an empty spec/LIST exits 1" "status $status" "$out"
fi
rm -rf "$dir"

# No list at all is a stop, never a fall back to every A-spec.
dir="$(scratch)"
make_sync_tree "$dir"
out="$(sync "$dir" 2>&1)"; status=$?
if [ "$status" -eq 1 ] && grep -qiF "spec/LIST" <<< "$out" && [ -z "$(files_of "$dir/repo/spec")" ]; then
  pass "a missing spec/LIST exits 1 and copies nothing"
else
  fail "a missing spec/LIST exits 1 and copies nothing" "status $status" "$out"
fi
rm -rf "$dir"

# No governance folder is a stop.
dir="$(scratch)"
make_sync_tree "$dir"
printf '%s\n' "$listed" > "$dir/repo/spec/LIST"
out="$(GOVERNANCE_DIR="$dir/nowhere" bash "$dir/repo/scripts/sync-specs.sh" 2>&1)"; status=$?
if [ "$status" -eq 1 ] && grep -qiF "governance" <<< "$out"; then
  pass "a missing governance folder exits 1"
else
  fail "a missing governance folder exits 1" "status $status" "$out"
fi
rm -rf "$dir"

# --- This repository's list is well formed -------------------------------------------------------
#
# Which names it holds is this repository's own; here only that the file exists and that
# every line is one spec's file name and nothing else.
LIST="$REPO_DIR/spec/LIST"
if [ -s "$LIST" ]; then
  pass "spec/LIST exists and is not empty"
else
  fail "spec/LIST exists and is not empty"
fi
bad="$(LC_ALL=C grep -vnE '^[A-Z]-[0-9]{3}[a-z]?-[a-z0-9-]+\.md$' "$LIST" 2>/dev/null)"
if [ -f "$LIST" ] && [ -z "$bad" ]; then
  pass "every line of spec/LIST is one spec's file name"
else
  fail "every line of spec/LIST is one spec's file name" "$bad"
fi
dupes="$(sort "$LIST" 2>/dev/null | uniq -d)"
if [ -f "$LIST" ] && [ -z "$dupes" ]; then
  pass "spec/LIST names no spec twice"
else
  fail "spec/LIST names no spec twice" "$dupes"
fi

report_counts
