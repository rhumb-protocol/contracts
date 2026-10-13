#!/usr/bin/env bash
# The files under tests/fixtures/ are the bytes their README records: every file but the README,
# hidden files included, has one row in the README's table of files, and its sha256 and size are
# the row's; every row names a file that is there, no file is recorded twice and the README does
# not record itself.
set -uo pipefail

# shellcheck source=scripts/tests/helpers.sh
. "$(cd "$(dirname "$0")" && pwd)/helpers.sh"

FIXTURES="$REPO_DIR/tests/fixtures"

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  else
    shasum -a 256 "$1" | cut -d' ' -f1
  fi
}

size_of() {
  wc -c < "$1" | tr -d ' '
}

# The rows of the README's table of files, the table under `## The files` up to the next
# heading, header and separator left out: file, size and sha256, tab-separated, backticks
# removed.
recorded_rows() {
  awk '
    $0 == "## The files" { inside = 1; next }
    inside && /^#/ { inside = 0 }
    inside && /^\|/ {
      if ($0 ~ /^\|[- :|]+\|[ \t]*$/) next
      rows++
      if (rows == 1) next
      line = $0
      sub(/^\|[ \t]*/, "", line); sub(/[ \t]*\|[ \t]*$/, "", line)
      n = split(line, cell, /[ \t]*\|[ \t]*/)
      for (i = 1; i <= 3; i++) gsub(/`/, "", cell[i])
      print cell[1] "\t" cell[2] "\t" cell[3]
    }
  ' "$1"
}

# The check over one fixtures folder: silent and status 0 when it holds, one line per fault and
# status 1 when not.
check_fixtures() {
  local dir="$1" readme="$1/README.md" status=0 recorded file name row sha bytes twice
  if [ ! -f "$readme" ]; then
    echo "no README.md in $dir"
    return 1
  fi
  recorded="$(recorded_rows "$readme")"
  if [ -z "$recorded" ]; then
    echo "the README has no table of files under '## The files'"
    return 1
  fi

  twice="$(cut -f1 <<< "$recorded" | LC_ALL=C sort | uniq -d)"
  if [ -n "$twice" ]; then
    echo "the README records a file twice: $twice"
    status=1
  fi
  if [ -n "$(awk -F'\t' '$1 == "README.md"' <<< "$recorded")" ]; then
    echo "the README records itself"
    status=1
  fi

  for file in "$dir"/* "$dir"/.[!.]* "$dir"/..?*; do
    [ -e "$file" ] || continue
    name="$(basename "$file")"
    [ "$name" = "README.md" ] && continue
    row="$(awk -F'\t' -v f="$name" '$1 == f' <<< "$recorded" | head -n 1)"
    if [ -z "$row" ]; then
      echo "$name has no row in the README's table of files"
      status=1
      continue
    fi
    IFS=$'\t' read -r _ bytes sha <<< "$row"
    if [ "$(sha256_of "$file")" != "$sha" ]; then
      echo "$name has sha256 $(sha256_of "$file"), the README says $sha"
      status=1
    fi
    if [ "$(size_of "$file")" != "$bytes" ]; then
      echo "$name has $(size_of "$file") bytes, the README says $bytes"
      status=1
    fi
  done

  while IFS=$'\t' read -r name _; do
    [ -n "$name" ] || continue
    if [ ! -f "$dir/$name" ]; then
      echo "the README's table names $name, which is not there"
      status=1
    fi
  done <<< "$recorded"
  return "$status"
}

# A copy of the committed folder for the cases below, so that each case changes one thing.
case_tree() {
  local tree
  tree="$(scratch)"
  cp -R "$FIXTURES" "$tree/fixtures"
  printf '%s\n' "$tree"
}

# Rewrites the README of a case tree with sed, leaving no backup beside it in the folder.
edit_readme() {
  local readme="$2/fixtures/README.md"
  sed -E "$1" "$readme" > "$2/readme.new" && mv "$2/readme.new" "$readme"
}

# Whether the check stops on a tree for the reason given.
check_stops() {
  local what="$1" expected="$2" tree="$3" out
  if out="$(check_fixtures "$tree/fixtures")"; then
    fail "$what" "expected a stop, the check passed"
  elif ! grep -qiF -- "$expected" <<< "$out"; then
    fail "$what" "stopped, but not for '$expected'" "$out"
  else
    pass "$what"
  fi
  rm -rf "$tree"
}

# The committed folder, and that the check read what it is meant to read.
if out="$(check_fixtures "$FIXTURES")"; then
  pass "every file under tests/fixtures/ is the bytes its README records"
else
  fail "every file under tests/fixtures/ is the bytes its README records" "$out"
fi
if [ -f "$FIXTURES/pool_soroban.wasm" ] && [ -n "$(recorded_rows "$FIXTURES/README.md")" ]; then
  pass "tests/fixtures/ holds the pool Wasm and the README's table of files"
else
  fail "tests/fixtures/ holds the pool Wasm and the README's table of files"
fi

tree="$(case_tree)"
printf 'x' > "$tree/fixtures/router_soroban.wasm"
check_stops "a file the README does not record stops the check" \
  "router_soroban.wasm has no row" "$tree"

tree="$(case_tree)"
printf 'x' > "$tree/fixtures/.DS_Store"
check_stops "a hidden file the README does not record stops the check" \
  ".DS_Store has no row" "$tree"

# A name that starts with two dots is matched by none of the other two globs.
tree="$(case_tree)"
printf 'x' > "$tree/fixtures/..x"
check_stops "a file whose name starts with two dots and that the README does not record stops the check" \
  "..x has no row" "$tree"

tree="$(case_tree)"
rm "$tree/fixtures/pool_soroban.wasm"
check_stops "a row whose file is not there stops the check" "which is not there" "$tree"

# One byte changed in place: the size is the same, so only the hash can tell.
tree="$(case_tree)"
wasm="$tree/fixtures/pool_soroban.wasm"
byte="$(od -An -tx1 -j 100 -N 1 "$wasm" | tr -d ' \n')"
if [ "$byte" = "00" ]; then flip='\001'; else flip='\000'; fi
# shellcheck disable=SC2059
printf "$flip" | dd of="$wasm" bs=1 seek=100 conv=notrunc 2>/dev/null
check_stops "a byte changed in place stops the check on its hash" "has sha256" "$tree"

# The size in the README changed alone: the bytes and the hash are the recorded ones.
tree="$(case_tree)"
size="$(size_of "$FIXTURES/pool_soroban.wasm")"
edit_readme "s/^(\| \`pool_soroban\.wasm\` \| )$size /\1$((size + 1)) /" "$tree"
check_stops "a size the bytes do not have stops the check on the size" \
  "bytes, the README says $((size + 1))" "$tree"

tree="$(case_tree)"
edit_readme '/^\| .pool_soroban\.wasm. \|/{p;}' "$tree"
check_stops "a file recorded twice stops the check" "records a file twice: pool_soroban.wasm" "$tree"

tree="$(case_tree)"
readme_sha="$(sha256_of "$FIXTURES/README.md")"
edit_readme "/^\| \`pool_soroban\.wasm\` \|/a\\
| \`README.md\` | 1 | \`$readme_sha\` | | | | |
" "$tree"
check_stops "a row for the README itself stops the check" "the README records itself" "$tree"

tree="$(case_tree)"
edit_readme 's/^## The files$/## Files/' "$tree"
check_stops "a README without its table of files stops the check" "no table of files" "$tree"

report_counts
