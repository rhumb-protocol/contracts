# contracts

The rate adapters of Rhumb on Stellar, built by Wadrock Labs: the contracts through which the
band pools of the engine read their prices. The engine itself lives in `wadrock/core` and is not
copied here; these crates call the engine's contracts through their Wasm interfaces.

## Layout
- `crates/adapter-redstone/` — the rate adapter that wraps a SEP-40 price feed and hands a
  pool what it says, converting nothing (A-001 §3.2).
- `crates/adapter-issuer/` — the rate adapter that holds an issuer's published price as the
  account named for it posts it (A-001 §3.3).
- `crates/sep40-mock/` — a test-only SEP-40 feed the reference tests point the adapters at.
  Never deployed to a network whose contracts are kept.
- `crates/pool-integration/` — test-only: the two adapters driven through the engine's real
  pool Wasm (A-001 §4 test 4). Exports no contract.
- `tests/fixtures/` — the engine's pool Wasm for the integration test; see its README for the
  rule on which build counts.
- `spec/` — the specifications these crates are built to, copied unchanged from governance by
  `scripts/sync-specs.sh`.
- `scripts/scan-secrets.sh` — fails when the tracked tree holds something shaped like a
  Stellar secret key, or an `.env` file.
- `scripts/check-ci-gates.sh` — fails when a required test could skip in CI, or a test script
  does not end on its verdict.
- `scripts/tests/` — the tests of the scripts above.
- `.github/workflows/ci.yml` — the checks below on a push to `main`, on every pull request,
  nightly and by hand, and the artefacts `adapter-wasm` and `test-wasm`, each with its sha256
  in the log, on a push to `main`.

## Specifications
The specifications are copies; they are never edited here.
- A-001 — the rate adapters.
- A-007 — the aggregator adapter, when it is built.
- E-003 — the rate provider interface the adapters implement.
- E-007 — the pool contract, whose ABI the crates encode.

## Build and test
```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --release --target wasm32v1-none --locked
```

Every cargo command passes `--locked`, so that what is built and tested is the committed
lockfile. The contract tests run in two modes: against each crate's own Wasm, and natively
inside the SDK's `as_contract`. A missing Wasm makes them skip with a notice, so that a working
copy without a Wasm build is not a wall of failures. Set `ADAPTER_WASM_REQUIRED=1` to turn
every skip into a failure; CI sets it after the Wasm build, so that a green CI can never mean
that the tests found nothing to run.

```
cargo build --workspace --release --target wasm32v1-none --locked
ADAPTER_WASM_REQUIRED=1 cargo test --workspace --locked
```

The scripts have tests of their own, and CI lints them with shellcheck:

```
scripts/tests/run.sh
scripts/check-ci-gates.sh
scripts/scan-secrets.sh
```

## What each part does

### `crates/adapter-redstone`
One deployed instance is one SEP-40 feed as one source of one band pool. It calls `decimals()`
and `lastprice(asset)` and hands the pool exactly what came back: no bound, no conversion, no
storage. A feed that is down makes the call fail, which the pool reads as a source without an
observation. The constructor refuses a feed reporting more than 18 decimals, so an adapter the
pool could never accept does not exist.

### `crates/adapter-issuer`
One deployed instance is one issuer's price, as the account named for it posts it. `post` is
that account's and refuses three things only: a price at or below zero, a timestamp that does
not move forward, and a timestamp later than the ledger's. Everything that is a judgement about
the price belongs to the engine. Governance can replace the posting account without touching
anything else, which is what makes a stolen posting key an incident survived by one transaction.

A `post` writes one ledger entry, the adapter's instance: the posting, the roles and the scale
all live in instance storage, and the extension a post carries below its threshold is of that
same entry, so the signer of a `post` can bound its write set to the adapter contract's
entries. The adapter's tests pin the one entry with the posting account as the transaction's
source account, as on a chain, and show the nonce entry an account that is not the source
account would add.

## Toolchain
Rust 1.98.1, target `wasm32v1-none`, the same pin as the engine; it is bumped here only when
core bumps it, in its own commit. Dependencies are pinned to exact versions.

## History
This repository's history starts at its split from the private operations repository. The first
commit holds the tree of the crates as it stood at the split, and its message names the commit of
the private operations repository that tree was taken from. The earlier history is private; the
public record starts with the first pull request after the split.

## Reporting a vulnerability
Report a vulnerability privately, through GitHub's private vulnerability reporting on this
repository (Security, then "Report a vulnerability"), and never in a public issue;
the full rules follow with the bug bounty.

## Licence
MIT, see LICENSE. Code taken from elsewhere names its origin, licence and commit hash in the
file header.
