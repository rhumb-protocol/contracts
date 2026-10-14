#!/usr/bin/env bash
# Every checkout of every workflow leaves no credential behind.
#
# `actions/checkout` keeps the token it was given in the working tree unless its step says
# `persist-credentials: false` in its own `with:`, and then every later step of the job can read
# it. No step of this repository pushes or fetches, so no step needs it.
#
# A line scan of YAML was found to be beaten by the YAML itself (an escape in a double-quoted
# scalar, the setting as text inside another input's value, a key written in escapes), so the
# workflows are read by a parser: zizmor, MIT, version 1.30.1 of 2026-09-09, by the digest of its
# image, and its audit `artipacked` alone. Its other audits are a matter of their own.
#
# It reads every workflow or it is red: offline, no configuration file, no inline ignore, and a
# workflow it cannot parse is an error (`--strict-collection`), never a workflow it skipped. The
# persona is `auditor`, the one that also reports a checkout whose setting is written `true`. It is
# given the `.yml` files themselves, so that no ignore file in the directory hides one, and none is
# a stop.
#
# zizmor reads a checkout only in its canonical spelling, while GitHub splits a `uses` on `/` and
# `\` and drops the empty pieces, so `actions//checkout@...` is the checkout to the runner and none
# to zizmor (found in a security review). So every `uses`, of a step and of a job, is first
# read as a YAML value by a second parser -- yq, MIT, version 4.53.6 of 2026-08-20, by the digest of
# its image -- and must be canonical: `owner/repo[/path]@<40 hex>` with no empty, `.` or `..` piece
# and no leading slash, or `docker://...`. Anything else is a stop. The same yq reads
# zizmor's findings, so that a stop names the files of a kept checkout and no others.
#
# Usage: scripts/check-checkouts.sh [repository root]   (a test may pass a tree of its own)
set -uo pipefail

ROOT="${1:-$(cd "$(dirname "$0")/.." && pwd)}"
# Pinned by tag as well as by digest: a tag can be pushed again, a digest cannot.
IMAGE="ghcr.io/zizmorcore/zizmor:1.30.1@sha256:a2eb396d886c053073405c7a980f2139ba2248ec172243cfa3841e57196e8101"
YQ="mikefarah/yq:4.53.6@sha256:cfc4eee658595834ef304eadb0c3ea721f3b7cb6404ad8b7cb909cc5b5145b23"

# Every container runs as the user of this script: the temporary directory and a test's tree are
# readable by their owner alone, and an image's own user would be refused them on a Linux runner.
RUNAS="$(id -u):$(id -g)"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# The workflows, by name, relative to the root: what both tools are given.
files=()
for path in "$ROOT"/.github/workflows/*.yml; do
  [ -f "$path" ] && files+=(".github/workflows/$(basename "$path")")
done
if [ "${#files[@]}" -eq 0 ]; then
  echo "no workflow named .yml under $ROOT/.github/workflows; a check of nothing is no check" >&2
  exit 1
fi

# 1. Every uses is canonical, read as a value. yq prints each one that is not, as JSON, with its
# file; it exits non-zero on a file it cannot read, which is a stop as well. The expression is
# written to a file of its own, so that no shell quoting stands between it and yq. A local `./`
# action is no canonical uses: neither tool reads a composite action, and a path of `..` pieces
# reaches the actions the runner downloaded (this repository calls none). And no key of a step, or
# of its `with`, may differ from another key or from its own name in case alone, and every such key
# is plain ASCII letters, digits, `_` and `-`: the runner reads inputs without case and puts each
# into the environment as `INPUT_` and the key upper-cased, a later one winning, so a second
# spelling, a letter that upper-cases into ASCII (the long s) or a NUL that cuts the name would be
# a second value of the same input.
cat > "$work/uses.yq" <<'EXPRESSION'
(
  filename as $f
  | (.jobs[] | (.uses, .steps[]?.uses)) | select(. != null)
  | select(
      (tag != "!!str")
      or (test("^(docker://.+|[A-Za-z0-9_-][A-Za-z0-9_.-]*/[A-Za-z0-9_.-]+(/[A-Za-z0-9_.-]+)*@[0-9a-f]{40})$") | not)
      or ((test("^docker://") | not) and test("(^|/)\.{1,2}(/|@)"))
    )
  | {"file": $f, "uses": .}
),
(
  filename as $f
  | .jobs[].steps[]? | keys[] | select((. != downcase) or (test("^[a-z0-9_-]+$") | not))
  | {"file": $f, "step key in another case or outside plain ascii": .}
),
(
  filename as $f
  | .jobs[].steps[]?.with | select(. != null) | keys
  | select(((map(downcase) | unique | length) != length)
      or (map(select(downcase == "persist-credentials" and . != "persist-credentials")) | length > 0)
      or (map(select(test("^[A-Za-z0-9_-]+$") | not)) | length > 0))
  | {"file": $f, "with keys alike but for case, or outside plain ascii": .}
)
EXPRESSION
if ! docker run --rm --user "$RUNAS" -v "$ROOT:/mnt:ro" -v "$work:/expression:ro" -w /mnt "$YQ" eval-all \
  --from-file /expression/uses.yq -o=json -I=0 "${files[@]}" > "$work/uses.json" 2> "$work/uses.log"; then
  echo "yq could not read the uses of the workflows:" >&2
  cat "$work/uses.log" >&2
  exit 1
fi
if [ -s "$work/uses.json" ]; then
  echo "a uses that is not canonical (owner/repo[/path]@<40 hex> or docker://...), or a key in another case:" >&2
  cat "$work/uses.json" >&2
  exit 1
fi

# 2. Every checkout leaves no credential behind, read by zizmor.
docker run --rm --user "$RUNAS" -v "$ROOT:/mnt:ro" -w /mnt "$IMAGE" \
  --offline --no-config --no-ignores --strict-collection --collect workflows --persona auditor \
  --format json "${files[@]}" > "$work/findings.json" 2> "$work/log.txt"
rc=$?
# zizmor's own exit status: 0 with no finding, 10 to 14 with findings by their highest severity,
# anything else an error. An error is a red check, never a pass: a workflow nobody read is not one
# that keeps no credential.
case "$rc" in
  0 | 1[0-4]) ;;
  *)
    echo "zizmor did not audit the workflows (exit $rc):" >&2
    cat "$work/log.txt" >&2
    exit 1
    ;;
esac
if ! grep -q '^\[' "$work/findings.json"; then
  echo "zizmor answered no list of findings:" >&2
  cat "$work/findings.json" "$work/log.txt" >&2
  exit 1
fi

if ! kept="$(docker run --rm --user "$RUNAS" -i "$YQ" -p=json \
  '[.[] | select(.ident == "artipacked")] | length' < "$work/findings.json")"; then
  echo "yq could not read zizmor's findings" >&2
  exit 1
fi
if [ "$kept" -gt 0 ]; then
  echo "$kept checkout(s) keep their credentials; give each step persist-credentials: false in its with:" >&2
  docker run --rm --user "$RUNAS" -i "$YQ" -p=json \
    '[.[] | select(.ident == "artipacked") | .locations[].symbolic.key.Local.verbatim_path] | unique | .[]' \
    < "$work/findings.json" >&2
else
  echo "every checkout of the workflows leaves no credential behind"
fi
# The last command is the verdict.
[ "$kept" -eq 0 ]
