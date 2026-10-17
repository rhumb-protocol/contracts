#!/usr/bin/env bash
# What scripts/scan-secrets.sh prints of the tree never forms a workflow command: the runner takes
# a line of the job's output that starts with `::` as one, and the scan prints the names and the
# lines it found, so every `::` in them is taken out and the stop names them without.
#
# The scan reads the repository it sits in, so every case is a git tree of its own under the
# system's temporary directory with a copy of the scan in it, made here and removed by its name.
set -uo pipefail

# shellcheck source=scripts/tests/helpers.sh
. "$(cd "$(dirname "$0")" && pwd)/helpers.sh"

COMMAND="::set-output name=x::y"
STRIPPED="set-output name=xy"
# A string shaped like a secret seed, built here so that this file holds none for the scan of
# this repository to find.
SEED="S$(printf 'A%.0s' $(seq 55))"

# A git tree with the scan in it and the files given as name and content pairs, added to the
# index, which is what the scan searches.
tree_of() {
  local dir="$1"
  shift
  mkdir -p "$dir/scripts"
  cp "$REPO_DIR/scripts/scan-secrets.sh" "$dir/scripts/"
  while [ "$#" -ge 2 ]; do
    printf '%s\n' "$2" > "$dir/$1"
    shift 2
  done
  git -C "$dir" init -q && git -C "$dir" add -A
}

dir="$(scratch)"
tree_of "$dir" notes.txt "a note"
assert_ok "a tree without a seed or an environment file passes" "$dir/scripts/scan-secrets.sh"
rm -rf "$dir"

dir="$(scratch)"
tree_of "$dir" "$COMMAND.env" "a value"
assert_stops_without_commands "an environment file named like a workflow command is named without its double colons" \
  "$STRIPPED.env" "$dir/scripts/scan-secrets.sh"
rm -rf "$dir"

dir="$(scratch)"
tree_of "$dir" notes.txt "$COMMAND $SEED"
assert_stops_without_commands "a seed on a line written like a workflow command is named without its double colons" \
  "notes.txt:1:$STRIPPED" "$dir/scripts/scan-secrets.sh"
rm -rf "$dir"

dir="$(scratch)"
tree_of "$dir" "$COMMAND" "$SEED"
assert_stops_without_commands "a seed in a file named like a workflow command is named without its double colons" \
  "$STRIPPED:1:" "$dir/scripts/scan-secrets.sh"
rm -rf "$dir"

report_counts
