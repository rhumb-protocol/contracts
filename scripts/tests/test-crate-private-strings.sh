#!/usr/bin/env bash
# The tracked tree names nothing of the operational side and nothing a public reader cannot
# follow: not the folder that holds a deployment's tables, not a deploy script or another script
# of the private repositories, not the body that amends the specs, not a private repository by
# name, and the host program that posts the issuer adapters' rates only where it is the issuer
# adapter's role of that name: in adapter-issuer, in pool-integration, whose tests drive that
# role, and in KNOWN-ISSUES.md, which lists what that role does not do. spec/ is read as it comes
# from governance and is not searched here. The strings are built from halves, so that this
# public file does not carry them, and each half-built string is shown to match the name it
# stands for before the tree is searched with it.
set -uo pipefail

# shellcheck source=scripts/tests/helpers.sh
. "$(cd "$(dirname "$0")" && pwd)/helpers.sh"

CRATES="$REPO_DIR/crates"

# The tracked files, text only, spec/ left out; grep exits 1 when it finds nothing and 2 when
# it could not read, and only 0 and 1 are a verdict.
tree_grep() {
  git -C "$REPO_DIR" grep -I -n -i -E -- "$1" -- . ':(exclude)spec'
}

read_ok() {
  [ "$1" -le 1 ] || fail "grep read every tracked file" "status $1"
}

# A string of the private layer, built from halves: the pattern must match the sample, so that
# a check that finds nothing is known to have searched for the right thing, and the tree must
# carry no match.
never_named() {
  local what="$1" pattern="$2" sample="$3" hits
  if LC_ALL=C grep -qiE -- "$pattern" <<< "$sample"; then
    pass "the pattern for $what matches its sample"
  else
    fail "the pattern for $what matches its sample" "pattern $pattern, sample $sample"
  fi
  hits="$(tree_grep "$pattern")"
  read_ok "$?"
  if [ -z "$hits" ]; then
    pass "the tree never names $what"
  else
    fail "the tree never names $what" "$hits"
  fi
}

program="kee""per"

# The role is still named where it is kept, so that the greps below are known to read the files.
if LC_ALL=C grep -rqF "set_$program" "$CRATES/adapter-issuer/src"; then
  pass "crates/adapter-issuer still names its role"
else
  fail "crates/adapter-issuer still names its role" "no set_$program under $CRATES/adapter-issuer/src"
fi

never_named "the folder of a deployment's tables" 'chain''-config' 'the chain''-config folder'
never_named "a deploy script" 'deploy-[a-z0-9-]+\.sh' 'deploy-''adapter-x.sh'
never_named "the artefact script" 'fetch-''artefacts\.sh' 'fetch-''artefacts.sh'
never_named "the local rollout script" 'rollout-''local\.sh' 'rollout-''local.sh'
never_named "the install test of the service" 'test-install-''service\.sh' 'test-install-''service.sh'
never_named "the body that amends the specs" '(^|[^a-z])the ''brain([^a-z]|$)' "the ""brain's"
never_named "the engine's repository by name" 'wadrock/''core' 'in wadrock/''core'

hits="$(git -C "$REPO_DIR" grep -I -n -i -F -- "$program" -- . ':(exclude)spec' \
  ':(exclude)crates/adapter-issuer' ':(exclude)crates/pool-integration' ':(exclude)KNOWN-ISSUES.md')"
read_ok "$?"
if [ -z "$hits" ]; then
  pass "the host program is named only in adapter-issuer, pool-integration and KNOWN-ISSUES.md"
else
  fail "the host program is named only in adapter-issuer, pool-integration and KNOWN-ISSUES.md" "$hits"
fi

report_counts
