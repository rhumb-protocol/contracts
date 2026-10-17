#!/usr/bin/env bash
# A green run can never mean that nothing ran: a required test that skips is a failure, not a
# pass, and a test script ends on the command whose exit status is its verdict.
#
# Two failure modes, both of which have happened here. A test that skips when its condition is
# missing is right on a laptop and wrong in CI, where the condition is always met; the
# `*_REQUIRED` flags turn those skips into failures, and a flag set to "0" -- or a flag nobody
# sets at all -- puts the skip back without anybody noticing. And a test file that ends on a
# bare `echo` exits 0 with any number of failures behind it, which is how a run was green while
# ten of its checks failed.
#
# `scripts/check-ci-gates.sh` is what CI runs to hold both of those. This file runs it against
# trees built for the purpose, and then against this repository, which is the assertion that
# matters: our own workflow raises every flag it has and our own test files end on their verdict.
#
# The check reads `.github/workflows/ci.yml`, `crates/` and `scripts/tests/` unless it is told
# other paths; the trees below are built in that shape, and the last cases hold that what lies
# outside it is read only when it is named.
set -uo pipefail

# shellcheck source=scripts/tests/helpers.sh
. "$(dirname "${BASH_SOURCE[0]}")/helpers.sh"

check="$REPO_DIR/scripts/check-ci-gates.sh"

# The flag names below are built from halves, the way ci.yml builds the names it greps for, so
# that this file's own fixtures are not read as flags of this repository when the check runs
# over the real tree at the end.
required="REQUIRED"
flag="ROLLOUT_$required"

# For the same reason the fixtures spell the word `trap` as TRAP and a here-document's `<<` as
# LTLT, and make_tree_with writes them out: the check reads every line of this file that carries
# the word, or a here-document delimiter, as code of this repository, and these are text. A
# definition behind `;`, `&&` or `then` is spelled with PARENS for its `()` or FUNCTION for its
# keyword, because the check refuses that shape wherever a line carries it.
trap_word="tr""ap"
heredoc_word="<""<"
parens_word="("")"
function_word="func""tion"

# A tree in the shape the check reads: a workflow, a test runner, a file CI runs directly, and a
# file the runner walks. The arguments say what to put wrong.
#   $1  the tree to build
#   $2  the value the workflow gives the flag ("1", "0", or "" for a workflow that never sets it)
#   $3  the last line of the file CI runs directly
#   $4  the last line of the file the runner walks
# The fixtures below are written verbatim into throwaway files; nothing in them is meant to
# expand here.
# shellcheck disable=SC2016
make_tree_with() {
  local dir="$1" value="$2" direct_end="$3" suite_end="$4"
  direct_end="${direct_end//TRAP/$trap_word}"
  direct_end="${direct_end//LTLT/$heredoc_word}"
  direct_end="${direct_end//PARENS/$parens_word}"
  direct_end="${direct_end//FUNCTION/$function_word}"
  suite_end="${suite_end//TRAP/$trap_word}"
  mkdir -p "$dir/.github/workflows" "$dir/scripts/tests"
  {
    echo "jobs:"
    echo "  check:"
    echo "    steps:"
    echo "      - name: The tests"
    echo "        run: scripts/tests/run.sh"
    echo "      - name: The direct test"
    echo "        run: scripts/tests/test-direct.sh"
    if [ -n "$value" ]; then
      echo "        env:"
      echo "          $flag: \"$value\""
    fi
  } > "$dir/.github/workflows/ci.yml"
  printf '#!/usr/bin/env bash\necho "the suite"\n[ "$failed" -eq 0 ]\n' > "$dir/scripts/tests/run.sh"
  printf '#!/usr/bin/env bash\n# a test CI runs directly\nif [ "${%s:-0}" = "1" ]; then :; fi\n%s\n' \
    "$flag" "$direct_end" > "$dir/scripts/tests/test-direct.sh"
  printf '#!/usr/bin/env bash\n%s\n' "$suite_end" > "$dir/scripts/tests/test-a-check.sh"
}

# The last line of a file the runner walks: literal text, written into the fixture as it stands.
# shellcheck disable=SC2016
counts='echo "$passed $failed"'

# --- A tree with nothing wrong ------------------------------------------------------------------
dir="$(scratch)"
make_tree_with "$dir" 1 "report_counts" "report_counts"
assert_ok "a workflow that raises its flag and test files that end on their verdict pass" \
  bash "$check" "$dir"
rm -rf "$dir"

# --- A flag at "0" -------------------------------------------------------------------------------
# The case the rule names: the condition is met in CI -- that is what CI is -- so a flag at "0" is
# a test that is allowed to skip in the one place it must not.
dir="$(scratch)"
make_tree_with "$dir" 0 "report_counts" "report_counts"
assert_stops "a required flag at 0 fails the run" "$flag" bash "$check" "$dir"
rm -rf "$dir"

# --- A flag nobody sets ---------------------------------------------------------------------------
# Worse than "0" and easier to do: the flag is read by a test, no step sets it, and the test
# skips in CI exactly as it skips on a laptop.
dir="$(scratch)"
make_tree_with "$dir" "" "report_counts" "report_counts"
assert_stops "a flag no CI step sets fails the run" "$flag" bash "$check" "$dir"
rm -rf "$dir"

# --- A file CI runs directly that ends on an echo ---------------------------------------------
dir="$(scratch)"
make_tree_with "$dir" 1 "$counts" "report_counts"
assert_stops "a file CI runs directly may not end on a command that always exits 0" \
  "test-direct.sh" bash "$check" "$dir"
rm -rf "$dir"

# --- A file CI runs directly is held to a list of endings, not a list of bad ones -----------------
# A list of the commands that always exit 0 misses every way of writing one that it does not
# name: `report_counts || true` ends on the right function and throws its status away, and so
# does anything followed by `; exit 0`. The check names the endings whose exit status is the
# verdict and refuses the rest, so an ending nobody thought of is a stop until somebody adds it
# on purpose. The allowed endings: `report_counts`, `exit "$var"`, and `[ "$var" -eq 0 ]`.
# shellcheck disable=SC2016
for ending in 'report_counts || true' 'report_counts || :' 'report_counts; exit 0' \
  'report_counts && true' '[ "$failed" -eq 0 ] || true' 'exit "$status" || true' \
  'exit 0' 'true' ':' 'run_everything' 'report_counts &' 'report_counts | tee counts' \
  '[ "$failed" -ne 1 ]' 'exit "$status"; true'; do
  dir="$(scratch)"
  make_tree_with "$dir" 1 "$ending" "report_counts"
  assert_stops "a file CI runs directly may not end on [$ending]" \
    "test-direct.sh" bash "$check" "$dir"
  rm -rf "$dir"
done

# shellcheck disable=SC2016
for ending in 'report_counts' 'exit "$status"' 'exit $status' '[ "$failed" -eq 0 ]'; do
  dir="$(scratch)"
  make_tree_with "$dir" 1 "$ending" "report_counts"
  assert_ok "a file CI runs directly may end on [$ending]" bash "$check" "$dir"
  rm -rf "$dir"
done

# --- The ending is the last command only when the file reaches it as one ---------------------------
# The allowed endings are matched against the last non-comment line, and four things make that
# line something other than the file's last command, each with `report_counts` as the text on
# that line. A `trap ... EXIT` earlier in the file runs after it and `exit 0` there is the
# status the file ends on, whatever report_counts said (the EXIT trap is also spelled `0`). A
# line that ends on `||` or on a backslash continues onto the last line, so the last command is
# `true || report_counts` (never run) or `echo done report_counts` (an echo). A here-document
# that is never terminated runs to the end of the file, so the last line is text and the last
# command is the `:` that reads it. Each of these ends on 0 with a failure behind it; each is a
# stop.
# shellcheck disable=SC2016
for ending in "TRAP 'exit 0' EXIT"$'\n'"report_counts" "TRAP 'exit 0' 0"$'\n'"report_counts" \
  "true ||"$'\n'"report_counts" "echo done \\"$'\n'"report_counts" ": LTLT'X'"$'\n'"report_counts"; do
  dir="$(scratch)"
  make_tree_with "$dir" 1 "$ending" "report_counts"
  assert_stops "a file CI runs directly may not reach report_counts through [${ending%%$'\n'*}]" \
    "test-direct.sh" bash "$check" "$dir"
  rm -rf "$dir"
done

# The same three in the other spellings bash accepts: `trap --`, the signal name in lower case,
# an action that is `exit` alone, a function named by the trap in either form (`f()` and
# `function f`) whose body calls exit, and the continuations `|` and `&&`.
# shellcheck disable=SC2016
for ending in "TRAP -- 'exit 0' EXIT"$'\n'"report_counts" "TRAP 'exit 0' exit"$'\n'"report_counts" \
  'TRAP "exit" INT EXIT'$'\n'"report_counts" \
  $'done_() {\n  exit 0\n}\nTRAP done_ EXIT\nreport_counts' \
  $'function done_ {\n  exit 0\n}\nTRAP done_ EXIT\nreport_counts' \
  "echo done |"$'\n'"report_counts" "true &&"$'\n'"report_counts"; do
  dir="$(scratch)"
  make_tree_with "$dir" 1 "$ending" "report_counts"
  assert_stops "a file CI runs directly may not reach report_counts through [$(printf '%s' "$ending" | tr '\n' '/')]" \
    "test-direct.sh" bash "$check" "$dir"
  rm -rf "$dir"
done

# A trap that only cleans up keeps the status of the command before it, and is how the suite's
# own files end: it stays allowed, and so does a function it names that returns without exit.
# shellcheck disable=SC2016
for ending in "TRAP 'rm -rf \"\$work\"' EXIT"$'\n'"report_counts" \
  $'cleanup() {\n  rm -rf "$work"\n  return 0\n}\nTRAP cleanup EXIT\nreport_counts' \
  "TRAP 'exit 0' INT"$'\n'"report_counts"; do
  dir="$(scratch)"
  make_tree_with "$dir" 1 "$ending" "report_counts"
  assert_ok "a file CI runs directly may reach report_counts past [$(printf '%s' "$ending" | tr '\n' '/')]" \
    bash "$check" "$dir"
  rm -rf "$dir"
done

# The same again in the shapes a reading of lines can still see, each of which was run and ends
# the file on 0 with a failure behind it: the word `trap` after another command on its line, or
# behind `builtin`, `command` or a backslash; the signal quoted, followed by `;`, or written
# `00`; an ERR or DEBUG trap, which fires without a signal (ERR on the `[ ... ]` ending, with no
# function in between); `exec` in the action, which replaces the shell and its status; a
# function named with an argument, a function that reaches `exit` through a second function,
# and a function body with a `}` line of its own before its `exit`; an action that spans lines;
# a here-document whose delimiter is the file's last line; and a continuation with a comment
# behind it. A trap the gate cannot read is a stop, like an ending it does not know.
# shellcheck disable=SC2016
for ending in "true; TRAP 'exit 0' EXIT"$'\n'"report_counts" \
  "builtin TRAP 'exit 0' EXIT"$'\n'"report_counts" \
  "\\TRAP 'exit 0' EXIT"$'\n'"report_counts" \
  "TRAP 'exit 0' \"EXIT\""$'\n'"report_counts" \
  "TRAP 'exit 0' EXIT;"$'\n'"report_counts" \
  "TRAP 'exit 0' 00"$'\n'"report_counts" \
  "TRAP 'exit 0' ERR"$'\n''[ "$failed" -eq 0 ]' \
  "TRAP 'exit 0' DEBUG"$'\n'"report_counts" \
  "TRAP 'exec true' EXIT"$'\n'"report_counts" \
  $'cleanup() {\n  exit 0\n}\nTRAP \'cleanup "$work"\' EXIT\nreport_counts' \
  $'fin() {\n  exit 0\n}\ncleanup() {\n  fin\n}\nTRAP cleanup EXIT\nreport_counts' \
  $'cleanup() {\n  {\n    :\n  }\n  exit 0\n}\nTRAP cleanup EXIT\nreport_counts' \
  $'TRAP \'\n  exit 0\n\' EXIT\nreport_counts' \
  ": LTLT'report_counts'"$'\n'"text"$'\n'"report_counts" \
  "true || # never"$'\n'"report_counts"; do
  dir="$(scratch)"
  make_tree_with "$dir" 1 "$ending" "report_counts"
  assert_stops "a file CI runs directly may not reach its ending through [$(printf '%s' "$ending" | tr '\n' '/')]" \
    "test-direct.sh" bash "$check" "$dir"
  rm -rf "$dir"
done

# The idiom those shapes must not take with them: a cleanup function named with its argument
# that returns.
# shellcheck disable=SC2016
dir="$(scratch)"
make_tree_with "$dir" 1 $'cleanup() {\n  rm -rf "$1"\n}\nTRAP \'cleanup "$work"\' EXIT\nreport_counts' "report_counts"
assert_ok "a file CI runs directly may reach report_counts past a cleanup named with an argument" \
  bash "$check" "$dir"
rm -rf "$dir"

# And the security reviewer's: a compound action whose second command is a function that exits
# (the spelling test-install-service.sh uses, with a function that returns); a function written
# on one line; a function body too large for one pipe buffer, where a check that pipes the body
# into `grep -q` under pipefail loses the `exit` it is looking for.
# shellcheck disable=SC2016
filler="$(for _ in $(seq 1 1500); do printf '  : "a line of a long cleanup that fills the pipe"\n'; done)"
for ending in $'done_() {\n  exit 0\n}\nTRAP \'rm -rf "$work"; done_\' EXIT\nreport_counts' \
  $'done_() { exit 0; }\nTRAP done_ EXIT\nreport_counts' \
  $'cleanup() {\n  exit 0\n'"$filler"$'\n}\nTRAP cleanup EXIT\nreport_counts'; do
  dir="$(scratch)"
  make_tree_with "$dir" 1 "$ending" "report_counts"
  assert_stops "a file CI runs directly may not reach its ending through [$(printf '%s' "$ending" | head -n 3 | tr '\n' '/')]" \
    "test-direct.sh" bash "$check" "$dir"
  rm -rf "$dir"
done

# What stays allowed: the suite's own compound cleanup, a one-line cleanup with the word exit
# later in the file (the body of a one-line function is its own line), and a comment in a
# cleanup that mentions exit.
# shellcheck disable=SC2016
for ending in $'remove_stand_in() {\n  rm -f "$work/x"\n  return 0\n}\nTRAP \'rm -rf "$work"; remove_stand_in\' EXIT\nreport_counts' \
  $'cleanup() { rm -rf "$work"; }\nTRAP cleanup EXIT\necho "the word exit in a message"\nreport_counts' \
  $'cleanup() {\n  rm -rf "$work"  # never exit here\n}\nTRAP cleanup EXIT\nreport_counts'; do
  dir="$(scratch)"
  make_tree_with "$dir" 1 "$ending" "report_counts"
  assert_ok "a file CI runs directly may reach report_counts past [$(printf '%s' "$ending" | head -n 3 | tr '\n' '/')]" \
    bash "$check" "$dir"
  rm -rf "$dir"
done

# A tab inside the body of a function a trap names, as an indent or before a comment: the body
# is read whole, tabs included, and its `exit` is found. A tab-indented cleanup that returns
# stays allowed.
# shellcheck disable=SC2016
for ending in $'cleanup() {\n\texit 0\n}\nTRAP cleanup EXIT\nreport_counts' \
  $'cleanup() {\n  rm -rf "$work"\t# tidy\n  exit 0\n}\nTRAP cleanup EXIT\nreport_counts'; do
  dir="$(scratch)"
  make_tree_with "$dir" 1 "$ending" "report_counts"
  assert_stops "a file CI runs directly may not reach its ending through [$(printf '%s' "$ending" | head -n 3 | tr '\n\t' '/>')]" \
    "test-direct.sh" bash "$check" "$dir"
  rm -rf "$dir"
done
dir="$(scratch)"
make_tree_with "$dir" 1 $'cleanup() {\n\trm -rf "$work"\n}\nTRAP cleanup EXIT\nreport_counts' "report_counts"
assert_ok "a file CI runs directly may reach report_counts past a tab-indented cleanup that returns" \
  bash "$check" "$dir"
rm -rf "$dir"

# A definition of the function a trap names that the gate's reading of bodies does not take:
# indented (inside an `if`), nested inside another function, with its `{` on the next line, or
# closed by an indented `}` so that the next function at column 0 is read as its tail. Each
# one is on a line the gate can see is a definition, so like a trap it cannot read it is a
# stop. When a function is defined twice, bash keeps the last definition, and so must the gate.
# And a here-document's delimiter may be quoted with a backslash, `LTLT\word`, which is the
# same delimiter as `LTLT'word'`.
# shellcheck disable=SC2016
for ending in $'if :; then\n  cleanup() {\n    exit 0\n  }\nfi\nTRAP cleanup EXIT\nreport_counts' \
  $'outer() {\n  cleanup() {\n    exit 0\n  }\n}\nouter\nTRAP cleanup EXIT\nreport_counts' \
  $'cleanup()\n{\n  exit 0\n}\nTRAP cleanup EXIT\nreport_counts' \
  $'cleanup() {\n  :\n  }\nother() {\n  exit 0\n}\nTRAP other EXIT\nreport_counts' \
  $'cleanup() {\n  return 0\n}\ncleanup() {\n  exit 0\n}\nTRAP cleanup EXIT\nreport_counts' \
  ": LTLT\\report_counts"$'\n'"text"$'\n'"report_counts"; do
  dir="$(scratch)"
  make_tree_with "$dir" 1 "$ending" "report_counts"
  assert_stops "a file CI runs directly may not reach its ending through [$(printf '%s' "$ending" | head -n 4 | tr '\n' '/')]" \
    "test-direct.sh" bash "$check" "$dir"
  rm -rf "$dir"
done

# A continuation whose line carries a `#` inside a string: the comment strip cuts the line at
# that `#`, and the operator at its end must still be seen. And helpers.sh, which every test
# file sources: a trap there applies to all of them, so its traps are read like a test file's.
# shellcheck disable=SC2016
dir="$(scratch)"
make_tree_with "$dir" 1 'echo "a #b" ||'$'\n'"report_counts" "report_counts"
assert_stops "a file CI runs directly may not reach report_counts through a continuation behind a # in a string" \
  "test-direct.sh" bash "$check" "$dir"
rm -rf "$dir"
dir="$(scratch)"
make_tree_with "$dir" 1 "report_counts" "report_counts"
printf '%s\n' "$trap_word 'exit 0' EXIT" > "$dir/scripts/tests/helpers.sh"
assert_stops "the traps of helpers.sh are read like the traps of a test file" \
  "helpers.sh" bash "$check" "$dir"
rm -rf "$dir"

# Three readings of a function body that end it before its `exit`, each found in review and
# each run: a definition line whose last `}` belongs to a `${...}` expansion or to a comment,
# read as a function written on one line; and a `}` at the definition's own indent that closes
# an inner group or is a line of a here-document, read as the end of the body.
# shellcheck disable=SC2016
for ending in $'cleanup() { local d=${work}\n  exit 0\n}\nTRAP cleanup EXIT\nreport_counts' \
  $'cleanup() { # tidy {}\n  exit 0\n}\nTRAP cleanup EXIT\nreport_counts' \
  $'cleanup() {\n  {\n    :\n}\n  exit 0\n}\nTRAP cleanup EXIT\nreport_counts' \
  $'cleanup() {\n  cat LTLT\'E\'\n}\nE\n  exit 0\n}\nTRAP cleanup EXIT\nreport_counts'; do
  dir="$(scratch)"
  make_tree_with "$dir" 1 "$ending" "report_counts"
  assert_stops "a file CI runs directly may not reach its ending through [$(printf '%s' "$ending" | head -n 4 | tr '\n' '/')]" \
    "test-direct.sh" bash "$check" "$dir"
  rm -rf "$dir"
done

# What stays allowed beside them: a one-line cleanup that ends on a `${...}` and closes, and a
# cleanup with an inner group at a deeper indent that returns.
# shellcheck disable=SC2016
for ending in $'cleanup() { rm -rf "${work}"; }\nTRAP cleanup EXIT\necho "the word exit in a message"\nreport_counts' \
  $'cleanup() {\n  {\n    rm -rf "$work"\n  }\n}\nTRAP cleanup EXIT\necho "the word exit in a message"\nreport_counts'; do
  dir="$(scratch)"
  make_tree_with "$dir" 1 "$ending" "report_counts"
  assert_ok "a file CI runs directly may reach report_counts past [$(printf '%s' "$ending" | head -n 3 | tr '\n' '/')]" \
    bash "$check" "$dir"
  rm -rf "$dir"
done

# A trap in helpers.sh that names a function helpers.sh does not define: bash looks the name up
# when the trap fires, in the shell of the test file that sourced helpers.sh, so the function
# that runs is the test file's, and its `exit` is the file's status.
# shellcheck disable=SC2016
dir="$(scratch)"
make_tree_with "$dir" 1 $'cleanup() {\n  exit 0\n}\nreport_counts' "report_counts"
printf '%s\n' "$trap_word cleanup EXIT" > "$dir/scripts/tests/helpers.sh"
assert_stops "a $trap_word of helpers.sh is followed into the functions of the test file that sources it" \
  "test-direct.sh" bash "$check" "$dir"
rm -rf "$dir"

# Three more readings that end a function body before bash does, queued from the review of the
# reading above, each run and each ending the file on 0 with a failure behind it. A closing line
# that is not a bare `}` -- `} 2>/dev/null`, `} # end`, `exit 0; }`, an indented `}` -- behind
# a `}` line at the definition's indent inside the body, which is then read as the end. A line
# shaped like a definition inside a here-document or a multi-line string, which ends the scan
# before the real `}`. And definitions the gate did not see at all: after `;`, `&&` or `then`,
# written `name ( ) {`, or with a name outside `[A-Za-z_][A-Za-z0-9_]*`, which bash accepts and
# a trap can name.
# shellcheck disable=SC2016
for ending in $'cleanup() {\n  {\n    :\n}\n  exit 0\n} 2>/dev/null\nTRAP cleanup EXIT\nreport_counts' \
  $'cleanup() {\n  {\n    :\n}\n  exit 0\n} # end\nTRAP cleanup EXIT\nreport_counts' \
  $'cleanup() {\n  {\n    :\n}\n  exit 0; }\nTRAP cleanup EXIT\nreport_counts' \
  $'cleanup() {\n  {\n    :\n}\n  exit 0\n  }\nTRAP cleanup EXIT\nreport_counts' \
  $'cleanup() {\n  cat LTLT\'E\'\n}\nx() {\nE\n  exit 0\n}\nTRAP cleanup EXIT\nreport_counts' \
  $'cleanup() {\n  msg="\n}\nx() {\n"\n  exit 0\n}\nTRAP cleanup EXIT\nreport_counts' \
  $'cleanup() {\n  msg=\'\n}\nx() {\n\'\n  exit 0\n}\nTRAP cleanup EXIT\nreport_counts' \
  $'true; cleanupPARENS {\n  exit 0\n}\nTRAP cleanup EXIT\nreport_counts' \
  $'true && cleanupPARENS {\n  exit 0\n}\nTRAP cleanup EXIT\nreport_counts' \
  $'if true; then cleanupPARENS {\n  exit 0\n}\nfi\nTRAP cleanup EXIT\nreport_counts' \
  $'true; FUNCTION cleanup {\n  exit 0\n}\nTRAP cleanup EXIT\nreport_counts' \
  $'cleanup ( ) {\n  exit 0\n}\nTRAP cleanup EXIT\nreport_counts' \
  $'clean-up() {\n  exit 0\n}\nTRAP \'clean-up\' EXIT\nreport_counts' \
  $'function clean.up {\n  exit 0\n}\nTRAP \'clean.up\' EXIT\nreport_counts'; do
  dir="$(scratch)"
  make_tree_with "$dir" 1 "$ending" "report_counts"
  assert_stops "a file CI runs directly may not reach its ending through [$(printf '%s' "$ending" | head -n 6 | tr '\n' '/')]" \
    "test-direct.sh" bash "$check" "$dir"
  rm -rf "$dir"
done

# What stays allowed beside them, read whole and no longer than bash reads it: a cleanup that
# returns and closes on `} 2>/dev/null` or on `rm -rf "$work"; }`, one written `name ( ) {`,
# one with a definition-shaped line in a here-document, and an empty array assignment, which
# is not a definition. The word exit after each is a message, outside every body.
# shellcheck disable=SC2016
for ending in $'cleanup() {\n  rm -rf "$work"\n} 2>/dev/null\nTRAP cleanup EXIT\necho "the word exit in a message"\nreport_counts' \
  $'cleanup() {\n  {\n    :\n}\n  rm -rf "$work"; }\nTRAP cleanup EXIT\necho "the word exit in a message"\nreport_counts' \
  $'cleanup ( ) {\n  rm -rf "$work"\n}\nTRAP cleanup EXIT\necho "the word exit in a message"\nreport_counts' \
  $'cleanup() {\n  cat LTLT\'E\'\nx() {\nE\n}\nTRAP cleanup EXIT\necho "the word exit in a message"\nreport_counts' \
  $'files=()\ncleanup() {\n  rm -rf "${files[@]}"\n}\nTRAP cleanup EXIT\necho "the word exit in a message"\nreport_counts'; do
  dir="$(scratch)"
  make_tree_with "$dir" 1 "$ending" "report_counts"
  assert_ok "a file CI runs directly may reach report_counts past [$(printf '%s' "$ending" | head -n 5 | tr '\n' '/')]" \
    bash "$check" "$dir"
  rm -rf "$dir"
done

# The same holds for a file the runner walks, whose one allowed ending is report_counts alone.
dir="$(scratch)"
make_tree_with "$dir" 1 "report_counts" 'report_counts || true'
assert_stops "a file the runner walks may not end on [report_counts || true]" \
  "test-a-check.sh" bash "$check" "$dir"
rm -rf "$dir"

# --- A suite file that does not end on its counts ------------------------------------------------
# The other half of the same rule: `run.sh` reads the counts off the last line, so a file that
# ends on something else reports nothing and its failures are added to no total.
dir="$(scratch)"
make_tree_with "$dir" 1 "report_counts" 'exit 0'
assert_stops "a file the runner walks must end on its counts" \
  "test-a-check.sh" bash "$check" "$dir"
rm -rf "$dir"

# --- A suite file that ends on a bare echo of its counts -----------------------------------------
# run.sh reads the counts and is not fooled, but the same file run on its own exits 0 with any
# number of failures behind it, and a file that is run directly today is walked by the runner
# tomorrow or the other way round. report_counts prints the same line and ends on the verdict,
# so it is the one ending that is right both ways.
dir="$(scratch)"
make_tree_with "$dir" 1 "report_counts" "$counts"
assert_stops "a file the runner walks may not end on a bare echo of its counts" \
  "test-a-check.sh" bash "$check" "$dir"
rm -rf "$dir"

# --- What the check reads, and nothing else ------------------------------------------------------
# By default the check reads the one workflow, the crates and the tests of the public scripts,
# because those are the gates of this workflow. A second workflow, a folder beside those and a
# test folder beside this one are read only when they are named, so a tree that holds them is
# checked by whoever names them, never in passing.
other="$(scratch)"
make_tree_with "$other" 1 "report_counts" "report_counts"
mkdir -p "$other/elsewhere/tests"
{
  echo "jobs:"
  echo "  other:"
  echo "    steps:"
  echo "      - name: Another job"
  echo "        run: elsewhere/tests/test-b.sh"
  echo "        env:"
  echo "          OTHER_$required: \"0\""
} > "$other/.github/workflows/other.yml"
# shellcheck disable=SC2016
printf '#!/usr/bin/env bash\nif [ "${%s:-0}" = "1" ]; then :; fi\n' "ELSEWHERE_$required" \
  > "$other/elsewhere/reads-a-flag.sh"
printf '#!/usr/bin/env bash\n%s\n' "$counts" > "$other/elsewhere/tests/test-b.sh"
assert_ok "a fault outside ci.yml, crates/ and scripts/tests/ is not read by default" \
  bash "$check" "$other"
assert_stops "a workflow named with --workflow is read instead of ci.yml" "OTHER_$required" \
  bash "$check" "$other" --workflow .github/workflows/other.yml
assert_stops "a path named with --reads is searched for the flags its tests wait for" \
  "ELSEWHERE_$required" bash "$check" "$other" --reads elsewhere
assert_stops "a folder named with --tests is held to the endings" "test-b.sh" \
  bash "$check" "$other" --tests elsewhere/tests
rm -rf "$other"

# The crates are read by default: a contract test that waits for a flag no step of ci.yml sets
# skips in CI as it skips on a laptop.
dir="$(scratch)"
make_tree_with "$dir" 1 "report_counts" "report_counts"
mkdir -p "$dir/crates/a/src"
printf '// reads %s\n' "CRATE_$required" > "$dir/crates/a/src/lib.rs"
assert_stops "a flag read in crates/ and set by no step of ci.yml fails the run" \
  "CRATE_$required" bash "$check" "$dir"
rm -rf "$dir"

# What a build leaves behind is not a test: a `target/` folder anywhere under a path the check
# reads, and a binary file, carry every string the sources and their dependencies hold,
# flag-shaped ones included, and a check that read them would stop on names no test waits for.
# A flag a source file reads beside them is still found.
dir="$(scratch)"
make_tree_with "$dir" 1 "report_counts" "report_counts"
mkdir -p "$dir/crates/a/src" "$dir/crates/a/target/debug" "$dir/crates/b/src"
printf '// a build product %s\n' "BUILT_$required" > "$dir/crates/a/target/debug/notes.rs"
printf 'lib\000%s\000' "BINARY_$required" > "$dir/crates/b/src/blob.rlib"
assert_ok "a flag-shaped string under a target/ folder or in a binary file is not a flag" \
  bash "$check" "$dir"
printf '// reads %s\n' "SOURCE_$required" > "$dir/crates/a/src/lib.rs"
assert_stops "and a flag a source file reads beside them is still found" \
  "SOURCE_$required" bash "$check" "$dir"
rm -rf "$dir"

# --- What the check read never forms a workflow command ------------------------------------------
# The runner takes a line of the job's output that starts with `::` as a workflow command, so
# every `::` in a value, a name or a line the check read and names is taken out, and the stop
# names it without. Each tree is made here and removed by its name.
command="::set-output name=x::y"
dir="$(scratch)"
make_tree_with "$dir" "$command" "report_counts" "report_counts"
assert_stops_without_commands "a flag set like a workflow command is named without its double colons" \
  "set-outputname=xy" bash "$check" "$dir"
rm -rf "$dir"
dir="$(scratch)"
make_tree_with "$dir" 1 "report_counts" "report_counts"
printf '#!/usr/bin/env bash\necho %s\n' "$command" > "$dir/scripts/tests/test-$command.sh"
assert_stops_without_commands "a test file named like a workflow command is named without its double colons" \
  "test-set-output name=xy.sh" bash "$check" "$dir"
assert_stops_without_commands "a last line written like a workflow command is named without its double colons" \
  "[echo set-output name=xy]" bash "$check" "$dir"
rm -rf "$dir"
assert_stops_without_commands "an argument written like a workflow command is named without its double colons" \
  "set-output name=xy" bash "$check" "$REPO_DIR" "$command"
# A tool the check runs over the tree names a path in its own messages, and a path can hold a line
# feed: a directory whose name ends in .sh, read as a test file, makes grep, awk and sed say so on
# the check's stderr.
dir="$(scratch)"
make_tree_with "$dir" 1 "report_counts" "report_counts"
mkdir "$dir/scripts/tests/x"$'\n'"::warning title=t::injected.sh"
assert_stops_without_commands "a tool's message on a path written like a workflow command is named without its double colons" \
  "warning title=tinjected.sh" bash "$check" "$dir"
rm -rf "$dir"

# The runner reads a `##[` anywhere in a line as a workflow command in its older form, so every
# `##[` is taken out as well: at the start of an argument, in the middle of a name and of a line,
# and in a name that holds a carriage return. (A flag's value is read up to its first `#`, so no
# `##[` reaches the check that way.)
assert_stops_without_commands "an argument with a command marker is named without it" \
  "unknown argument: error]x" bash "$check" "$REPO_DIR" "##[error]x"
dir="$(scratch)"
make_tree_with "$dir" 1 "report_counts" "report_counts"
printf '#!/usr/bin/env bash\necho %s\n' "##[error]x" > "$dir/scripts/tests/test-foo ##[stop-commands]bar.sh"
assert_stops_without_commands "a test file named with a command marker in its middle is named without it" \
  "test-foo stop-commands]bar.sh" bash "$check" "$dir"
assert_stops_without_commands "a last line with a command marker is named without it" \
  "[echo error]x]" bash "$check" "$dir"
rm -rf "$dir"
dir="$(scratch)"
make_tree_with "$dir" 1 "report_counts" "report_counts"
printf '#!/usr/bin/env bash\necho x\n' > "$dir/scripts/tests/test-##["$'\r'"x.sh"
assert_stops_without_commands "a test file named with a command marker before a carriage return is named without it" \
  "x.sh" bash "$check" "$dir"
rm -rf "$dir"

# --- This repository ------------------------------------------------------------------------------
# The assertion the cases above exist for.
assert_ok "this repository's own CI cannot pass by skipping" bash "$check" "$REPO_DIR"

report_counts
