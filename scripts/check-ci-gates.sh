#!/usr/bin/env bash
# A green CI run may never mean that nothing ran: a required test that skips is a failure, not a
# pass, and a test script ends on the command whose exit status is its verdict.
#
# Two things are held here, both of which have gone wrong in this repository before.
#
# The `*_REQUIRED` flags. A test that skips when what it needs is missing is right on a laptop
# and wrong in CI, where what it needs is always there: the Wasm is built, the chain is up, the
# artefacts are downloaded. The flags turn those skips into failures, so a flag at "0" hands the
# skip back, and a flag that no step sets at all never raised anything in the first place. Both
# are a stop here. There is no "the condition was not met" case to judge: CI is the place where
# the conditions are met, which is why the flags exist.
#
# The last command of a test script. A file that ends on a bare `echo` exits 0 with any number of
# failures behind it; that is how the rollout was green while ten of its checks failed. A file CI
# runs directly ends on one of a short list of endings whose exit status is the verdict, and
# anything not on the list is a stop, `report_counts || true` included; a file the runner walks
# ends on `report_counts`, which prints the counts the runner reads its verdict from and exits
# on them, so the same file run on its own is not green on failures either.
#
# What it reads: by default the one workflow `.github/workflows/ci.yml`, the flags the tests in
# `crates/` and `scripts/tests/` wait for, and the endings of the test scripts in `scripts/tests/`,
# and nothing else. Another workflow, another place a flag can be read in and another test folder
# are read only when they are named, each path relative to the root: --workflow replaces the
# workflow, --reads (repeatable) replaces the places, --tests replaces the test folder.
#
# Usage: scripts/check-ci-gates.sh [repository root] [--workflow <file>] [--reads <path>]...
#                                  [--tests <dir>]   (the tests pass a tree of their own)
set -uo pipefail

# What the check read and prints never forms a workflow command: the runner takes a line of the
# job's output that starts with `::`, after its leading spaces, as one, and a stop names values,
# file names and lines of the files it read, while grep, awk and sed name the paths they fail on
# in messages of their own. So the whole of the check's output and of every command it runs goes
# through this filter, its own words with it, so that no `::` forms where the two meet; taking out
# each pair leaves at most one colon of every run, so no `::` is left. Bytes, not characters, so
# that a line that is no valid text in the locale is still printed. Whoever reads the output reads
# it to its end, which comes when the filters have printed everything and closed it.
unmarked() {
  LC_ALL=C sed 's/:://g'
}
exec > >(unmarked) 2> >(unmarked >&2)

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
if [ "$#" -gt 0 ] && [ "${1#--}" = "$1" ]; then
  ROOT="$1"
  shift
fi
WORKFLOW=".github/workflows/ci.yml"
READS=()
TESTS="scripts/tests"
while [ "$#" -gt 0 ]; do
  case "$1" in
    --workflow|--reads|--tests)
      if [ "$#" -lt 2 ] || [ -z "$2" ]; then
        echo "$1 needs a path relative to the repository root" >&2
        exit 2
      fi
      case "$1" in
        --workflow) WORKFLOW="$2" ;;
        --reads) READS+=("$2") ;;
        --tests) TESTS="$2" ;;
      esac
      shift 2
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done
if [ "${#READS[@]}" -eq 0 ]; then
  READS=("crates" "scripts/tests")
fi
status=0

stop() {
  echo "$1" >&2
  status=1
}

# --- 1. Every flag a workflow sets is set to "1" ------------------------------------------------
#
# The value is read as the workflow writes it: `NAME: "1"`, quoted or not.
flags_set=""
if [ -f "$ROOT/$WORKFLOW" ]; then
  while IFS= read -r line; do
    [ -n "$line" ] || continue
    name="${line%%:*}"
    value="${line#*:}"
    value="$(printf '%s' "$value" | tr -d ' "'"'")"
    case "$value" in
      1) flags_set="$flags_set $name" ;;
      *) stop "$name is set to \"$value\" in a workflow; a required test that skips is a failure, not a pass" ;;
    esac
  done <<< "$(grep -hoE '[A-Z][A-Z0-9_]*_REQUIRED:[^#]*' "$ROOT/$WORKFLOW" 2>/dev/null)"
fi

# --- 2. Every flag a test reads is raised by some step -------------------------------------------
#
# The tree is read rather than a list kept here: a list would be a second place to remember, and
# the flag that is forgotten is exactly the one that would be missing from it.
while IFS= read -r name; do
  [ -n "$name" ] || continue
  case " $flags_set " in
    *" $name "*) ;;
    *) stop "$name is read by a test and no CI step sets it to \"1\"; the test skips in CI as it skips on a laptop, and a required test that skips is a failure" ;;
  esac
# What a build leaves behind is not read: a `target/` folder carries every string the sources
# and their dependencies hold, and a binary file is not a test, so `-I` skips binaries and
# `--exclude-dir=target` skips build output under any path read.
done <<< "$(grep -rhoEI --exclude-dir=target '[A-Z][A-Z0-9_]*_REQUIRED' \
             "${READS[@]/#/$ROOT/}" \
             2>/dev/null | sort -u)"

# --- 3. The last command of a test script is the verdict ------------------------------------------
#
# Which files CI runs directly is read out of the workflow, so that a file moved into or out of
# the suite is judged by the way it is actually run.
directly=""
if [ -f "$ROOT/$WORKFLOW" ]; then
  directly="$(grep -hoE '[A-Za-z0-9._/-]*\.sh' "$ROOT/$WORKFLOW" 2>/dev/null | sort -u)"
fi

last_command_of() {
  grep -vE '^[[:space:]]*(#|$)' "$1" | tail -n 1 | sed 's/^[[:space:]]*//'
}

# Why the last line of a test script is not the command the file ends on, or nothing when it is.
# The endings below are judged on the last non-comment line, and that line is the file's last
# command only when the file reaches it as one. Each check here is a way it does not, and each
# was run and ends a file on 0 with a failure behind it:
#   - anything `bash -n` says about the file, which is where an unterminated here-document shows;
#     a test script CI runs is clean under `bash -n`, so any warning at all is a stop;
#   - a trap the gate cannot read. The gate reads one shape, the one the suite uses: `trap` first
#     on its line, then one single-quoted action on that line or one bare word, then signal names
#     in capitals or 0, separated by spaces, and nothing after. Any other line that carries the
#     word `trap` -- after another command, behind `builtin`, `command`, `eval` or a backslash,
#     a quoted signal, a `;` behind it, an action over several lines -- is a stop, as an ending
#     the gate does not know is;
#   - a readable trap on a signal that fires without being sent (EXIT, 0, ERR, DEBUG, RETURN)
#     whose action reaches `exit` or `exec`, directly or through the functions of the file and
#     of helpers.sh it names, followed to every function they name in turn, in every definition
#     of each. A trap that only cleans up keeps the status of the command before it, which is
#     why the trap itself is allowed. helpers.sh is read for its traps too, because every test
#     file sources it;
#   - a function definition the gate cannot read (see functions_of), for the same reason as a
#     trap it cannot read;
#   - a here-document whose delimiter, bare or quoted with `'`, `"` or `\`, is the file's last
#     line: the last line is then the end of text and the last command whatever read it;
#   - a line before the last that ends on `\`, `||`, `&&` or `|`, read with and without what
#     looks like a comment behind it: the last line continues it, so the last command is
#     `true || report_counts` (never run) or an echo that prints it.
# Out of reach of any reading of lines, and not accidents, which rule 11 is about: an action,
# a signal or a function name held in a variable, a constant in the variable an allowed
# `exit "$var"` names, quoting inside the word `trap` (`tra\p`), and a redefinition of `exit`,
# `trap` or `report_counts`. Also out of reach: an `exit` at the top of the file before its
# last line, because a line reading cannot tell it from the `exit 0` at the start of a line
# inside a multi-line string, which is how the suite writes its stubs. Some shapes stop that
# are harmless -- a comment or a message in a cleanup function that mentions exit, the word
# `trap` in a message, two spaces before a signal, `trap - EXIT` -- because the gate errs on
# the side rule 11 wants, and no file of the suite writes them.

# The functions defined in a file, each as a line `@@fn <name>` followed by its lines, each
# prefixed with `|` so that no line of a body can pass for a record of its own. A definition is
# `name() {` or `function name {` (with or without the parentheses, with or without spaces
# between them) at the start of a line, at any indent, nested ones included, with a name in
# `[A-Za-z_][A-Za-z0-9_]*`.
#
# Where its body ends is bash's to say, not a reading of lines: a `}` that closes an inner
# group, a here-document or a multi-line string holding a `}` or a definition-shaped line, and
# a closing line such as `} 2>/dev/null`, `} # end`, `exit 0; }` or an indented `}` each defeat
# any rule about which `}` is the last. So a reading of lines only proposes an end -- the line
# itself when it closes there (a `}` at the end of the line, as a word of its own, once the
# `${...}` expansions and a comment are taken off it), else the last line that is a `}` at the
# definition's own indent before the next definition at that indent or a shallower one -- and
# bash decides (body_end_of): the lines from the definition to the proposed end are handed to
# `bash -n`, and the end stands only when they parse whole, without a word on its output. When
# they parse whole, every construct they open is closed by then, the function among them, so
# the body bash runs ends at or before that line: the body read is never shorter than bash's.
# When they do not, or no end was proposed, the end is the first line at which the lines from
# the definition parse whole, which is bash's own end of the definition, and the end of the file
# when no line does. Nothing is ever executed: `bash -n` only parses.
#
# A line that is a definition in any other shape is written `@@bad <line>`: like a trap, a
# function the gate cannot read is a stop, and not a function it pretends is not there. That is
# the `{` on the next line, a body in parentheses, a definition behind `;`, `&`, `|`, `(`, `{`,
# `then`, `do` or `else` on its line (`true; f() {`, `if :; then f() {`), and a name outside the
# class above (`clean-up() {`, `function clean.up {`), which bash accepts and which a trap can
# name in quotes. Some harmless lines stop for it too -- such a shape inside a string or a
# here-document -- because the gate errs on the side rule 11 wants, and no file of the suite
# writes them.
functions_of() {
  local file="$1" record name first proposed total
  total="$(awk 'END { print NR }' "$1")"
  while IFS= read -r record; do
    case "$record" in
      "@@bad "*) printf '%s\n' "$record" ;;
      "@@span "*)
        read -r _ name first proposed <<< "$record"
        printf '@@fn %s\n' "$name"
        sed -n "${first},$(body_end_of "$file" "$first" "$proposed" "$total")p" "$file" | sed 's/^/|/'
        ;;
    esac
  done <<< "$(awk '
    function definition(l) {
      if (l ~ /^[ \t]*#/) return 0
      return match(l, /^[ \t]*function[ \t]+[A-Za-z_][A-Za-z0-9_]*([ \t]*\([ \t]*\))?/) != 0 ||
             match(l, /^[ \t]*[A-Za-z_][A-Za-z0-9_]*[ \t]*\([ \t]*\)/) != 0
    }
    # A definition the shape above does not take: behind an operator, a group or a keyword on
    # its line, or with a name of other characters. The name here is any word without a
    # character that ends one or that an assignment or an expansion would start with, read once
    # the `${...}` expansions are taken off the line, so that `files=()`, `$( )` and
    # `"^${name}()"` are not definitions.
    function hidden(l) {
      if (l ~ /^[ \t]*#/) return 0
      gsub(/\$\{[^}]*\}/, "", l)
      return l ~ /(^|[;&|({]|(^|[ \t])(then|do|else))[ \t]*function[ \t]/ ||
             l ~ /(^|[;&|({]|(^|[ \t])(then|do|else))[ \t]*[^ \t;&|()<>=$"'\''`\\]+[ \t]*\([ \t]*\)/
    }
    { line[NR] = $0 }
    END {
      for (i = 1; i <= NR; i++) {
        l = line[i]
        if (!definition(l)) {
          if (hidden(l)) print "@@bad " l
          continue
        }
        head = substr(l, 1, RLENGTH); rest = substr(l, RLENGTH + 1)
        indent = l; sub(/[^ \t].*$/, "", indent)
        name = head; sub(/^[ \t]*(function[ \t]+)?/, "", name); sub(/[ \t(].*$/, "", name)
        if (rest !~ /^[ \t]*\{/ || hidden(rest)) { print "@@bad " l; continue }
        after = rest; sub(/^[ \t]*\{/, "", after)
        gsub(/\$\{[^}]*\}/, "", after)
        if (after ~ /^[ \t]*#/) after = ""
        sub(/[ \t]+#.*$/, "", after)
        if (after ~ /^\}[ \t;]*$/ || after ~ /[ \t;]\}[ \t;]*$/) { print "@@span " name " " i " " i; continue }
        end = 0
        for (j = i + 1; j <= NR; j++) {
          if (definition(line[j])) {
            other = line[j]; sub(/[^ \t].*$/, "", other)
            if (length(other) <= length(indent)) break
          }
        }
        for (k = j - 1; k > i; k--) {
          if (line[k] ~ ("^" indent "}[ \t;]*$")) { end = k; break }
        }
        print "@@span " name " " i " " end
      }
    }' "$file")"
}

# Whether lines $2 to $3 of file $1 parse whole: `bash -n` accepts them and says nothing, not
# even the warning of a here-document that runs to their end.
parses_whole() {
  local said
  said="$(sed -n "$2,$3p" "$1" | bash -n 2>&1)" && [ -z "$said" ]
}

# The last line of the definition that starts on line $2 of file $1 (see functions_of): the
# proposed end $3 when the lines up to it parse whole, else the first line at which they do,
# else the last line of the file, $4. A proposed end of 0 is none.
body_end_of() {
  local file="$1" first="$2" proposed="$3" total="$4" k
  if [ "$proposed" -gt 0 ] && parses_whole "$file" "$first" "$proposed"; then
    echo "$proposed"
    return 0
  fi
  for ((k = first; k <= total; k++)); do
    if parses_whole "$file" "$first" "$k"; then
      echo "$k"
      return 0
    fi
  done
  echo "$total"
}

# The trap half of the checks below, for a test file and for helpers.sh, which every test file
# sources and whose traps therefore apply to all of them: why a trap or a function definition
# of the file hands the file's status to something after its last command, or nothing. A test
# file is read together with helpers.sh, its traps as well as its functions: a trap there that
# names a function only the test file defines runs the test file's function when it fires.
trap_reason_of() {
  local file="$1" code functions bad line action signals text seen word queue found
  code="$(grep -vE '^[[:space:]]*#' "$file" | sed -E 's/[[:space:]]+#.*$//')"
  functions="$(functions_of "$file")"
  if [ "$file" != "$tests_dir/helpers.sh" ] && [ -f "$tests_dir/helpers.sh" ]; then
    code="$code"$'\n'"$(grep -vE '^[[:space:]]*#' "$tests_dir/helpers.sh" | sed -E 's/[[:space:]]+#.*$//')"
    functions="$functions"$'\n'"$(functions_of "$tests_dir/helpers.sh")"
  fi
  bad="$(grep -m 1 '^@@bad ' <<< "$functions" || true)"
  if [ -n "$bad" ]; then
    bad="${bad#@@bad }"
    printf 'a function definition the gate cannot read, [%s]' "${bad#"${bad%%[![:space:]]*}"}"
    return 0
  fi
  while IFS= read -r line; do
    [ -n "$line" ] || continue
    if ! grep -qE "^[[:space:]]*trap ('[^']*'|[A-Za-z_][A-Za-z0-9_]*)( ([A-Z]+|0))+[[:space:]]*\$" <<< "$line"; then
      printf 'a trap the gate cannot read, [%s]' "${line#"${line%%[![:space:]]*}"}"
      return 0
    fi
    signals=" $(sed -E "s/^[[:space:]]*trap ('[^']*'|[A-Za-z_][A-Za-z0-9_]*)//" <<< "$line") "
    case "$signals" in
      *" EXIT "*|*" 0 "*|*" ERR "*|*" DEBUG "*|*" RETURN "*) ;;
      *) continue ;;
    esac
    action="$(sed -E "s/^[[:space:]]*trap ('([^']*)'|([A-Za-z_][A-Za-z0-9_]*)).*/\2\3/" <<< "$line")"
    # The action and every function it reaches, followed name by name until nothing new; a name
    # defined more than once is read in every definition, of which bash keeps the last.
    text="$action"
    seen=" "
    queue="$(grep -oE '[A-Za-z_][A-Za-z0-9_]*' <<< "$action" || true)"
    while [ -n "$queue" ]; do
      word="$(head -n 1 <<< "$queue")"
      queue="$(tail -n +2 <<< "$queue")"
      case "$seen" in *" $word "*) continue ;; esac
      seen="$seen$word "
      found="$(awk -v fn="@@fn $word" '/^@@/ { on = ($0 == fn); next } on { print substr($0, 2) }' <<< "$functions")"
      [ -n "$found" ] || continue
      found="$(grep -vE '^[[:space:]]*#' <<< "$found" | sed -E 's/[[:space:]]+#.*$//')"
      text="$text"$'\n'"$found"
      queue="$queue"$'\n'"$(grep -oE '[A-Za-z_][A-Za-z0-9_]*' <<< "$found" || true)"
      queue="$(sed '/^$/d' <<< "$queue")"
    done
    if grep -qwE 'exit|exec' <<< "$text"; then
      printf 'a trap on%s runs [%s], which reaches exit or exec after the last command' \
        "${signals% }" "$action"
      return 0
    fi
  done <<< "$(grep -wE 'trap' <<< "$code" || true)"
  return 0
}

unreached_ending_of() {
  local file="$1" said code reason last delimiter raw previous
  said="$(bash -n "$file" 2>&1)" || true
  if [ -n "$said" ]; then
    printf 'bash -n says: %s' "$(head -n 1 <<< "$said")"
    return 0
  fi
  reason="$(trap_reason_of "$file")"
  if [ -n "$reason" ]; then
    printf '%s' "$reason"
    return 0
  fi
  # The file without its comment lines and without a comment behind a command. A `#` inside a
  # string after a space is cut too, so the continuation check below reads the line both ways.
  raw="$(grep -vE '^[[:space:]]*(#|$)' "$file")"
  code="$(sed -E 's/[[:space:]]+#.*$//' <<< "$raw" | grep -vE '^[[:space:]]*$')"
  last="$(tail -n 1 <<< "$code" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
  while IFS= read -r delimiter; do
    [ -n "$delimiter" ] || continue
    if [ "$delimiter" = "$last" ]; then
      printf 'a here-document ends on the last line [%s], so the last command is the one that read it' "$last"
      return 0
    fi
  done <<< "$(grep -oE "<<-?[[:space:]]*['\"\\\\]?[A-Za-z_][A-Za-z0-9_]*" <<< "$code" \
                | sed -E "s/^<<-?[[:space:]]*['\"\\\\]?//")"
  if [ "$(wc -l <<< "$raw")" -ge 2 ]; then
    for previous in "$(tail -n 2 <<< "$raw" | sed -n '1p')" "$(tail -n 2 <<< "$code" | sed -n '1p')"; do
      previous="${previous%"${previous##*[![:space:]]}"}"
      if grep -qE '(\\|\|\||&&|\|)$' <<< "$previous"; then
        printf 'the line before the last, [%s], continues onto it' "$previous"
        return 0
      fi
    done
  fi
  return 0
}

# One alternative per allowed ending, matched against the whole last command: `report_counts`,
# `exit "$var"` with or without the quotes, and `[ "$var" -eq 0 ]` with or without them. The
# dollar signs are the regex's literal ones, which is why the pattern is in single quotes.
# shellcheck disable=SC2016
allowed_direct_endings='report_counts|exit \$[A-Za-z_][A-Za-z0-9_]*|exit "\$[A-Za-z_][A-Za-z0-9_]*"|\[ \$[A-Za-z_][A-Za-z0-9_]* -eq 0 \]|\[ "\$[A-Za-z_][A-Za-z0-9_]*" -eq 0 \]'

tests_dir="$ROOT/$TESTS"
if [ -d "$tests_dir" ]; then
  for file in "$tests_dir"/*.sh; do
    [ -e "$file" ] || continue
    name="$(basename "$file")"
    # helpers.sh is sourced, never run; it has no verdict to end on, but its traps and the
    # functions they name apply to every file that sources it.
    if [ "$name" = "helpers.sh" ]; then
      unreached="$(trap_reason_of "$file")"
      [ -z "$unreached" ] || stop "$name is sourced by every test file and $unreached; the last command of a test script is the one whose exit status is the verdict"
      continue
    fi
    last="$(last_command_of "$file")"
    unreached="$(unreached_ending_of "$file")"
    if [ -n "$unreached" ]; then
      stop "$name does not reach its last line [$last] as its last command: $unreached; the last command of a test script is the one whose exit status is the verdict"
      continue
    fi
    run_directly=0
    case "$directly" in
      *"$TESTS/$name"*) run_directly=1 ;;
    esac
    if [ "$run_directly" -eq 1 ]; then
      # The endings whose exit status is the verdict, and nothing else. A list of the commands
      # that always exit 0 misses every spelling it does not name -- `report_counts || true`
      # ends on the right function and throws its status away -- so the check names what is
      # allowed and an ending nobody thought of is a stop until it is added here on purpose:
      # report_counts itself, `exit` on a variable, or a test that the failures are zero.
      if ! grep -qxE "$allowed_direct_endings" <<< "$last"; then
        stop "$name is run directly by CI and ends on [$last]; the last command of a test script is the one whose exit status is the verdict, and a file CI runs directly ends on report_counts, exit \"\$var\" or [ \"\$var\" -eq 0 ]"
      fi
    else
      # A file the runner walks: `run.sh` reads the counts off the last line and turns them into
      # the status. A file that ends on anything else reports nothing and fails nothing, and one
      # that ends on a bare echo of its counts is green when it is run on its own.
      case "$last" in
        report_counts) ;;
        *)
          stop "$name is run through run.sh and ends on [$last]; a file the runner walks ends on report_counts, which prints the counts the runner reads and exits on them"
          ;;
      esac
    fi
  done
fi

if [ "$status" -eq 0 ]; then
  echo "the flags are raised and every test script ends on its verdict"
fi
exit "$status"
