#!/usr/bin/env bash
# What scripts/check-checkouts.sh must catch: a checkout that keeps the token it was given in the
# working tree, where every later step of its job can read it. A step says
# `persist-credentials: false` in its own `with:`, or the check is red; and the check reads the
# workflows as YAML, so that no spelling of a step -- an escape in a quoted scalar, the setting as
# text inside another value, a key written in escapes -- passes it.
#
# Every case is a tree of its own under the system's temporary directory with its workflows in it;
# the check takes a root, so it can be pointed at a tree that is not this repository. The trees
# are made by scratch(), readable by their owner alone, which is the case the check's containers
# must be able to read as well.
#
# The check runs two pinned images under docker, so this file needs docker; without it every case
# fails, never skips.
set -uo pipefail

# shellcheck source=scripts/tests/helpers.sh
. "$(cd "$(dirname "$0")" && pwd)/helpers.sh"

CHECK="$REPO_DIR/scripts/check-checkouts.sh"
CI="$REPO_DIR/.github/workflows/ci.yml"
SHA="3d3c42e5aac5ba805825da76410c181273ba90b1"

# The check runs, as CI runs it, from a step of the check job.
if grep -qxF '        run: scripts/check-checkouts.sh' "$CI"; then
  pass "a step of ci.yml runs scripts/check-checkouts.sh"
else
  fail "a step of ci.yml runs scripts/check-checkouts.sh"
fi
check_job="$(awk '
  $0 == "  check:" { inside = 1; next }
  inside && /^  [a-z]/ { inside = 0 }
  inside { print }
' "$CI")"
if grep -qxF '        run: scripts/check-checkouts.sh' <<< "$check_job"; then
  pass "the step is in the check job"
else
  fail "the step is in the check job"
fi
for pin in \
  'IMAGE="ghcr.io/zizmorcore/zizmor:1.30.1@sha256:a2eb396d886c053073405c7a980f2139ba2248ec172243cfa3841e57196e8101"' \
  'YQ="mikefarah/yq:4.53.6@sha256:cfc4eee658595834ef304eadb0c3ea721f3b7cb6404ad8b7cb909cc5b5145b23"'; do
  if [ -f "$CHECK" ] && grep -qxF -- "$pin" "$CHECK"; then
    pass "the check pins [$pin]"
  else
    fail "the check pins [$pin]"
  fi
done

# Runs the check on tree $3 and holds its status to $2 (0, or anything else for a stop); a stop
# names $4 when it is given and does not name $5 when it is given.
judge() {
  local what="$1" expected="$2" dir="$3" named="${4:-}" absent="${5:-}"
  local output rc
  output="$("$CHECK" "$dir" 2>&1)"
  rc=$?
  rm -rf "$dir"
  if [ "$expected" -eq 0 ] && [ "$rc" -ne 0 ]; then
    fail "$what" "exit $rc, expected 0" "$output"
  elif [ "$expected" -ne 0 ] && [ "$rc" -eq 0 ]; then
    fail "$what" "exit 0, expected a stop" "$output"
  elif [ -n "$named" ] && ! grep -qF -- "$named" <<< "$output"; then
    fail "$what" "the output does not name $named" "$output"
  elif [ -n "$absent" ] && grep -qF -- "$absent" <<< "$output"; then
    fail "$what" "the output names $absent" "$output"
  else
    pass "$what"
  fi
}

# A workflow named $1 with one job of the same name, whose lines below the job's key are $2.
workflow() {
  printf '%s\n' "name: $1" "on: push" "permissions: {}" "jobs:" "  $1:" "$2"
}

# A tree with one workflow, probe.yml, whose one job has the steps given; a stop names the file.
case_of() {
  local what="$1" expected="$2" steps="$3" dir named=""
  dir="$(scratch)"
  mkdir -p "$dir/.github/workflows"
  workflow probe "$(printf '%s\n' "    runs-on: ubuntu-24.04" "    steps:" "$steps")" \
    > "$dir/.github/workflows/probe.yml"
  [ "$expected" -eq 0 ] || named="probe.yml"
  judge "$what" "$expected" "$dir" "$named"
}

# A tree with the files given as name and content pairs under .github/workflows.
tree_case() {
  local what="$1" expected="$2" named="$3" absent="$4" dir
  shift 4
  dir="$(scratch)"
  mkdir -p "$dir/.github/workflows"
  while [ "$#" -ge 2 ]; do
    printf '%s\n' "$2" > "$dir/.github/workflows/$1"
    shift 2
  done
  judge "$what" "$expected" "$dir" "$named" "$absent"
}

WITH_FALSE="$(printf '%s\n' "        with:" "          persist-credentials: false")"

# The shapes of a checkout that keeps its credentials.
case_of "a checkout without the setting is a stop" 1 \
  "      - uses: actions/checkout@$SHA"
case_of "a checkout whose setting is true is a stop" 1 \
  "$(printf '%s\n' "      - uses: actions/checkout@$SHA" "        with:" \
    "          persist-credentials: true")"
case_of "the setting in a comment is no setting" 1 \
  "$(printf '%s\n' "      - uses: actions/checkout@$SHA" "        # persist-credentials: false")"
case_of "the setting in another step is not the checkout's" 1 \
  "$(printf '%s\n' "      - uses: actions/checkout@$SHA" "      - name: elsewhere" \
    "        uses: actions/setup-node@39370e3970a6d050c480ffad4ff0ed4d3fdee5af" "$WITH_FALSE")"
case_of "a checkout written name first without the setting is a stop" 1 \
  "$(printf '%s\n' "      - name: check out" "        uses: actions/checkout@$SHA")"
case_of "the setting under env is not the action's input" 1 \
  "$(printf '%s\n' "      - uses: actions/checkout@$SHA" "        env:" \
    "          persist-credentials: false")"

# The shapes a scan of lines lets through, and a parser of YAML does not.
case_of "a uses written with escapes in a double-quoted scalar is a stop" 1 \
  "      - uses: \"actions/\\x63heckout@$SHA\""
case_of "the setting as text inside another input's value is a stop" 1 \
  "$(printf '%s\n' "      - uses: actions/checkout@$SHA" "        with:" "          ref: |" \
    "            persist-credentials: false")"
case_of "a key written in escapes is a stop" 1 \
  "      - \"u\\x73es\": \"actions/checkout@$SHA\""

# What the check refuses to read past: a workflow it cannot parse, and an inline ignore.
case_of "a workflow the check cannot parse is a stop" 1 \
  "      - uses: [actions/checkout@$SHA"
case_of "an inline ignore of the finding does not silence it" 1 \
  "      - uses: actions/checkout@$SHA   # zizmor: ignore[artipacked]"

# The counter-cases: the setting in the step's own with, in either order of keys.
case_of "a checkout with persist-credentials false passes" 0 \
  "$(printf '%s\n' "      - uses: actions/checkout@$SHA" "$WITH_FALSE")"
case_of "a checkout written name first with the setting among other inputs passes" 0 \
  "$(printf '%s\n' "      - name: check out" "        uses: actions/checkout@$SHA" \
    "        with:" "          fetch-depth: 1" "          persist-credentials: false")"

# Every `uses` is read as a value and must be canonical: GitHub splits a `uses` on `/` and `\` and
# drops the empty pieces, so each spelling below is the checkout to the runner while zizmor 1.30.1
# does not read it as one. Not canonical is a stop of its own.
for spelling in "actions/checkout/@$SHA" "actions//checkout@$SHA" "/actions/checkout@$SHA" \
  "actions/checkout//@$SHA" "actions/checkout/.@$SHA" "Actions/Checkout/@$SHA"; do
  case_of "a uses spelled $spelling is a stop" 1 \
    "$(printf '%s\n' "      - uses: $spelling" "$WITH_FALSE")"
done
case_of "a uses with a backslash is a stop" 1 \
  "$(printf '%s\n' "      - uses: 'actions\\checkout@$SHA'" "$WITH_FALSE")"
case_of "a uses pinned by a tag and not a commit is a stop" 1 \
  "$(printf '%s\n' "      - uses: actions/checkout@v4" "$WITH_FALSE")"
case_of "a uses of another action spelled with an empty piece is a stop" 1 \
  "      - uses: actions//setup-node@$SHA"
# A local action is read by neither tool (a composite action may hold a checkout of its own) and a
# path of `..` pieces can reach the actions the runner downloaded; this repository calls none, so a
# `./` uses is a stop.
case_of "a local action is a stop" 1 \
  "      - uses: ./.github/actions/local"
case_of "a local path that climbs out of the workspace is a stop" 1 \
  "      - uses: ./../../_actions/actions/checkout/$SHA"
case_of "a docker image pinned by digest passes" 0 \
  "      - uses: docker://alpine@sha256:4b7ce07002c69e8f3d704a9c5d6fd3053be500b7f1c69fc0d80990c2ad8dd412"

# A key of a step or of its with that differs from its own name in case only is a stop: the
# runner's inputs compare without case, and a second spelling must not carry a second value.
case_of "a persist-credentials written in another case beside the setting is a stop" 1 \
  "$(printf '%s\n' "      - uses: actions/checkout@$SHA" "$WITH_FALSE" \
    "          Persist-Credentials: true")"
case_of "a uses written in another case is a stop" 1 \
  "$(printf '%s\n' "      - run: true" "        Uses: actions/checkout@$SHA")"

# The runner turns an input into the environment name INPUT_ and the key upper-cased, a later
# entry winning: a key outside plain ASCII (a long s upper-cases to S) or holding a NUL (the name
# is cut there) can become INPUT_PERSIST-CREDENTIALS beside the setting. A key of a step or its
# with is plain ASCII, or it is a stop.
case_of "a key with a long s beside the setting is a stop" 1 \
  "$(printf '%s\n' "      - uses: actions/checkout@$SHA" "$WITH_FALSE" \
    '          "per\u017fi\u017ft-credentials": true')"
case_of "a key that holds a NUL before the setting is a stop" 1 \
  "$(printf '%s\n' "      - uses: actions/checkout@$SHA" "        with:" \
    '          "persist-credentials=true\0": x' "          persist-credentials: false")"

# A reusable workflow called by a job is a `uses` too.
tree_case "a job calling a reusable workflow spelled with an empty piece is a stop" 1 \
  "called.yml" "" \
  called.yml "$(workflow called "    uses: owner//repo/.github/workflows/x.yml@$SHA")"
tree_case "a job calling a reusable workflow by its canonical name passes" 0 "" "" \
  called.yml "$(workflow called "    uses: owner/repo/.github/workflows/x.yml@$SHA")"

# GitHub runs every `.yml` and `.yaml` file in .github/workflows, so the check reads them all,
# those whose name starts with a dot included; a workflow it does not read must never pass.
KEPT="$(printf '%s\n' "    runs-on: ubuntu-24.04" "    steps:" "      - uses: actions/checkout@$SHA")"
OK="$(printf '%s\n' "    runs-on: ubuntu-24.04" "    steps:" "      - run: true")"
tree_case "a kept checkout in a .yaml workflow is a stop" 1 "probe.yaml" "" \
  ok.yml "$(workflow ok "$OK")" probe.yaml "$(workflow probe "$KEPT")"
tree_case "a kept checkout in a workflow whose name starts with a dot is a stop" 1 ".probe.yml" "" \
  ok.yml "$(workflow ok "$OK")" .probe.yml "$(workflow probe "$KEPT")"
tree_case "a .yaml workflow with the setting passes" 0 "" "" \
  ok.yml "$(workflow ok "$OK")" \
  fine.yaml "$(workflow fine "$(printf '%s\n' "    runs-on: ubuntu-24.04" "    steps:" \
    "      - uses: actions/checkout@$SHA" "$WITH_FALSE")")"

# Any other file in .github/workflows is one the check does not read, and a stop that names it:
# an extension in capitals, a file of another kind, an ignore file beside the workflows, and a
# directory.
tree_case "a workflow with an upper-case extension is a stop" 1 "probe.YML" "" \
  ok.yml "$(workflow ok "$OK")" probe.YML "$(workflow probe "$KEPT")"
tree_case "a file of another kind beside the workflows is a stop" 1 "notes.txt" "" \
  ok.yml "$(workflow ok "$OK")" notes.txt "a note"
tree_case "an ignore file beside the workflows is a stop" 1 ".gitignore" "" \
  ok.yml "$(workflow ok "$OK")" .gitignore "probe.yml"
nested="$(scratch)"
mkdir -p "$nested/.github/workflows/more"
workflow ok "$OK" > "$nested/.github/workflows/ok.yml"
workflow probe "$KEPT" > "$nested/.github/workflows/more/probe.yml"
judge "a directory beside the workflows is a stop" 1 "$nested" "more"

# A root given as a relative path is read as the same tree: the stop is the finding on its
# workflow, not a parser that was handed an empty volume of that name.
relative="$(scratch)"
mkdir -p "$relative/.github/workflows"
workflow probe "$KEPT" > "$relative/.github/workflows/probe.yml"
output="$(cd "$(dirname "$relative")" && "$CHECK" "$(basename "$relative")" 2>&1)"
rc=$?
rm -rf "$relative"
if [ "$rc" -ne 0 ] && grep -qF -- "keep their credentials" <<< "$output" \
  && grep -qF -- "probe.yml" <<< "$output"; then
  pass "a relative root is read, and its kept checkout named"
else
  fail "a relative root is read, and its kept checkout named" "exit $rc" "$output"
fi

# The stop names the files whose checkout keeps its credentials, and no file whose findings are
# of another audit.
tree_case "the stop names the file of the kept checkout and not a file of other findings" 1 \
  "probe.yml" "ok.yml" \
  probe.yml "$(workflow probe "$(printf '%s\n' "    runs-on: ubuntu-24.04" "    steps:" \
    "      - uses: actions/checkout@$SHA")")" \
  ok.yml "$(workflow ok "$(printf '%s\n' "    runs-on: ubuntu-24.04" "    steps:" \
    "      - run: true")")"

# A workflow file that is a symbolic link is a stop that names the link, even when it points to a
# workflow inside the tree that sets the setting: the parsers would read whatever the link reaches.
linked="$(scratch)"
mkdir -p "$linked/.github/workflows"
workflow ok "$OK" > "$linked/.github/workflows/ok.yml"
workflow fine "$(printf '%s\n' "    runs-on: ubuntu-24.04" "    steps:" \
  "      - uses: actions/checkout@$SHA" "$WITH_FALSE")" > "$linked/fine.yml"
ln -s ../../fine.yml "$linked/.github/workflows/link.yml"
judge "a workflow file that is a symbolic link is a stop" 1 "$linked" "link.yml"

# A root whose name starts with a dash is a path, not an option.
dashed="$(scratch)"
mkdir -p "$dashed/-tree/.github/workflows"
workflow probe "$KEPT" > "$dashed/-tree/.github/workflows/probe.yml"
output="$(cd "$dashed" && "$CHECK" -tree 2>&1)"
rc=$?
rm -rf "$dashed"
if [ "$rc" -ne 0 ] && grep -qF -- "keep their credentials" <<< "$output" \
  && grep -qF -- "probe.yml" <<< "$output"; then
  pass "a root whose name starts with a dash is read, and its kept checkout named"
else
  fail "a root whose name starts with a dash is read, and its kept checkout named" "exit $rc" "$output"
fi

# A tree without a workflow is a stop: a check of nothing is no check.
empty="$(scratch)"
mkdir -p "$empty/.github/workflows"
judge "a tree without a workflow is a stop" 1 "$empty" "no workflow"

# And this repository's own workflows pass, which is the case that matters on every run.
assert_ok "this repository's own workflows pass the check" "$CHECK" "$REPO_DIR"

report_counts
