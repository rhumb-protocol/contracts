# A-007: The aggregator adapter for Soroswap

Date: 2026-09-23
Status: proposed; amended 2026-10-12 (A-010 step 3, ADR-015 §4): this spec is copied into
the public `contracts` repository, so its text names no folder of the brain; the one
reference to the brain's archive of deploy records (§6) is rewritten as what it says,
nothing in the rules changed
Target: `rhumb-protocol/contracts`: the crate `crates/adapter-soroswap`, its deploy script and
the script that lists a later pool (§3.2), the entry `soroswap-aggregator` in
`chain-config/<network>.toml`. Moved out of A-005 on 2026-09-23 (A-005 §3.7 held this text
until then), so that A-005 could close on the SDK and the quote API; built after A-006.
Depends on: A-000 (the rules of a brand repository), A-002 (the testnet contracts, the form of
a deploy script, `chain-config/<network>.toml`), A-005 §3.6 (the quote API, which names a pool
an off-chain router can pass in `bytes`), §7.9 and §7.10 (the published interfaces of the
router and the pool), E-008 (the router's plan, its bounds and errors), E-006 §5 (no route
through an intermediate asset), I-001 (the exploit reviewer). Blocks: the first item of the
outreach of brief §6, the listing of Rhumb as a venue of the Soroswap aggregator.

## 1. Goal

One Soroban contract in the contracts repository that lets the Soroswap aggregator route a leg
of its own swaps through Rhumb's router. It moves tokens only inside a transaction the trader
signed, through the router of E-008, and holds none. Listing Rhumb in the aggregator is not
code: the aggregator's venue set is a closed enum in Soroswap's contract (§7.1), so a fifth
venue is Soroswap's change, reached by the owner's outreach, never by code of ours.

Aggregator means: a contract that splits one swap over several venues; the Soroswap
aggregator is the one on Stellar that has adapters. Adapter means: the contract the aggregator
calls for one venue, through a fixed trait. Leg means: the share of a swap one venue executes.

## 2. Current state (2026-09-23)

- Testnet holds the router and three pools of A-002's rollout (§7.3), pool `VERSION` 8 and
  router `VERSION` 2 since the redeployment of 2026-09-30 (version 5 at the time of writing,
  6 from 2026-09-27). The SDK and the quote API of A-005 are built and quote over them.
- The router answers `quote_exact_in` and `quote_exact_out` on chain, bounded to
  `MAX_QUOTE_PARTS = 4` parts over `MAX_QUOTE_CANDIDATES = 4` candidates (E-008 §2): the
  worst case uses 37.7 % of the CPU limit and 67.2 % of the memory limit for band candidates.
- The Soroswap aggregator (§7.1) calls one adapter contract per venue through a trait of five
  functions (§7.2); its adapters for Soroswap, Phoenix, Aquarius and Comet are in
  `soroswap/aggregator`. The venue set is the closed enum `Protocol { Soroswap, Phoenix, Aqua,
  Comet }`.
- The contracts repository's security reviewer already names `crates/adapter-*` in its
  exception list (Fable, high), and the exploit reviewer of I-001 runs on the same crates.

## 3. What gets built

### 3.1 The contract `crates/adapter-soroswap`

One Soroban contract that implements the adapter trait of the Soroswap aggregator (§7.2, read
from `soroswap/aggregator` at the commit named there and copied as an interface, never linked)
and executes its swaps through Rhumb's router. It is chain code under every rule of A-000
§3.3 and reviewed as an adapter (§3.3).

- Storage (instance): `Governance`, `Initialized`, `ProtocolId` (a `String`, `"rhumb"`),
  `ProtocolAddress` (the router of E-008), and `Pair(token_a, token_b)` to `Vec<(pool,
  direction_from_a)>`, the pools of that pair in the order governance set them, at most
  `MAX_POOLS_PER_PAIR = 4` (§7.4), with the direction a trade from `token_a` to `token_b`
  takes in each; the reverse direction is derived. The key orders the two tokens by their
  bytes so that one entry serves both directions.
- `__constructor(governance)`: stores governance. `initialize(protocol_id, protocol_address)`
  (the trait's, once; `governance.require_auth()` on the stored governance, so that only the
  deployment script's signer can run it; `AlreadyInitialized` after). `set_pools(caller,
  token_a, token_b, pools: Vec<Address>)` (governance): reads `tokens()` of every pool, refuses
  a pool whose tokens are not these two (`NotPoolOfPair`), a duplicate (`DuplicatePool`) and
  more than four (`TooManyPools`), stores the list with the derived directions, emits `pools`.
  `set_governance` and `upgrade` as E-008 §3.2. `get_protocol_id()`, `get_protocol_address()`
  as the trait.
- `swap_exact_tokens_for_tokens(amount_in, amount_out_min, path, to, deadline, bytes)`: the
  trait's entry point, called by the aggregator with the leg's amount, `amount_out_min = 0`
  (the aggregator checks its own total), the path, the trader as `to` and the deadline. In
  order: `to.require_auth()` (as Soroswap's own adapter does; the router requires it again for
  the same address, one more node of the same signed tree); `deadline` before the ledger's
  timestamp is `DeadlineExpired` (the trait's code 405); a negative amount `NegativeNotAllowed`
  (403); a path of fewer than two tokens `WrongMinimumPathLength` (407), of more than two
  `UnsupportedPath` (420: a route through an intermediate asset is not built, E-006 §5); a pair
  without a list `PairNotListed` (421). The pool: when `bytes` is `Some` with exactly one
  entry, that entry is a pool id and must be in the pair's list, else `PoolNotListed` (422);
  with more entries `WrongPoolHashesLength` (408); when `bytes` is `None`, the adapter asks
  every listed pool `quote_exact_in(direction, amount_in)` by a view sub-call, skips a pool
  whose quote fails, and takes the largest output, ties to the first in the list; no pool
  accepting is `NoRoute` (423). Then `router.swap_exact_in(to, Plan { legs: [Leg { pool,
  direction, amount: amount_in }] }, amount_out_min, to)`, the trader as `from` and `to`, so
  that no token ever sits in the adapter and the trader's signature bounds the leg (E-008 §2);
  the router's or the pool's error propagates unchanged. Emits `swap` (`("swap", to)`: the
  pool, `amount_in`, `amount_out`) and returns `[amount_in, amount_out]`, the amounts along the
  path in the shape the aggregator sums.
- `swap_tokens_for_exact_tokens(amount_out, amount_in_max, path, to, deadline, bytes)`: the
  mirror with `quote_exact_out`, the smallest input winning, and `router.swap_exact_out` with
  `amount_in_max`; returns `[amount_in, amount_out]`.
- Why one pool and not the planner: an aggregator transaction calls several venues in one
  budget, and the router's on-chain quote alone takes up to two thirds of the memory limit
  (§2). The aggregator already splits a swap over venues by `parts`; splitting within Rhumb is
  what the quote API of A-005 §3.6 gives an off-chain router, which can then name the pool in
  `bytes`, one leg per distribution. Four view calls to choose a pool cost about 11 M CPU and
  fit beside other venues; a plan of several legs does not.

### 3.2 Deploy and listing scripts

- `scripts/deploy/deploy-adapter-soroswap.sh <network>` in the form of A-002 §3.6: from the CI
  artefact `adapter_soroswap.wasm` of the contracts repository, hash-checked against
  `[artefacts]`; deploys with the constructor, calls `initialize("rhumb", <router of
  [router]>)`, and `set_pools` once per pair of `[pools]` grouped by tokens, all signed by the
  deployer, governance handed over and read back as A-002 §3.2; writes the entry
  `soroswap-aggregator` under `[adapters]` with the artefact triple, the date and the script's
  commit.
- `set-aggregator-pools.sh <network> <pair>` for a later pool, governance signs.
- On mainnet only after the deploy ADR and never before a pool of the pair exists on testnet
  (A-002 §3.3 point 5). An agent runs neither script; the owner does.

### 3.3 Agents and models

Nothing changes in the contracts repository's agent files: the crate is `crates/adapter-*`, so
the security reviewer's exception (Fable, high) and the exploit reviewer of I-001 already cover
it, and the contracts builder builds it on Opus as it builds every crate.

### 3.4 Registration is not code

The aggregator's `Protocol` enum is closed (§7.1); Rhumb becomes a venue of the aggregator when
Soroswap adds a variant and its admin calls `update_adapters` with this contract's address.
That is the first item of the outreach of brief §6, done by the owner, as the labs; the pull
request to `soroswap/aggregator` that adds the variant is the owner's, in the labs' name, and
never an agent's. Until then the adapter is deployed, tested against a test-only copy of the
aggregator's calling pattern (§4 test 1) and quoted through the SDK.

## 4. Verification (the tests of this spec)

1. The adapter (Soroban test environment with the real pool and router Wasms of the
   artefacts): the trait's five functions exist with the signatures of §7.2; a swap through a
   test-only caller that invokes the adapter exactly as the aggregator does (the trader as
   `to`, `amount_out_min = 0`, the deadline, `bytes` `None`) moves the trader's balances by
   what the chosen pool's own `swap_exact_in` moves and emits `route` on the router and `swap`
   on the pool and on the adapter; with `bytes` naming a listed pool that pool is used, an
   unlisted one is `PoolNotListed`, two entries `WrongPoolHashesLength`; a three-token path is
   `UnsupportedPath`; a past deadline `DeadlineExpired`; a pair without a list
   `PairNotListed`; every pool refusing is `NoRoute`; the best single pool is the one with the
   largest output among the pools of test 14 of E-006 rebuilt on real pools; `set_pools`
   refuses a pool of another pair, a duplicate and a fifth; the adapter's balance of every
   token is `0` after every test; a call without the trader's authorisation of the adapter
   invocation fails in the host; the exact-out mirror for each case.
2. Cost: a benchmark row in the crate's README for `swap_exact_tokens_for_tokens` with four
   listed band pools (four view calls and one router leg), the method of E-007 §4.16.1; it
   must stay under a quarter of both mainnet limits, so that three other venues fit beside it
   in one aggregator transaction, or `MAX_POOLS_PER_PAIR` is lowered and the README says so.

## 5. What is not built here

- The change to the aggregator's `Protocol` enum: Soroswap's contract, the owner's pull
  request, through the outreach.
- A route through an intermediate asset, or a plan of several Rhumb legs in one adapter call
  (E-006 §5, §3.1 above).
- Any change to the engine, the SDK or the quote API.

## 6. Done when

- `crates/adapter-soroswap` exists in the contracts repository, is deployed on testnet from
  its artefact by the script of §3.2, the entry `soroswap-aggregator` stands in
  `chain-config/testnet.toml`, and the deploy record (the Wasm, its hash, the ids) is
  archived in the brain.
- Every test of §4 exists, was committed before the implementation in lint-clean commits, and
  passes; the benchmark row stands in the crate's README.
- The contracts repository's security reviewer (Fable, high) and the exploit reviewer of I-001
  have approved every pull request.
- This spec is copied to `contracts/spec/A-007-aggregator-adapter.md` unchanged.

## 7. Appendix: the data this spec needs

An A-spec carries its own data and never refers outside `spec/` for a value. The router's and
the pool's published interfaces are A-005 §7.9 and §7.10, which the contracts repository holds
in its own `spec/`.

### 7.1 The Soroswap aggregator

| Network | Contract | Id | Source | The source's date |
|---|---|---|---|---|
| mainnet | aggregator | `CAYP3UWLJM7ZPTUKL6R6BFGTRWLZ46LRKOXTERI2K6BIJAWGYY62TXTO` | `github.com/soroswap/aggregator`, `public/mainnet.contracts.json` | commit `82c15fd2483ed9a7ba7132fb2499147650a5286b`, 2025-06-16 |
| mainnet | its adapters: Soroswap, Phoenix, Aquarius | `CC6KQUATUBCIFZRDJL5X5PHCYGOHLPHKZQPUOTZTQTASGU5AUQ6DS7SC`, `CCEBUGFV3D73OMV7MUXXA43AREY53MUHVD5SMUM7YZODNGY4NZBA2TSC`, `CDHDUKHFZB6FORHEBZCNYI3GGVNVOLEITSGI7OKU4UIQND5QG75KGSRR` | the same file | the same commit |
| testnet | aggregator | `CC74XDT7UVLUZCELKBIYXFYIX6A6LGPWURJVUXGRPQO745RWX7WEURMA` | the same repository, `public/testnet.contracts.json` on `main` | commit `84de10e0f8d26168b4a76f8c23b963e50917517c`, 2025-12-22, "deployed testnet contracts" (read 2026-09-21) |
| testnet | its Soroswap adapter, Phoenix adapter | `CCFXCAP7GT7XFTKGPENDZFN6MZ5VMPFONUHH46VKVN7E75C7FWA7NQ66`, `CBQBQXFQAQ5BQH5RATLBVYE5OTRJFRFPC6WBQNPENI22UODKPL2BZZB5` | the same file | the same commit |

Read 2026-09-21 from `contracts/aggregator/src/models.rs` at that commit: the venue set is the
closed enum `Protocol { Soroswap = 0, Phoenix = 1, Aqua = 2, Comet = 3 }`; a distribution is
`DexDistribution { protocol_id: Protocol, path: Vec<Address>, parts: u32, bytes:
Option<Vec<BytesN<32>>> }`, at most `MAX_DISTRIBUTION_LENGTH = 15` per swap; an adapter is
registered as `Adapter { protocol_id: Protocol, router: Address, paused: bool }` through the
admin's `update_adapters(adapter_vec: Vec<Adapter>)`. The aggregator's
`swap_exact_tokens_for_tokens(token_in, token_out, amount_in, amount_out_min, distribution,
to, deadline)` gives every distribution `(amount_in · parts) / total_parts`, the last the
remainder, and calls each adapter with that amount, `amount_out_min = 0`, the path, `to` and
the deadline, checking its own total against `amount_out_min` afterwards; the exact-out
mirror calls each adapter with `amount_in_max = i128::MAX`. The testnet aggregator lists
Soroswap and Phoenix; it cannot list Rhumb until the enum changes (§3.4).

### 7.2 The adapter trait

`contracts/adapters/interface/src/lib.rs` of `soroswap/aggregator` at commit
`84de10e0f8d26168b4a76f8c23b963e50917517c` (read 2026-09-21), copied here so that the builder
implements exactly this and the reviewer checks against it:

```rust
fn initialize(e: Env, protocol_id: String, protocol_address: Address) -> Result<(), AdapterError>;
fn swap_exact_tokens_for_tokens(env: Env, amount_in: i128, amount_out_min: i128, path: Vec<Address>,
    to: Address, deadline: u64, bytes: Option<Vec<BytesN<32>>>) -> Result<Vec<i128>, AdapterError>;
fn swap_tokens_for_exact_tokens(e: Env, amount_out: i128, amount_in_max: i128, path: Vec<Address>,
    to: Address, deadline: u64, bytes: Option<Vec<BytesN<32>>>) -> Result<Vec<i128>, AdapterError>;
fn get_protocol_id(e: &Env) -> Result<String, AdapterError>;
fn get_protocol_address(e: &Env) -> Result<Address, AdapterError>;
```

Its error codes (`error.rs`, the same commit): `NotInitialized = 401`, `AlreadyInitialized =
402`, `NegativeNotAllowed = 403`, `ProtocolAddressNotFound = 404`, `DeadlineExpired = 405`,
`MissingPoolHashes = 406`, `WrongMinimumPathLength = 407`, `WrongPoolHashesLength = 408`. The
Rhumb adapter keeps these numbers for the conditions it shares and adds `UnsupportedPath =
420`, `PairNotListed = 421`, `PoolNotListed = 422`, `NoRoute = 423`, `NotPoolOfPair = 424`,
`DuplicatePool = 425`, `TooManyPools = 426`, `Unauthorised = 427`. Soroswap's own adapter
(`contracts/adapters/soroswap/src/`) calls `to.require_auth()` and then its router's
`swap_exact_tokens_for_tokens(amount_in, amount_out_min, path, to, deadline)`, whose router
pulls the input from `to`; the Rhumb adapter does the same with E-008's `swap_exact_in(from =
to, plan, min_out, to)`.

### 7.3 Contracts (testnet, from `chain-config/testnet.toml` of the contracts repository at commit `7d000f1717636ed4048fca48eb8f4aa9239bcdd7`, the version 8 redeployment of 2026-09-30; written 2026-09-16 at the merge of pull request 17 with the version 5 ids)

| Contract | Id |
|---|---|
| router (`version()` 2, the bytes of the version 8 run) | `CBTSORXAJ56XFEFMCLNNYZMV3XPNWQOABBPTZFX23OYL2HZS5HFOMBT2` |
| `USDY/USDC`, band, pool `VERSION` 8 | `CC544IEQQIFNLZAP6ZFNNZDVNF3N6TYDKW7ZKZ64FLLHADEOQ6BRHWDS` |
| `USTRY/USDC`, band, pool `VERSION` 8 | `CDKUHS6YCLAWWLHXUIMLYXAMMXGI74354CM3N7TN5GHQSR46FQTMPCMO` |
| `PYUSD/USDC`, stable, pool `VERSION` 8 | `CAZJJGXRYHY4NLCX3E7RVYIKYZZ3MDMMLIXT73SSUDHJYQPRWRUBXWJU` |

The scripts read these from `chain-config/testnet.toml`, never from code; this table is what
the reviewer checks the entries against. The adapter routes through `[router]` alone and
names pools of `[pools]` alone: no contract of `[superseded]` (the version 5 and 6 pools,
the old router and the version 1 issuer adapters, A-002 §7.1) is ever a target, and a
`[superseded]` row may be a router or an adapter without a `pool` field (A-005 §7.1).
Mainnet: no Rhumb contract until the deploy ADR.

### 7.4 Reference values

`MAX_POOLS_PER_PAIR 4` (the same bound the SDK applies to a pair, A-005 §7.8), the protocol
id `"rhumb"`, the error codes of §7.2.
