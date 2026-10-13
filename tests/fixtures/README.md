# Test fixtures

## The pool Wasm of the engine

Spec A-001 §4 test 4 drives the two adapters through a real band pool: the pool contract of
E-007, registered in the SDK's test environment with `sources 2`, one `adapter-redstone` on a
mock SEP-40 feed and one `adapter-issuer`, running the scenarios of E-003 test 7.

That test needs the pool's Wasm, and the engine is not built here: this repository calls the
engine's contracts and copies nothing from them.

**Only the optimised build artefact of core's CI counts.** It is the Linux build, produced by
core's pipeline and published with its SHA-256, and it is the artefact a deployment would use.
A Wasm built locally -- on this machine, on another architecture, with another toolchain or
without the `wasm-opt` pass -- is **not** a valid fixture and must never be placed here, even
when it passes the tests: a fixture that is not the artefact the pool is deployed from proves
the adapters against something nobody will ever run.

The file is placed here by hand, in a commit of its own, from the artefact `contract-wasm` of a
green run of core's `main`, and only when it is the Wasm of a pool deployed on testnet. Each
file has a row below with its sha256 and the deployment it comes from;
`scripts/tests/test-fixture-hashes.sh` holds the bytes to the row.

## The files

| File | Bytes | SHA-256 | `VERSION` | Core commit | Core CI run | Deployed |
|---|---:|---|---|---|---|---|
| `pool_soroban.wasm` | 106019 | `fa3f548b5be725a66976a7f88ded6d93eb8cab4b30a0e639d3a25d317793f3b1` | 8 | `a499892381d4fd6ec2b5ae878dc2f0f80fc756ca` | `36690431683` | testnet, 2026-09-30 |

`pool_soroban.wasm` comes from core's pull request 27, "feat(pool-soroban): announce rate,
roles and cap raises, VERSION 8"; the run is `main`, push, completed success 2026-09-30, job
`wasm` (109806085301). It was placed here on 2026-09-30.

That pull request is the E-007 amendment that delays governance's changes: the rate, roles and cap
raises are announced and executed after a delay (`announce_roles` with the three roles, the
argument-less `set_roles` a day later, `cancel_roles`; the same for the rate and the cap, and for
the upgrade since `VERSION` 7), `clear_sentinel` takes the sentinel away at once, the constructor
stores the three roles it is given, and `sources()` is a view. The pool's `VERSION` is 8; no pool
is ever deployed on 7. This crate's `abi.rs` states the new shapes of the roles calls, and test 4
drives the announcement and its execution.

The hash in the table was checked against the **job log** of that run, where the optimisation step
reports `pool_soroban.wasm: 136082 bytes before wasm-opt, 106019 bytes after` (the bound is
122880) and the step "The contract Wasms and their sha256 for the artefact (E-007 section 3.6)"
prints the sums it built, and not against the `pool_soroban.wasm.sha256` that travels inside
the artefact. The artefact `contract-wasm` of that run was downloaded with `gh run download`
and the file hashed to the same value before it was placed here. A hash carried by the same
download as the file it describes is a checksum against a truncated transfer, not evidence of
origin: both would be replaced together. The job log is written by the run itself and cannot be
rewritten by whoever hands over the file, so it is the one place a reader can compare against.
Only `pool_soroban.wasm` is copied in; `router_soroban.wasm` is in the same artefact and no test
here uses it.

The fixture before this one was the `VERSION` 6 artefact of run `36307091725` (commit
`5676a7a44ea0f3e3f7b489171bc4063d758f3bfa`, sha256
`6fb578147b98087a0453677a5aedfeaae03b0f4529d9176b5691d2774c7a1f70`, 94256 bytes), which the
three testnet pools deployed on 2026-09-27 run until they are redeployed; before it, the
`VERSION` 5 artefact of run `34780522549` (sha256
`17be44b9312738eabbf7378811baa93c14d6837040e1a79dc7ea28689a8e24f1`, 94023 bytes); and first,
from 2026-09-13 to 2026-09-14, a build of core that was never deployed (sha256
`7f65ff96cc617bc5cc595f4a7024f0d1aca757410b5d4034707efd2f3be1318b`, 94103 bytes).

It is refreshed the same way, by hand and in its own commit, until core is public and CI can
fetch a tagged release. Refreshing means: record the new row in the table above in that
commit, once the deployment of those bytes is on record, and let test 4 run against it; a
fixture whose row no longer matches the bytes is a fixture nobody can check.

## Test 4

Test 4 lives in `crates/pool-integration`. It reads `pool_soroban.wasm` from this directory and
requires it: there is no skip, because a test that passed without the engine's pool would
report green for the one requirement of A-001 this repository cannot meet on its own. The three
Wasm files this workspace builds itself (the two adapters and the mock feed) are read from the
target directory with the usual notice-and-skip, unless `ADAPTER_WASM_REQUIRED` is set, which
CI sets after the Wasm build.
