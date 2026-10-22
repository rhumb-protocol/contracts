# A-001: Rate adapters and the keeper

Date: 2026-09-12
Status: done; amended 2026-09-29 (ADR-013, from the exploit pass over `contracts`): the
RedStone adapter's instance TTL rests on the keeper's weekly operation and `check-ttl.sh`
alone (§3.2, §3.4), both adapters gain an `init` event in one small pull request before the
deploy ADR, deployed on testnet in the version 8 redeployment (§3.2, §3.3), and the
constructors' authorisation is the deployer's alone, recorded (§3.3); amended 2026-10-01
(ADR-014 §6, the owner's choice of the second endpoints): the keeper reads each chain
through two endpoints in order with one failover rule, a keyed URL in the secret file, the
endpoint named in every observation (§3.4, §4 test 9, §7.9); from that pull request,
2026-10-01: no failover on a submission, the secret file as one line per key and read by
`observe` and `status` too, no failover for the issuers' APIs, the CSV header (§3.4);
corrected 2026-10-01: `centrifuge-evm` reads Ethereum through the list with failover like
`ondo-evm`, only the issuers' APIs are exempt (§3.4 point 4, §4 test 9; one small
`contracts` pull request, tests first, as pull request 52 refuses its list);
merged 2026-10-01 (contracts pull request 52, commit
`376967406f505a63e36781f7318035ea95f25bdc`) and run on testnet and mainnet since that day;
amended 2026-10-01 (ADR-014 §6 point 25): `keeper observe <network> --endpoint 2`, one
reading forced through the second endpoint, read-only (§3.4, §4 test 10); merged as
contracts pull request 53 and run on testnet and mainnet since 2026-10-02 at commit
`5c9d3df9b6dbf6c808e6c855d9f6151479315722` (CI run `37018756912`), the forced reading of
mainnet showing endpoint 2 for USDY and USTRY on both chains (ADR-014 §6 point 25 met);
amended 2026-10-02: `centrifuge-evm` asks `eth_getLogs` in pieces of at most 2 000 blocks
and merges them (§3.4, §4 test 5); queued as one small `contracts` pull request, tests
first, before the deJTRSY ADR; settled 2026-10-05 (contracts pull request 54's five
questions): pieces newest first, stopping at the first event, failover per piece, the span
as `eth_getlogs_block_span` per chain (reference `2000`), logs with `removed: true`
ignored and the order sorted by the reader (§3.4, §4 test 5); settled 2026-10-05
(contracts pull request 55's four questions): a log outside the range, address or topics
refused with a failed reading, an empty block header an endpoint error, a missing
`removed` read as false with a journal line, one `endpoint 2` journal line per piece;
merged 2026-10-05 (contracts pull request 56: a fixed clock in the reader's tests, the four
log checks, no unedited provider text in the journal); settled 2026-10-05 after it: the
block number of a header compared with the block asked, a difference an endpoint error, and
the `endpoint 2` line of a piece naming its block range (§3.4, §4 test 5); queued as one
small keeper pull request; queued 2026-10-09 (from the incident plan: a stolen
governance key owns the adapters for good): `set_governance` of both adapters behind the
delay and the guardian's veto, by the pattern of E-007 §3.2, in the adapters' next
`VERSION`, one small `contracts` pull request, tests first (§3.3); amended 2026-10-12
(A-010 step 3, ADR-015 §4): this spec is copied into the public `contracts` repository, so
its text names no folder of the brain and no private component by name; the references to
the brain's research and plans and to the indexer's alerting unit are rewritten as what
they say (the values stand in §7), nothing in the rules changed; amended 2026-10-16, from
the testnet outage of 2026-10-15 and 16 (pool USDY/USDC `rate-stale` after a `post` that
succeeded and a `refresh` that did not): a refresh-pending state per pool after a successful
`post`, retried every cycle until a `refresh` of that pool succeeds, the rule "refresh only
after a post" kept for a pool without a pending refresh (§3.4); one keeper pull request by
ops, tests first, together with the `extend-ttl` ceiling of A-002 §3.10 of the same day;
amended 2026-10-16 (A-003 §7.2): the testnet Soroban list has two entries, Validation Cloud
testnet as `secret` first while the SDF endpoint falters and the SDF endpoint second, the
keeper's testnet configuration following that order (§3.4; ops pull request 77); amended
2026-10-20 (ADR-014 accepted, `26b3504`): §7.4 carries the mainnet `RateConfig` and
`PoolConfig` values the ADR fixed
Target: `rhumb-protocol/contracts`, crates `adapter-redstone` and `adapter-issuer` (Soroban
contracts), the host program `keeper/`, and the first entries of `chain-config/`.
Depends on: A-000 (the repository, done), ADR-007 (the sources, the configurations, the rules
for `chain-config/`, the keeper's key), E-007 §5 and §3.2 (the adapter interface and
`refresh`), E-003 §3.2 (`Observation::from_sep40`, what the pool does with what an adapter
returns). Blocks: A-002 (the deploy scripts; no pool is created before its two adapters exist).

## 1. Goal

Give every band pool of Rhumb its two sources (ADR-007 point 1) as two small contracts the pool
calls, and the one program that keeps them fed: an adapter that wraps a SEP-40 feed of RedStone,
an adapter that holds the issuer's published price as our keeper posts it, and the keeper that
reads the issuers, posts, calls `refresh`, keeps ledger entries alive and reports what it sees.
The adapters convert nothing but the shape and decide nothing; every gate is the engine's
(E-003). After this spec, a pool for USDY or USTRY can be deployed by A-002 with two live
sources, and the keeper can measure, in its observe-only mode, the conditions ADR-007 point 2
sets for deJTRSY.

Adapter means: the contract per source that answers a pool's `observe()` with a price, its
decimals and its time (E-007 §5). Keeper means: a program on the owner's machine that runs on
a schedule, holds the operational key of ADR-007 point 5, and signs only what that point lists.
SEP-40 means: the Stellar oracle interface (`lastprice(asset) -> Option<PriceData { price:
i128, timestamp: u64 }>`, `decimals() -> u32`). Observe-only mode means: the keeper reads and
logs but posts nothing.

## 2. Current state (2026-09-12)

- `rhumb-protocol/contracts` exists with the layout of A-000, a placeholder crate as its only
  workspace member (A-000 §3.2), an empty `crates/`, `chain-config/` and `scripts/deploy/`, CI,
  and the local network on ports `4240` to `4242`.
- The pool contract of E-007 calls, in `refresh`, `observe()` on every address in its `Sources`
  and builds `Observation::from_sep40(price, decimals, timestamp, i)`: it accepts `price > 0`,
  `decimals <= 18`, a rate `price · 10^(18 - decimals)` inside `10^12 ..= 10^24`, and a
  `timestamp` in seconds at most `2^40`; a `None`, a failing call or a value outside those
  bounds is skipped for that source (E-007 §3.2, E-003 §3.2). Nothing else reaches the pool.
- RedStone's SEP-40 contract on mainnet is `CBMGLKUQZVSAIL5CPDDAWSUY7MAKXISHMOZEVLMBUWBMFGHRJSR4WYRF`
  and maps a Stellar token contract id to its feed (read on 2026-09-12; the ids and feed
  names stand in §7.2).
  Whether its `lastprice` timestamps are seconds, as SEP-40 says, or milliseconds, as RedStone's
  own contracts store internally, is not documented. Read on 2026-09-12 (§7.2): seconds; the
  feed reports 8 decimals, not 18; the USDY feed is named `USDY_FUNDAMENTAL/USD`.
- Ondo's price is public on Ethereum (`USDYOracleWrapper`
  `0x87b126e5518b6a1Bb8465779b4607C45C643DF90`, `getPrice()`, 18 decimals); Etherfuse's price
  is public at `https://api.etherfuse.com/lookup/tokens/cost/{symbol}` (no key, cached about
  five minutes, `current_time` in the response); Centrifuge's price is public through the
  `UpdatePricePoolPerShare` event on Ethereum and `https://api.centrifuge.io/` (research §3,
  §4, §6). None of them is on Stellar.
- No keeper key exists yet. ADR-007 point 5 defines it; the owner creates it outside every
  repository before the first testnet run.

## 3. What gets built

### 3.1 Layout and the first commit

```
contracts/
├── crates/
│   ├── adapter-redstone/     the SEP-40 wrapper (§3.2), one Wasm
│   └── adapter-issuer/       the keeper-fed adapter (§3.3), one Wasm
├── keeper/                   the keeper, its own Cargo workspace (§3.4), host only
│   ├── Cargo.toml
│   ├── src/
│   └── config/               mainnet.toml, testnet.toml: endpoints, intervals, no secrets
├── chain-config/
│   ├── mainnet.toml          contract ids (§3.5)
│   └── testnet.toml
└── tests/fixtures/           the pool Wasm of core for the integration test (§4), with its
                              origin recorded
```

The first commit of the pull request deletes `crates/placeholder` (A-000 §3.2) and adds the
tests of `adapter-redstone`, so that the workspace never has zero members and every commit
builds on its own (A-000 §3.3 rule 4). Every test commit is also lint-clean: `cargo clippy
--all-targets -- -D warnings` passes on it, so a stub's parameters are prefixed with `_` and a
helper that belongs to the implementation stays in the implementation commit (reviewer's note
on the adapters pull request, 2026-09-13). `keeper/` is excluded from the root workspace
(`[workspace] exclude`) because it is a host program with network and signing dependencies
that must not be built for `wasm32v1-none`; CI gets a second job for it (§3.6).

### 3.2 `adapter-redstone`

One contract, deployed once per feed, immutable after its constructor: a source is replaced by
deploying a new adapter and calling the pool's `set_rate` (E-007 §3.2), never by editing one.

- Constructor `(oracle: Address, asset: Asset)`, where `Asset` is SEP-40's enum
  (`Stellar(Address)` for a token contract id, `Other(Symbol)` for a named feed); stored in
  instance storage. For every pool of ADR-007 the asset is `Stellar(<token contract id>)`, the
  same id `chain-config/` records for the token (ADR-007 point 8). The constructor calls
  `oracle.decimals()` once and refuses to deploy when it fails or returns more than 18, so
  that an adapter on a feed the pool could never accept does not exist.
- `observe() -> Option<(i128, u32, u64)>`: calls `oracle.decimals()` and
  `oracle.lastprice(asset)`; returns `None` when the latter is `None`; otherwise
  `Some((price, decimals, timestamp))` exactly as received. No bound, no conversion, no
  storage write: the pool's `from_sep40` and the engine's gates do the judging. A failing
  oracle call fails `observe`, which the pool treats as a dead source (E-007 §3.2).
- Views: `oracle()`, `asset()`, `version()`.
- Events: `init` with `oracle` and `asset`, published by the constructor (added 2026-09-29,
  ADR-013 point 2, from finding 8 of the exploit pass: the deployment stored the feed and the
  asset and published nothing, so a watcher that reads events had no first ledger). An event
  is part of the interface: it is built with `VERSION` raised in one small pull request of
  its own for both adapters, tests first, the failing tests of the branch
  `exploit-pass/constructor-event` as its first commits, merged before the testnet
  redeployment of pool version 8 (ADR-013 point 2 as amended the same day); the deploy ADR
  fixes the adapter artefacts at that version or later, so every mainnet adapter publishes
  `init` from its first ledger. This adapter exists on mainnet only (A-002 §3.9) and is first
  deployed there with the event. The constructor stores the feed and the asset with the
  deployer's authorisation alone (§3.3, the same rule).
- The timestamp unit. SEP-40 defines seconds. Before the first testnet deployment the builder
  simulates `lastprice` on RedStone's mainnet contract for the USDY feed (a read-only
  simulation, no key, no transaction) and records the value and its unit in the pull request
  and in the crate's README. If the unit is milliseconds, the adapter does not convert: that is
  an open question for the brain, because a conversion would be a decision about the source.
  Done on 2026-09-12: the unit is seconds and the question is closed (§7.2). What remains per
  asset is the confirmation that the feed is fundamental, with its source (§7.2).
- No TTL code: the keeper extends the adapter's instance TTL with the ledger operation of
  §3.4, as it does for the pools. Stated 2026-09-29 (ADR-013 point 1, from finding 7 of the
  exploit pass): nothing in this adapter keeps its own instance alive. `observe` is the one
  entry point a pool calls and it stays a read, so that a pool's `refresh` writes nothing
  under the adapter's address and the keeper's bounded write set of `refresh` (A-002 §3.10)
  stands; the issuer adapter renews itself on every `post`, this one never. The weekly
  operation of §3.4 lists every RedStone adapter's instance by name beside the code entries,
  and `check-ttl.sh` (A-002 §3.8) checks and extends those instances before every mainnet
  deployment as it does the code entries. An archived RedStone adapter is a restore by a
  human and, until then, a band pool without its source 0: it serves its rate for `grace`
  past the last reading's `max_age` and refuses trades after (E-007 §3.2).

### 3.3 `adapter-issuer`

One contract, deployed once per issuer source, shared by every pool of that asset (the USDY
adapter serves both USDY pools when a wide tier exists).

- Constructor `(governance: Address, keeper: Address, decimals: u32)`, `decimals <= 18`; for
  every source of ADR-007 the deploy script passes `18`, and the keeper scales every source to
  18 decimals (§3.4). Stored in instance storage with `Latest: Option<(i128, u64)>`, empty at
  creation. The constructor stores `governance` and `keeper` with the deployer's
  authorisation alone and requires neither's signature (stated 2026-09-29, ADR-013 point 3,
  from finding 9 of the exploit pass; the rule of ADR-012 point 5 for the pool and the
  router): anyone can deploy an adapter naming the brand's addresses, and it is not the
  brand's. The brand's `chain-config/` is the registry, `deploy-adapter-issuer.sh` reads
  `governance()` and `keeper()` back after every deployment (A-002 §3.6), and a look-alike
  reaches no pool unless governance signs it into one. The test of the branch
  `exploit-pass/constructor-auth` is turned to this rule and taken in with the `init` pull
  request below, never merged as written. The constructor publishes `init` with
  `governance`, `keeper` and `decimals` (added 2026-09-29, ADR-013 point 2, from finding 8):
  built with `VERSION` raised in the one small pull request of §3.2, before the deploy ADR;
  the testnet issuer adapters are redeployed from the new artefact in the runbook of A-002
  §3.7 for pool version 8, first, the version 8 pools reading them, and their old ids move
  to `[superseded]` as the pools' and the router's do.
- `post(price: i128, timestamp: u64)` (`keeper.require_auth()`): refuses `price <= 0`
  (`BadPrice`), a `timestamp` older than the stored one (`NotNewer`; an equal one is
  accepted, so that a changed price under the same publication time can be posted, aligned
  with the keeper's rule of §3.4 on 2026-09-16, Rhumb pull request 20) and a `timestamp`
  greater than the ledger's (`FromTheFuture`); stores `(price, timestamp)`; extends the
  instance TTL when below its threshold (the values of E-007 §3.1); emits `post` with both
  values. It applies no step bound, no drift band and no staleness rule: those are the
  engine's, and a bound here would hide what the keeper posted.
- `observe() -> Option<(i128, u32, u64)>`: `Latest` with `decimals`, or `None` before the
  first post.
- `set_keeper(keeper: Address)` (`governance.require_auth()`): rotation of the operational key
  (ADR-007 point 5); emits `keeper`. `set_governance(governance: Address)` (the old governance
  signs): for the move to a multisig of E-007 §3.6; emits `governance`.
- Views: `keeper()`, `governance()`, `decimals()`, `latest()`, `version()`.
- Errors: `BadPrice`, `NotNewer`, `FromTheFuture`, `BadDecimals`, numbered from 300. That
  block is apart from the pool's codes (E-007 §3.5) and the router's own (200 to 214), but it
  shares its numbers with the planner errors the router reports from 300 (E-008 §3.5, corrected
  2026-09-13). On the chain that is harmless: an adapter's code can only surface in a keeper
  transaction (`post`, `set_keeper`, `set_governance`), never through a pool, which skips a
  failing source, and never through the router, which never calls an adapter; so a wallet or
  an indexer always knows the failing contract from the transaction it sent. The numbers stay.
- The timestamp is the source's own publication time (ADR-007 point 6, the contracts
  builder's rule "the source's own timestamp is the observation's time"): for Ondo the block
  time of the Ethereum read, for Etherfuse `current_time` of the response, for Centrifuge the
  block time of the `UpdatePricePoolPerShare` event, never the NAV's valuation time
  (`tokenPriceComputedAt`). The adapter cannot check this; the keeper's tests do (§4).

### 3.4 The keeper

A Rust program under `keeper/`, run by the owner as a service on one machine, one process per
network. It is not a contract, holds the key of ADR-007 point 5, and never runs inside an agent
session with a real key.

- Configuration: `keeper/config/<network>.toml` (committed; endpoints, intervals, the reader
  per asset, no secrets) and `chain-config/<network>.toml` (§3.5) for every contract id.
  Since 2026-10-01 (ADR-014 §6) `soroban_rpc` and `ethereum_rpc` are ordered lists of one
  or two endpoints each; an entry is a URL without a key, or the word `secret`, which stands
  for the URL the secret file carries under `soroban_rpc_secret` or `ethereum_rpc_secret`
  (the file of `KEEPER_SECRET_FILE` below, so a keyed URL is never in a committed file), and
  a list with two `secret` entries or an unknown entry refuses to start. The failover rule,
  the same for both chains and every reader: the first endpoint is asked first; on a
  transport error, an answer that is not a success, or no answer within
  `endpoint_timeout_seconds` (reference `10`, set from the probe of the providers whose
  result §7.9 carries), the second is asked once, in the same reading,
  and the reading fails if the second fails too; a reading never retries beyond that. Every
  reading records which endpoint answered as `1` or `2`, never the URL: the observe CSV
  gains a column `endpoint` per source, a post's journal line carries it, and a reading
  that fell to the second endpoint is one journal line of its own (`endpoint 2:
  <chain> <reader> <reason>`), so that `morning.sh` can count the first endpoint's failures
  per day (A-004 §4.10). The mainnet lists are Validation Cloud beside
  `mainnet.sorobanrpc.com` and Alchemy beside the public Ethereum endpoint (§7.9). On
  testnet (amended 2026-10-16, A-003 §7.2; until that day the testnet lists held one entry
  each) the Soroban list has two entries since 2026-10-16: `["secret",
  "https://soroban-testnet.stellar.org"]`, `secret` the Validation Cloud testnet endpoint
  of the owner's account, its keyed URL the `soroban_rpc_secret` line of the testnet
  keeper's secret file (A-004 §4.5), first in the list while the SDF endpoint falters (the
  outage of 2026-10-15 and 16), the public SDF endpoint second; the failover rule above
  holds unchanged, so a testnet reading asks Validation Cloud first and the SDF endpoint
  on its failure, and the order is swapped back by a configuration change alone.
  `keeper/config/testnet.toml` follows that order (ops pull request 77). The Ethereum
  list on testnet is unchanged. The
  testnet config names only the assets that have a testnet pool; the measurement of ADR-007
  point 2 for deJTRSY runs in observe-only mode against mainnet, where the token and both
  sources are, so `keeper/config/testnet.toml` does not name it (2026-09-16). The
  key comes from the file named by `KEEPER_SECRET_FILE`, outside every repository, read once
  at start; the program refuses to start when the variable is unset, when the file is
  world-readable, or when the file lies inside a git working tree.
- Amended 2026-10-01, from the failover pull request, five points that the rule above left
  open. (1) **Submitting has no failover.** A signed transaction (`post`, `refresh`, an
  extension) is sent once, to the endpoint whose simulation it was built from, and never to
  the other: a submission that reached the first endpoint and timed out may have been
  accepted there, and a second submission would be a second transaction, or a second fee for
  the same sequence number. The failover rule covers reads and simulations alone; a failed
  submission is logged and left for the next cycle, as today. (2) **`observe`, `status` and
  `check-ttl.sh` read the secret file too**, for the two URL lines and nothing else, so that
  the mainnet observe-only mode reads Stellar through both endpoints and the keeper's
  mainnet journal can show the reading from the second one that ADR-014 §6 point 25 asks for
  before the first pool. The observe unit of A-004 §8.2 therefore names `KEEPER_SECRET_FILE`
  as the other two do, and the mainnet file carries no seed until the deploy ADR's key is
  placed (A-004 §4.5). What keeps the observe-only mode from signing is now that it never
  builds a transaction, held by test 7 (no transaction submitted) and not by the absence of
  a key; a secret file without a `seed` line is refused by `run` and `extend-ttl` and
  accepted by `observe` and `status`. (3) **The secret file has one line per key**:
  `seed`, `soroban_rpc_secret`, `ethereum_rpc_secret`, each at most once, any other key a
  refusal to start; its exact form is in A-004 §4.5. A file of one line that is a bare
  seed, the file of today's testnet server, reads as `seed` alone, so the release needs no
  change of the file on the day it is installed. (4) **The issuers' own APIs have no
  failover** (corrected 2026-10-01: an earlier text of this point exempted `centrifuge-evm`
  as well). `etherfuse-api` reads `api.etherfuse.com`, the issuer's own API, one source and
  not an RPC endpoint with a provider behind it; the keeper reads it at its one URL and a
  failed read is a failed reading, caught by the staleness gate (the pool's `max_age` and
  `grace`, E-003 §3.2) and never answered with a guess. `centrifuge-evm` is an Ethereum
  reader like `ondo-evm`: its `eth_getLogs` goes through the Ethereum JSON-RPC list under
  the failover rule (ADR-014 §6 point 22); only its cross-read of `api.centrifuge.io`, the
  issuer's API, stays one URL without failover. The failover rule holds for the Soroban RPC
  and the Ethereum JSON-RPC lists and for those alone, and the config refuses a list under
  `etherfuse-api` and under the cross-read URL of `centrifuge-evm`. (5) **The observe CSV gains its columns
  without losing its history**: a CSV of twelve columns that an earlier release wrote gets a
  new header line at the first write of the new release, below the old rows, and the rows
  after it carry the `endpoint` columns; the old rows stay as they were and a reader that
  reads the CSV by header (the brain's session, A-004 §4.11) reads both parts. The file is
  never rewritten and never split.
- Amended 2026-10-01 (ADR-014 §6 point 25): **`keeper observe <network> --endpoint 2`**
  takes one reading on purpose through the second endpoint of each list, so that the mainnet
  journal shows a reading from the second endpoint of each chain without waiting for the
  first to fail. The flag exists on `observe` alone, which builds no transaction; `run`,
  `extend-ttl`, `status` and `restore-check` refuse it, and `observe` refuses any value but
  `2`. With it, every read through a Soroban RPC or Ethereum JSON-RPC list goes to the second
  entry only: the first is never asked and there is no failover back to it, so a failing
  second endpoint is a failed reading, reported as one, and never a reading through the first
  in its name. The issuers' own APIs (`etherfuse-api` and Centrifuge's API, point 4) are read
  as always. The CSV row carries `endpoint 2` in `issuer_endpoint` and `feed_endpoint` for each
  source read through a list, and the journal line names the reading as forced
  (`endpoint 2 (forced)`), so that `morning.sh` does not count it as a failover of the first
  endpoint. A second endpoint that is not configured (no `secret` entry, or no URL line in
  the secret file) refuses the run before anything is read. The run is started by hand,
  once per need, as the keeper user (A-004 §4.8); no timer carries the flag.
- Readers, one per issuer, each returning `(price scaled to 18 decimals, publication time in
  seconds)` or an error, never a guess:
  - `ondo-evm`: `eth_call` of `getPrice()` on the configured contract at the latest block and
    `eth_getBlockByNumber` for that block's timestamp, through the Ethereum JSON-RPC
    endpoints of the config in order, under the failover rule above (a public endpoint, or
    one whose keyed URL lives in the same file as the keeper's, never in the config).
  - `etherfuse-api`: `GET /lookup/tokens/cost/{symbol}`; price from `token_cost_in_usd` for
    USTRY and `token_cost_in_fiat` for CETES; time from `current_time`; the reader also reads
    `GET /lookup/stablebonds` at start (public; `GET /ramp/assets` answers 401 without a key,
    amended 2026-09-13) and, on mainnet, refuses to run when the Stellar asset it names for
    the symbol derives to a contract id other than the one in `chain-config/` (ADR-007 point
    8). On testnet and on the local network the same check runs, prints the derived and the
    recorded id side by side with the reason, and continues (amended 2026-09-14): the tokens
    there are test assets Rhumb issued itself and can never be the issuer's, so a stop there
    would only block every rollout that is not mainnet. The same holds for an issuer that
    cannot be reached at start (a transport error, a timeout, an answer without the symbol):
    on mainnet the keeper stops, on testnet and on the local network it reports the failure
    the way it reports a mismatch and goes on (explicit since 2026-09-15). The network is told
    apart by its passphrase, never by a name or a flag; a test holds that the mainnet
    passphrase stops on both, and the other two report on both. The three passphrases are
    written once, in the `sac` module that derives asset contract ids from them, and
    `main.rs` and every other place that tells networks apart take them from there (chore,
    2026-09-15).
  - `centrifuge-evm`: the newest `UpdatePricePoolPerShare` event for the configured share
    class through `eth_getLogs`, its price and its block time, through the Ethereum JSON-RPC
    endpoints of the config in order, under the failover rule above, cross-read against
    `api.centrifuge.io`'s `tokenPrice` for the same token; a difference between the two is
    logged and the event's value is used; `tokenPriceComputedAt` is read for the log line and
    never posted. Amended 2026-10-02 (the forced reading through Alchemy failed with
    `transport: status 400`, the reader having asked `eth_getLogs` over a block range larger
    than the provider allows): the reader asks its block range in consecutive pieces of at
    most 2 000 blocks, or of the provider's own limit when that is lower, each piece one
    request under the failover rule, and merges the answers in block order before it takes
    the newest event. A piece that fails on every endpoint fails the reading, never a
    merge of the pieces that answered. Settled 2026-10-05 (the five questions of contracts
    pull request 54): (1) the reader walks the pieces from newest to oldest and stops at the
    first piece that holds an `UpdatePricePoolPerShare` event for the share class, since it
    wants the newest one only; the older pieces are not asked. (2) The worst case is
    therefore small, and the service's `TimeoutStartSec=600` (A-004) stays as it is. (3)
    Failover is per piece: every piece is one request under the failover rule above, so one
    reading may hold pieces answered by endpoint 1 and by endpoint 2; the reading records
    `2` when any piece fell to the second endpoint. (4) The span is a configuration value,
    `eth_getlogs_block_span` in `keeper/config/<network>.toml`, one value per chain beside
    `ethereum_rpc`, reference `2000`; it is set to the lower limit of the chain's two
    endpoints, a missing or zero value refuses to start, and no provider limit is read at
    run time. (5) A log with `removed: true` is ignored, and the reader sorts what it keeps
    by `blockNumber` and then `logIndex` itself, never trusting the provider's order.
    Settled 2026-10-05 (the four questions of contracts pull request 55): (1) a log outside
    the piece's requested block range, or from another address, or with other topics than
    the request named, is refused, and the reading fails with a reason that names it,
    never skipping the log and reading on. (2) An empty block header for a block that a
    kept event names counts as an endpoint error under the failover rule, so that request
    goes to the second endpoint. (3) A log without a `removed` field reads as
    `removed: false`, with a journal line that says so. (4) A request that fell to the
    second endpoint is one `endpoint 2` journal line of its own, one per piece, as the
    failover is per piece; `morning.sh` therefore counts the first endpoint's failed
    requests, not its failed readings.
    Settled 2026-10-05, after contracts pull request 56: (1) the reader compares the block
    number in the header an endpoint returns with the block it asked for; a header for
    another block counts as an endpoint error under the failover rule, as an empty header
    does, so that request goes to the second endpoint, and a reading never takes its time
    from a block it did not ask for. (2) The `endpoint 2` journal line of a piece names the
    piece by its block range: `endpoint 2: ethereum centrifuge-evm blocks <from>-<to>
    <reason>`, so that a line in the journal says which request fell over, not only which
    reader. The line begins with `endpoint 2:` and is one line, its reason the keeper's own
    words and never the provider's text (A-004 §4.10), because `morning.sh` counts only a
    line that begins so.
  - Decimal strings are parsed to integers by hand (a fixed-point parser with its own tests);
    no floating point anywhere in the price path. A source with more than 18 decimals is
    floored to 18 and the reader's doc comment says so.
- Post cycle, per asset, every `interval` (reference: 15 minutes, RedStone's heartbeat being
  12 hours and the issuers' cadence a day). An asset of the keeper's config whose token is not
  in `chain-config/<network>.toml` is skipped with a line that names it, as observe-only mode
  skips it (amended 2026-09-16), unless a pool of that asset stands in `[pools]`: a pool
  without its token entry is a broken configuration, and that stops the run. For every other
  asset: read; when the publication time is newer than the
  last posted one, build and sign `post` on the asset's issuer adapter, submit, wait for the
  result; then call `refresh` on every pool of that asset, whatever `post` did, so that a
  RedStone write that arrived in between reaches the pool too; log the `refresh` event's
  per-source outcome. A failed read or a failed transaction is logged and retried at the next
  cycle; the keeper never posts a value it did not read in this cycle. One refusal is handled
  inside the cycle (added 2026-09-14, from a USTRY post the adapter refused with
  `FromTheFuture`): a publication time can be seconds ahead of the last closed ledger, because
  the issuer's clock and the ledger's are not one clock. The contract keeps no tolerance, the
  adapter refuses a time past the ledger's and the keeper posts the source's own time, all
  three as they should be; the keeper therefore waits, re-reading the ledger's close time about
  every ledger, and submits the same `post` once that time has reached the publication time.
  It waits at most `max_skew_seconds` of `keeper/config/<network>.toml`, set from the largest
  skew measured so far and recorded in the log with every wait (reference `60` until measured);
  a skew beyond it is logged as an error and left for the next cycle, never posted with an
  altered time.
- Amended 2026-09-16, from the keeper's first hours on the server (USDY was posted every
  cycle, four times the same price `1146411010000000000`, because the reading's time is the
  block time of the read and so always newer than the adapter's): **a post is news or it is
  not made.** The keeper posts when the adapter holds nothing for the asset, when the
  reading's price differs from the price the adapter holds, or when the reading's time is one
  the source itself gave the value and is newer than the adapter's. Which of the two a time
  is, is a property of the reader, stated in its doc comment and carried on the reading:
  `centrifuge-evm`'s time is the block of the price event, a publication; `ondo-evm`'s is the
  block of the read and `etherfuse-api`'s the `current_time` of the response, both the time
  of the read and not of the value. An unchanged price under a newer read time is not
  posted: the adapter would accept it, the ledger would charge for it, and the pool would
  learn nothing. `refresh` follows a post and nothing else: a cycle that posted nothing calls
  no pool, so that a quiet cycle costs no fee, and the rule above ("whatever `post` did") is
  withdrawn for the pools of today, which read one source; the module comment of `cycle.rs`
  states the withdrawn rule and changes with it. Tests first, in a small pull request of the
  keeper: the decision over its cases per reader kind (adapter empty; same price, newer read
  time; changed price; same price, newer publication time; older time on both kinds); a
  cycle whose reading is not news submits neither `post` nor `refresh` (the recorded
  transport sees no `sendTransaction`); a cycle that posted refreshes every pool of the
  asset as before. For a pool that reads two sources (every mainnet band pool, ADR-007
  point 1; decided 2026-09-16), `refresh` also follows a RedStone write: each cycle the
  keeper makes the read-only simulation of `lastprice` on the pool's RedStone adapter that
  the observe-only mode already makes (no key, no fee), and calls `refresh` when the feed's
  timestamp is newer than the one the pool's last `refresh` read for that source, which the
  state file records per pool from the `refresh` event's per-source outcome. That is what
  the withdrawn rule was for: a feed that moved while the issuer stood still reaches the
  pool within a cycle, and a cycle in which neither source moved still costs nothing. Its
  tests come with the same pull request (a newer feed timestamp refreshes without a post; an
  equal one does not; a pool with one source never simulates), and the code path is exercised
  on the local network only when a pool there reads two sources, which today none does. One
  thing stays open until the observe-only week of ADR-007 point 2 has been read: whether a
  source that keeps a price flat for longer than `max_age` needs a heartbeat post (the
  unchanged price under the time of a new read, at most every so many hours). Testnet
  answers it: USDY compounds per calendar day (§7.3), so under this rule it posts about once
  a day, and the log shows whether a day is ever skipped. No heartbeat is built before then.
- Amended 2026-10-16, from the testnet outage of 2026-10-15 and 16 (endpoint 1 of the RPC
  timing out almost continuously from about 19:00 UTC; the keeper's `post` for USDY
  succeeded, the `refresh` of USDY/USDC that followed it did not, and because the next
  readings were no news nothing called the pool again, so it stood `rate-stale` with a fresh
  value in its adapter): **a post is not finished until its refresh is.** After a successful
  `post` the keeper records, in the state file, a pending refresh for every pool of that
  asset, and clears it for a pool only when a `refresh` of that pool has succeeded. Every
  following cycle tries the `refresh` of each pool with a pending refresh again, also when
  the cycle's reading is no news and posts nothing; a failed `refresh` is logged and retried
  at the next cycle like any failed transaction, and a submission still has no failover
  (point 1 above), so the retry waits for the cycle. The rule above, "`refresh` follows a
  post and nothing else", stays for a pool without a pending refresh: a quiet cycle in which
  nothing is pending still calls no pool and costs no fee. The pending refresh lives in the
  state file, so it survives a restart of the service. Tests first, in the keeper pull
  request ops opens for it, with the `extend-ttl` ceiling of A-002 §3.10 of the same day: a
  `post` that succeeded with a `refresh` that failed leaves the pool pending; the next cycle
  with a reading that is no news submits `refresh` for that pool and no `post` (the recorded
  transport sees one `sendTransaction`); a `refresh` that succeeds clears the pending state
  and one that fails keeps it; a pool without a pending refresh is not called in a quiet
  cycle, as before; the pending state is read back from the state file after a restart.
  Decided 2026-10-16 (ops pull request 79): when the state file cannot be written, the run
  stops after the first asset and posts nothing more; it does not carry on, because a run
  that carried on would post without recording, and a post the state file does not hold
  leaves its refresh pending nowhere.
- Observe-only mode (`--observe-only`): the same readers, plus a read-only simulation of
  `lastprice` on RedStone's SEP-40 contract for the same asset, logged side by side with the
  difference in basis points and both timestamps, and nothing signed. It takes `--asset
  <name>` to read one asset of `keeper/config/<network>.toml` (amended 2026-09-15; the deploy
  script of A-002 uses it for the pool's asset); without `--asset` it walks every asset of
  the config and skips, with a line that names the asset and the reason, one that
  `chain-config/<network>.toml` does not carry, and never stops on it: an asset is in the
  keeper's config for the measurement of ADR-007 point 2 before it has a token on that
  network, and a rollout must not be blocked by an asset it does not deploy. This mode needs no key
  and is the measurement ADR-007 point 2 asks for (deJTRSY: the two sources within the band
  on every business day of a month) and the check on RedStone's timestamp unit (§3.2). For
  deJTRSY the log also carries, on every Monday of the measurement (New York time, the market
  hours of ADR-007 point 6), the delay between the opening and the first publication of each
  source after the weekend: under the market-hours row the pool refuses trades from the
  opening until both sources have published, and the ADR that admits deJTRSY (ADR-007 point
  2) needs to know how long that is (agreed 2026-09-13). Its
  log is a CSV the brain can read; it is not a metric of the pipeline of rule 4 and is never
  written into governance.
- TTL: once a week the keeper submits an extend-footprint-TTL operation (a ledger operation
  any account may submit for any entry, no contract call) for the instance entries of every
  pool, every adapter and the router of the network, to the values of E-007 §3.1. This is the
  "keeper extends the instance TTL of every pool weekly" of E-007 §3.1. Stated 2026-09-29
  (ADR-013 point 1): "every adapter" names the RedStone adapters' instances explicitly, and
  this operation with `check-ttl.sh` (A-002 §3.8) is the only thing between a RedStone
  adapter and archival, because that adapter writes nothing itself (§3.2); the deploy check
  of A-002 §3.8 reports and extends those instances beside the code entries. The same operation
  extends the `ContractCode` entries of every Wasm hash those contracts run on (the pool, the
  router, `adapter-redstone`, `adapter-issuer`), to the same values (added 2026-09-13): no
  contract call extends a code entry, an expired code entry fails every `post` and `refresh`
  whatever the instance TTL says, and nobody else does it. The keeper takes the hash of each
  deployed contract of `[router]`, `[adapters]` and `[pools]` from the chain, the
  `ContractInstance.executable` of its instance entry, extends that code entry, and compares
  the hash with the `[artefacts]` row of the contract's kind in `chain-config/<network>.toml`
  (amended 2026-09-15): a contract without a row, a row under the wrong name, or a hash that
  differs from the row's stops the run on mainnet and is reported on testnet and the local
  network; a row that no deployed contract runs is logged once per run as "not matched" and
  skipped, never a stop. A-002 checks the code TTL of every deployed contract before the
  first mainnet pool, as a deploy check.
- Watch: the keeper follows the `rate` events of every pool and logs every status change with
  its reason; on a suspension it writes an alert line and does nothing else, because only the
  guardian acts (E-007 §3.2). Alert transport beyond the log is the indexer's concern
  (brief §4.4). The walk over `getEvents` reads the pools in groups of at most five contract
  ids, one cursor per group, and continues from each answer's cursor until its `latestLedger`
  is reached: one answer covers a window of the range, not the range, so an empty page is not
  the end (amended 2026-09-16, the same defect the indexer's setup found; a small pull request
  with the test first).
- Dependencies pinned exactly, from the Stellar organisation's crates for XDR, RPC and
  signing where they exist; no shelling out to any CLI; no float in the price path; the
  workspace lints of the root `Cargo.toml` apply to the keeper too, with `std` allowed.
- Trace output (`KEEPER_TRACE=1`) never prints a URL: an endpoint may carry an access key in
  its URL (the second endpoint of either chain), so a trace line names the reader, the
  method and the endpoint's number and shows the request body and the answer, with the URL
  left out (reviewer's note, 2026-09-13; the number since 2026-10-01).

### 3.5 `chain-config/`

`chain-config/<network>.toml`, one table per kind: `tokens`, `sources` (the SEP-40 contracts
of oracles), `adapters`, `pools`, `router`; every value a Soroban contract id, never an asset
code or an issuer (ADR-007 point 8). This spec writes `tokens` and `sources` for mainnet and
testnet; `adapters`, `pools` and `router` are written by A-002 after each deployment.

- `tokens`: USDC, USDY, USTRY, PYUSD, and, so that the observe-only mode can measure them,
  deJTRSY and USDT0; each entry carries the id, how it was obtained (the Stellar Asset Contract
  id derived from the classic asset and confirmed by a read of `name()` and `decimals()`; the
  Soroban token's id from the issuer's own publication) and the date. The Centrifuge ids are the
  Soroban contracts of §7.1, never the classic look-alikes. Etherfuse's ids are taken from
  `GET /lookup/stablebonds` and the entry records the response's date (ADR-007 point 8, as
  amended 2026-09-13).
- `sources`: RedStone's SEP-40 contract per network, with the feed name per token as a
  comment and, per feed, where and when its fundamental kind was confirmed (§7.2); a testnet
  entry only where RedStone publishes there; otherwise the testnet pool of A-002 runs on one
  source, the issuer adapter, which E-007 allows off mainnet (amended 2026-09-13 by A-002
  §3.9: a mock oracle nobody feeds goes stale within `max_age`). Every id comes with the
  external source of §7.1 it was checked against; the ids of `keeper/README.md` are not one.

### 3.6 CI, README, hygiene

- The root job of A-000 runs on the two adapter crates. A second job `keeper` runs
  `cargo fmt`, `cargo clippy --all-targets -- -D warnings` and `cargo test` inside `keeper/`,
  and a third job starts `scripts/local-network.sh`, runs the tests behind the `localnet`
  feature (§4) and stops it. Every `cargo` invocation of every job passes `--locked`, so the
  committed lockfile is the one that is built and tested (reviewer's note, 2026-09-13). No job holds a credential; the local-network tests generate what
  they need (§4).
- `README.md` gains a section per crate and one for the keeper: what it does, how to run it
  in observe-only mode without a key, how the owner runs it with one, and the two sources per
  asset of ADR-007 as a table with contract ids once A-002 has deployed them.
- Every rule of A-000 §3.3 applies: tests first, rounding in names, no float, no key, no AI or
  personal name, no edit of the engine. The security reviewer's four brand items (A-000 §3.5)
  are the review points of this pull request.

## 4. Verification (the tests of this spec)

Unit tests with the SDK's test environment, property tests where a range is involved:

1. `adapter-redstone`: against a mock SEP-40 contract in the test crate, `observe` returns
   exactly what `lastprice` and `decimals` return for every price in `i128`, every `decimals`
   in `0 ..= 32` and every timestamp in `u64` (property test); `None` from `lastprice` gives
   `None`; a `lastprice` that fails makes `observe` fail; a `decimals` above 18 or failing
   refuses the constructor.
2. `adapter-issuer`: `post` by the keeper stores and emits; by any other address fails auth;
   `price <= 0`, a repeated or older timestamp and a timestamp past the ledger's fail with their
   codes; `observe` is `None` before the first post and the stored triple after;
   `set_keeper` and `set_governance` obey their auth and the new keeper can post while the old
   one cannot; the instance TTL is extended by `post` when below the threshold.
3. What the pool accepts: for every value the two adapters can return, a test restates the
   acceptance rule of E-003 §3.2 (`price > 0`, `decimals <= 18`, rate inside `10^12 ..=
   10^24`, timestamp at most `2^40`) and checks that the adapters never narrow it and never
   widen it: an adapter returns what it holds, and the rule is the pool's. This is a
   restatement of four conditions in a test, not engine code.
4. Integration through the real pool: the pool Wasm of core, built by the owner from the
   commit recorded in `tests/fixtures/README.md` with its SHA-256 and copied in by hand
   (refreshed only by hand, by its own commit, until core is public and CI can fetch a tagged
   release), is registered in the SDK's test environment with a band pool of `sources 2`
   under the USDY configuration of ADR-007 point 6, one `adapter-redstone` on a mock SEP-40
   contract and one `adapter-issuer`. The scenarios of E-003 test 7 driven through them: one
   source alone never moves the rate; two agreeing sources set the interval; a keeper post
   that disagrees by more than `cross_check` suspends with `SourcesDisagree`; a dead adapter
   drops out and `read` returns `SourceLost` after `grace`; a `None` from the mock is skipped
   and reported in the `refresh` event.
5. Keeper readers: against recorded responses in the test tree (an Ondo `eth_call` and block,
   an Etherfuse cost response and a `lookup/stablebonds` response, a Centrifuge log and a GraphQL
   response), each reader returns the expected 18-decimal integer and publication time; the
   decimal parser is property-tested against integer arithmetic for every scale from 0 to 30;
   the Centrifuge reader never returns `tokenPriceComputedAt` as the time (a fixture whose two
   times differ makes this visible); over a recorded range of more than 2 000 blocks it
   sends no `eth_getLogs` wider than 2 000 blocks, merges the pieces and returns the newest
   event, whichever piece holds it, and a piece refused on both stub endpoints fails the
   reading (added 2026-10-02); with `eth_getlogs_block_span` set to 2 000, a recorded range
   whose newest piece holds the event asks that piece alone, and one whose event sits two
   pieces back asks exactly three, newest first; a piece refused on the first stub endpoint
   and answered by the second gives a reading recorded as endpoint `2`; a log with
   `removed: true` that is newer than every other is not returned; a piece whose logs come
   back out of order returns the highest (`blockNumber`, `logIndex`); a config without
   `eth_getlogs_block_span`, or with `0`, refuses to start (added 2026-10-05); a piece whose
   answer holds a log outside its block range, one from another address and one with other
   topics each fail the reading with a reason that names the log; an empty block header
   for an event's block on the first stub endpoint is asked again of the second; a log
   without `removed` is kept as `removed: false` with its journal line; a reading whose
   three pieces fall to the second endpoint writes three `endpoint 2` journal lines (added
   2026-10-05, contracts pull request 55); a header for another block than the one asked,
   from the first stub endpoint, is asked again of the second, and a header for another
   block from both fails the reading; each `endpoint 2` line of a piece begins with
   `endpoint 2:` and names that piece's block range (added 2026-10-05, after contracts pull
   request 56); the Etherfuse
   reader refuses to start on an id mismatch.
6. Keeper on the local network (feature `localnet`, the CI job of §3.6): the test generates an
   ephemeral keypair in memory, funds it through the local friendbot, deploys `adapter-issuer`
   with that key as keeper through the RPC, runs one post cycle against a recorded reader
   response, and checks `observe` on the adapter and the `post` event; the key never touches a
   file and is discarded with the process. This is the one place in the brand's repositories
   where a key exists outside a human's hands, it is a throwaway on a throwaway network, and
   the test says so in its doc comment. This throwaway key is the one exception to rule 5 of
   A-000 §3.3 and to the builder's rule (confirmed 2026-09-13); no other test, script or agent
   ever holds a key. The keeper refuses to start with `KEEPER_SECRET_FILE`
   unset, world-readable, or inside a git working tree (three tests).
7. Observe-only mode: with a recorded reader response and a mock SEP-40 contract on the local
   network, the CSV line carries both prices, both times and the difference in basis points,
   for a deJTRSY fixture on a Monday the delay of each source since the opening (§3.4), and no
   transaction was submitted (the RPC's transaction count is unchanged).
8. Hygiene: `grep` for a Stellar seed, for `.env`, for an asset code with an issuer in
   `chain-config/`, and for the names of A-000 §4.6 finds nothing; `scripts/scan-secrets.sh`
   passes; the two Wasm files are below the bound of A-000 §3.8.
9. Failover (added 2026-10-01, ADR-014 §6): against two stub endpoints per chain in the test
   tree, a reading whose first endpoint refuses the connection, answers a non-success status,
   or answers after the timeout is served by the second with `endpoint 2` in the CSV row and
   in one journal line; a reading whose second endpoint fails too is an error and never a
   guess; a first endpoint that answers is never followed by a request to the second; a
   config whose list names `secret` reads the URL from the secret file and the trace never
   shows it; a list with two `secret` entries, an unknown entry or three entries refuses to
   start; and a cycle with one failover posts nothing differently from a cycle without one.
   Added 2026-10-01 with the five points of §3.4: a submission whose endpoint times out is
   not sent to the second endpoint (the recorded transport of the second sees no
   `sendTransaction`); `observe` and `status` start with a secret file that carries the two
   URL lines and no `seed`, and `run` and `extend-ttl` refuse it; a file whose one line is a
   bare seed reads as `seed`; a file with a repeated key or an unknown key refuses to start;
   a config that gives `etherfuse-api`, or the cross-read URL of `centrifuge-evm`, a list of
   endpoints refuses to start, while `centrifuge-evm`'s `eth_getLogs` whose first endpoint
   fails is served by the second with `endpoint 2` in `issuer_endpoint`, as `ondo-evm`'s
   `eth_call` is (corrected 2026-10-01, §3.4 point 4); and an existing CSV of twelve columns gets the new header once, its old rows
   unchanged and both parts read back by header.
10. Forced second endpoint (added 2026-10-01, §3.4): against the two stub endpoints of test 9,
   `observe --endpoint 2` sends every list read to the second stub and none to the first (the
   first stub's recorded transport is empty); the CSV row carries `endpoint 2` for each source
   read through a list and the journal line says `endpoint 2 (forced)`; a second stub that
   refuses the connection makes the reading fail, with no request to the first; `run`,
   `extend-ttl`, `status` and `restore-check` refuse the flag, `observe --endpoint 1` and
   `--endpoint 3` refuse to start; a missing URL line for the second endpoint refuses before
   any read; and no transaction is submitted (the transaction count is unchanged, as in
   test 7).

## 5. What is not built here

- The deploy scripts, the genesis deposits, the router deployment, the testnet run and the
  mainnet pools: A-002. No pool exists after this spec.
- A Reflector adapter, a Pyth adapter, or any adapter on a trade-derived source (E-003 §3.4,
  ADR-007). The answer to ADR-007's Reflector question is a read-only call the builder makes and
  records in the pull request; it produces no code.
- A CETES/MXN reader or a deJAAA reader: ADR-007 points 2 and 3.
- An adapter that converts timestamps, bounds prices, or falls back to another source: the
  engine gates, the pool skips, a human decides.
- Alerting beyond the keeper's log, metrics export, the "in range" rule, the share of on-chain
  trading: the indexer (its own A-spec).
- A keeper that acts on a suspension: it reports; the guardian resumes (E-007 §3.2).
- Any change to the engine, its specs or its agents; the pool Wasm fixture is a copy of a
  built artefact, not engine source.
- A new agent: the contracts builder and the security reviewer of A-000 §3.5 cover this spec.

## 6. Done when

- `crates/placeholder` is gone, `adapter-redstone` and `adapter-issuer` build for
  `wasm32v1-none` with the workspace lints, and `keeper/` builds and tests on the host.
- Every entry point, error and event of §3.2 and §3.3 exists and nothing beyond them is
  exported; the timestamp unit of RedStone's feed is recorded in the README and the pull
  request.
- Every test of §4 exists, was committed before the implementation, and passes in the three
  CI jobs; line coverage of the two contract crates' own source is 100 %, with the SDK's
  macro-generated lines excluded as in E-007 §6.
- `chain-config/mainnet.toml` and `testnet.toml` hold `tokens` and `sources` with the
  provenance of every id (§3.5), and every id was checked by the security reviewer against
  ADR-007 and the research.
- The keeper has run in observe-only mode against mainnet for USDY and USTRY for one week
  without a key, and its CSV (kept outside the repository) shows the two sources within
  `cross_check`; the pull request states the measured maximum difference.
- The security reviewer has approved the pull request. Review points: that the adapters
  convert nothing and bound nothing; the auth on `post`, `set_keeper` and `set_governance`;
  the monotone timestamp and the future check; that no reader can return a time that is not a
  publication time; the absence of floats in the price path; the key handling of the keeper
  and of test 6; every id in `chain-config/`.
- This spec is copied to `contracts/spec/A-001-rate-adapters.md` unchanged. Only then does
  work on A-002 start.

## 7. Appendix: the data this spec needs (added 2026-09-13)

An A-spec carries its own data and never refers outside `spec/` for a value: the brand's
repository holds `spec/` and nothing else of the brain, so a pointer to an ADR or to the
research is a pointer to a file the builder cannot open. Where the body above cites
ADR-007 or the research, the value it means stands here; where a value is not yet known, this
appendix says what it is, where it comes from and who fetches it. Every id and every feed name
below was read on 2026-09-12 unless a row says otherwise.

### 7.1 Tokens (mainnet)

`chain-config/` names a token by its Soroban contract id only. For a classic asset that id is
the Stellar Asset Contract (SAC) derived from `code:issuer`; for a Soroban token it is the id
the issuer publishes. The column "external source" is the provenance ADR-007 point 8 requires
and the reviewer checks; "status" says whether that check has been made. The ids in
`keeper/README.md` are RedStone's own token-to-feed mapping, confirmed only by each contract's
`symbol()`, which is a claim by the contract about itself: they are a starting point, never a
source for `chain-config/`.

| Asset | Classic asset (`code:issuer`) | Soroban contract id | External source (ADR-007 point 8) | Status |
|---|---|---|---|---|
| USDC | `USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN` (read 2026-09-17 from the `[[CURRENCIES]]` entry `code="USDC"` of `https://www.centre.io/.well-known/stellar.toml`, which also lists the issuer under `ACCOUNTS`; Horizon names the asset under that issuer with about 239 million authorised on the day, and the account's `home_domain` is `circle.com`) | `CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75` (RedStone's mapping) | derivation of the SAC id from the classic asset, issuer from Circle's `stellar.toml`, the date recorded | checked 2026-09-13 against `centre.io`'s `stellar.toml` and the account's own `home_domain`; one weak spot recorded in the entry's comment: Horizon reports the home domain as `circle.com`, whose `stellar.toml` answered 404, so SEP-1's loop does not close at the home domain |
| USDY | `USDY:GAJMPX5NBOG6TQFPQGRABJEEB2YE7RFRLUKJDZAZGAD5GFX4J7TADAZ6` | `CB3YA656OYIHU57657I5KGSBRHE5I3OZU4VFC22PYAOANFZHEWNYGAGP` | Ondo's addresses page (`docs.ondo.finance/addresses`) and Ondo's `stellar.toml` for the issuer; the SAC id derived from it | checked 2026-09-13: derived from the issuer `ondo.finance`'s `stellar.toml` names, whose home domain closes SEP-1's loop |
| USTRY | `USTRY:GCRYUGD5NVARGXT56XEZI5CIFCQETYHAPQQTHO2O3IQZTHDH4LATMYWC` | `CBLV4ATSIWU67CFSQU2NVRKINQIKUZ2ODSZBUJTJ43VJVRSBTZYOPNUR` (RedStone's mapping) | `GET https://api.etherfuse.com/lookup/stablebonds` (public; `/ramp/assets` answers 401 without a key), the response and its date recorded, and `etherfuse.com`'s `stellar.toml`; never typed from a document | checked 2026-09-13 against both; the keeper re-derives the id at every start |
| PYUSD | `PYUSD:GDQE7IXJ4HUHV6RQHIUPRJSEZE4DRS5WY577O2FY6YQ5LVWZ7JZTU2V5` | `CCCRWH6Q3FNP3I2I57BDLM5AFAT7O6OF6GKQOC6SSJNDAVRZ57SPHGU2` | Paxos's `stellar.toml` for the issuer; the SAC id derived from it | checked 2026-09-13: derived from the issuer `token-metadata.paxos.com`'s `stellar.toml` names, the home domain Horizon reports |
| USDT0 | `USDT0:GATISXX6BZ6NC7IKQBY37CJD4SOZL3CYZJWXEDG6JVIY4WBS6KXJHN6Q` | SAC `CBSJZEIO5C7KC2SF3MKSNXXJSW5G3VTNBX4ATMKUI3B2MR4JKM4R26YF`; OFT contract `CBOWOLFSDM5PZXNFIVDMP5NZ7U2GSIHED6H6R446QOHF266XINKUMMF6` | USDT0's deployments page (`docs.usdt0.to/technical-documentation/deployments`) | checked 2026-09-13 against that page: the derived SAC equals its Token entry, the OFT id equals its entry |
| deJTRSY | none: a Soroban token (LayerZero OFT, 18 decimals), created 2025-12-01 | `CBI7UCH5KGSVQRO5H4SUCZUTZABCITZLRHQQZTWL2TK4RZ72TAR6IHRV` | Centrifuge's own publication of the Stellar contract id: its documentation repository (`github.com/centrifuge/documentation`, `docs/developer/protocol/deployments/index.mdx`, commit `c69b51c87964e855be774f15640bfb732f9229ba`); the Soroswap token list, where the research took it from, is not an issuer publication | checked 2026-09-13 against that commit |
| deJAAA | none; for the record only, no pool (ADR-007 point 3) | `CC64WBDGS6QQP22QTTIACYIXT3WF7BBQEYOQPLTP7GTKYY7PZ74QYGSL` | as deJTRSY | not in `chain-config/` |
| CETES | `CETES:GCRYUGD5NVARGXT56XEZI5CIFCQETYHAPQQTHO2O3IQZTHDH4LATMYWC` | `CAL6ER2TI6CTRAY6BFXWNWA7WTYXUXTQCHUBCIBU5O6KM3HJFG6Z6VXV` (RedStone's mapping) | as USTRY (`GET /lookup/stablebonds`) | not in `chain-config/` until CETES/MXNe is admitted |

Look-alikes, never to be used: classic assets with the codes `deJTRSY` and `deJAAA` from the
issuer `GA3XIPEQ622KSQB2OPQQLRDOYXHWKJTL2NNHIIP2NWFSXWBLZ4O7GFDV` (home domain `ultramint.org`),
and seven issuers under the code `USDM1`. Flags worth knowing: USDY, PYUSD and USDT0 carry
`auth_revocable` and `clawback`; USTRY and CETES carry no flags.

Testnet: no token id is known today. The deploy script of A-002 creates or derives what the
testnet pools need and writes the entries with the same provenance rule; where RedStone
publishes no testnet feed, the pool runs on a mock oracle the script deploys, and the entry
says so (§3.5).

### 7.2 RedStone (source 0 of every band pool)

- Shared SEP-40 contract on mainnet: `CBMGLKUQZVSAIL5CPDDAWSUY7MAKXISHMOZEVLMBUWBMFGHRJSR4WYRF`,
  fed by the adapter `CA526Y2NQWGWVVQ7RFFPGAZMU66PSYJ3UC2MTVAV4ZU7OM5BOPHDXUSG`. `lastprice`
  is asked with `Stellar(<token contract id>)` of §7.1. Every `adapter-redstone` of Rhumb is
  deployed on this contract.
- `decimals()` returns `8`, not the 18 the body assumed (read 2026-09-12). `adapter-issuer`
  reports 18. `Observation::from_sep40` scales both to 18 exactly and never rounds, so the
  pool is unaffected; the two sources can differ by up to `10^-8` for the scale alone, which
  is the floor on how closely they can ever agree and is far inside every `cross_check` below.
- The timestamp unit is seconds. Read on 2026-09-12 at ledger 64400463 for USDY:
  `lastprice().timestamp -> 1789231300`; the simulation's diagnostic events show the inner
  call `read_price_data_for_feed("USDY_FUNDAMENTAL/USD")` returning `package_timestamp ->
  1789231300000`, so RedStone stores milliseconds and its SEP-40 wrapper divides by a thousand.
  The adapter converts nothing, and the open question of §3.2 is closed.
- Cadence: the heartbeat writes on weekends too, repeating or slightly moving the last value;
  the oldest reading seen was about six and a half hours old, which is what a 12-hour
  heartbeat looks like and what `max_age` has to live with.

| Feed name | Per-feed price contract | Threshold / heartbeat | Serves | Fundamental (NAV-derived), confirmed by |
|---|---|---|---|---|
| `USDY_FUNDAMENTAL/USD` | `CAEUQPFSHHZ2EH4RKMU6S2NEOZVMDMZZ47RRN5FZCAPDME5OACBHEOXM` | 0.5 % / 12 h | USDY/USDC | confirmed: the name, read on chain on 2026-09-12 through the diagnostic event above, and RedStone's node manifest (2026-09-13, below), which names the fetcher `ethereum-evm-fetcher-ondo`, Ondo's own oracle on Ethereum |
| `USTRY` | `CAEGYZFAJ3HLSDBUIVJ77SK5HLLS4MPE4LO5OB2W3L2TEDMAS6RCTMZ4` | 0.1 % / 12 h | USTRY/USDC | confirmed (closed 2026-09-13): RedStone's node manifest names the fetcher `etherfuse-ustry-api`, the issuer's own API, the same endpoint the keeper's `etherfuse-api` reader reads; the name carries no label, which is why the manifest is the proof |
| `deJTRSY_FUNDAMENTAL/USD` | `CBOERMDX6PS5QYPBCCTV26JUJIVPQ2MCODQGQFOWVVGFJZLARZCBWTDK` | 0.25 % / 12 h | deJTRSY/USDC, when admitted | confirmed: the name, and RedStone's node manifest, which names the source `ethereum-evm-fetcher-centrifuge`; the 13 bp gap of the research was 2 bp on 2026-09-13 (§7.7) |
| `PYUSD` | `CB3PNR7FSAHQH3UAVYKSJVPLWEQRPPYVFL2UIR4P3RYEPZLTOF7PL2L4` | 0.05 % / 12 h | watch source of PYUSD/USDC (indexer, never a pool) | market-based: the manifest names nine exchanges and two pools as fetchers; never a rate source, a watch source at most |
| `USDT0` | per-feed contract not found; the shared contract serves it | 0.5 % / 6 h | watch source of USDT0/USDC, when admitted | market-based: exchange and pool fetchers in the manifest; never a rate source, a watch source at most |
| `CETES` | `CCYGBQZ5JKE2TGK2XNNVIZRKFEIT27NZADS3XJFSO4CCIH3LHIUP34LG` | 0.1 % / 12 h | none until CETES/MXNe is admitted; quoted in USD | as USTRY |
| `MXNe` | `CDYTKTOHHMMZQSBGEVTFSOAMN6ABFCRZ6TCNLZNFQZ6OK3ZGDB3666IV` | 0.1 % / 24 h | none | not applicable |
| `deJAAA_FUNDAMENTAL/USD` | `CB3J6JU2L26HUMWPINRCUACGPJPSQPKPHSZLGQ2VV7YCJBFWERECZB7O` | 0.25 % / 12 h | none (dropped) | not applicable |

The rule, per asset, before a pool is pointed at a feed: the builder confirms that the feed is
fundamental (it reads the issuer's or the protocol's own value) and not market-based (it reads
trades on any venue), and records in the pull request and in the comment of the `sources`
entry of `chain-config/` where the confirmation comes from and when it was read. Three sources
of confirmation count: the feed name as read on chain through the diagnostic events of a
simulated `lastprice` (the inner `read_price_data_for_feed` call names the feed); RedStone's
relayer and node manifests in its public monorepo (`redstone-oracles-monorepo`, packages
`relayer-remote-config` and `node-remote-config`), which name the fetcher behind a feed; and
RedStone's written answer to the labs' question. A dashboard label alone, or a price that
happens to equal the issuer's, is not confirmation. A feed whose kind cannot be confirmed is
not a source (E-003 §3.4, A-000 §3.3 rule 6). The manifest used on 2026-09-13:
`redstone-oracles-monorepo`, `packages/node-remote-config/dev/manifests/data-services/primary.json`,
commit `57510d50776584eb9f4140464a103a82eaf47e1a`; the relayer manifest that names the SEP-40
contract and the per-feed triggers: `packages/relayer-remote-config/main/relayer-manifests-non-evm/stellarSep40.json`.

### 7.3 Issuer prices (source 1, read by the keeper)

| Asset | Reader (§3.4) | Where | What is read | Publication time to stamp | How the value moves |
|---|---|---|---|---|---|
| USDY | `ondo-evm` | Ethereum: `USDYOracleWrapper` `0x87b126e5518b6a1Bb8465779b4607C45C643DF90` over `RWADynamicOracle` `0xa0219aa5b31e65bc920b5b6dfb8edf0988121de0`; `getPrice()`, 18 decimals | the price at the latest block | the block time of the read | compounds per calendar day at an APY Ondo sets monthly in advance; no jumps; Ondo's REST price API is for other products and "not recommended as an oracle" |
| USTRY | `etherfuse-api` | `GET https://api.etherfuse.com/lookup/tokens/cost/USTRY`; identifiers from `GET .../lookup/stablebonds` (public; `.../ramp/assets` needs an authorisation header the keeper does not have); no key; cached about five minutes | `token_cost_in_usd` | `current_time` of the response | a rate reset weekly (Thursday, around noon MST), the price computed continuously from it, weekends included; no daily strike |
| CETES | `etherfuse-api` | as USTRY, symbol `CETES` | `token_cost_in_fiat` (MXN) | `current_time` | as USTRY; not read until CETES/MXNe is admitted |
| deJTRSY | `centrifuge-evm` | Ethereum: the newest `UpdatePricePoolPerShare` event for the deJTRSY share class, through `eth_getLogs`; cross-read against the GraphQL API `https://api.centrifuge.io/` (`tokenPrice`, `tokenPriceComputedAt`); the address of the emitting contract is taken from Centrifuge's own publication and recorded with the date | the event's price for the deJTRSY share class (its own price, not the fund's: 1.03649 against 1.11567 for JTRSY on 2026-09-12) | the block time of the event, never `tokenPriceComputedAt` | NAV struck at 12:00 UTC on US business days, published on Ethereum the next business day (median lag 26.5 h, up to 98 h over a long weekend); weekends and holidays repeat the last value |

### 7.4 `RateConfig` per asset (E-003 §3.1 fields)

Seconds for the durations, basis points for the bounds. `PoolConfig` and the cap belong to the
deploy ADR; the reference values test 4 uses are below the table.

| Field | USDY | USTRY | deJTRSY (when admitted) | CETES/MXNe (when admitted) |
|---|---|---|---|---|
| `max_age` | `108000` (30 h) | `108000` | `108000` | `108000` |
| `max_closed_age` | `108000` | `108000` | `360000` (100 h) | `108000` |
| `grace` | `7200` (2 h) | `7200` | `7200` | `7200` |
| `max_step` | `10` | `10` | `10` | `10` |
| `drift` | `Some { min_per_day: 0, max_per_day: 5, max_jump: 5 }` | same | same | same |
| `hours` | `None` | `None` | `Some(MarketHours)`: `utc_offset_minutes -300` (`-240` in summer, changed by a human), `open_minute 0`, `close_minute 1440`, `days` Monday to Friday, `holidays` the SIFMA bond-market holidays of the year, read from `sifma.org` when the entry is written and recorded with the date | `None`, to be confirmed against the feed that then exists |
| `sources` | `2` | `2` | `2` | `2` |
| `cross_check` | `Some(5)` | `Some(5)` | `Some(10)` | `Some(5)` |

USTRY takes the daily-NAV shape and not market hours because its price accrues every calendar
day and never reprices at an opening. deJTRSY takes market hours because its value is struck on
business days and published a business day later: the pool refuses trades from Monday's
opening until both sources have published after it, and `cross_check` equals the band because
the two sources are a business day apart by construction. `cross_check` of 5 bp covers a
RedStone write up to 12 hours old against a fresh issuer read, which differ by under one basis
point.

Reference `PoolConfig` for test 4 and for the testnet pools of A-002 (the deploy ADR fixes the
mainnet values): band pools `width 10`, `fee 5`, `bump_per_day 5`, `bump_max 5` (a second,
wide pool `width 20`, `fee 10` where wanted); stable pools `amp 200`, `fee 1` and `fee 5`;
cap `10^22` (10 000 tokens) per pool at the start of the beta.

Amended 2026-10-20 (ADR-014 §1 to §3, accepted that day, `26b3504`): the mainnet values
are fixed. USDY and USTRY take the columns of the table above as they stand (`max_age
108000`, `max_closed_age 108000`, `grace 7200`, `max_step 10`, drift `0 ..= 5` with
`max_jump 5`, `hours None`, `sources 2`, `cross_check Some(5)`), and the observe week
proposed no narrower band (ADR-014 §3 point 9). `PoolConfig` of both band pools is `width
10`, `fee 5`, `bump_per_day 5`, `bump_max 5`, with no wide second pool; the stable pool
PYUSD/USDC is `amp 200`, `fee 5`, with no `fee 1` pool in the beta; the cap is `10^22` per
pool. The reference values above stay the reference for test 4 and the testnet pools; the
deJTRSY and CETES columns wait for their own ADRs.

### 7.5 Reflector: the question and the reading

The question (ADR-007 point 9): whether PYUSD, USDT and MXN are in Reflector's external and
fiat contracts, so that one could serve as a watch source for a stable pool; a Reflector feed
is excluded as a rate source whatever the answer. Read read-only on 2026-09-12 by `assets()`
on the three mainnet contracts, each reporting 14 decimals:

| Contract | What it carries | Read 2026-09-12 |
|---|---|---|
| `CALI2BYU2JE6WVRUFYTS6MSBNEHGJ35P4AVCZYF3B6QOE3QKOB2PLE6M` | Stellar DEX trades: 51 Stellar assets | USTRY, CETES, PYUSD and TESOURO are in it; USDY, deJTRSY and USDT0 are not |
| `CAFJZQWSED6YAWZU3GWRTOCNPPCGBN32L7QV43XX5LZLFTK6JLN34DLN` | external CEX and DEX candles (Binance, Bybit, Coinbase, Gate, Kraken, OKX), by ticker | BTC, ETH, USDT, XRP, XLM and ten more; PYUSD is not in it |
| `CBKGPWGKSKZF52CFHMTRR23TBWTPMRDIYZ4O2P5VS65BMHYH4DXMCJZC` | fiat exchange rates | EUR, GBP, JPY, MXN, XAU and nineteen more |

The answer, recorded here as the reading it is: no Reflector contract is a rate source, on the
first ground alone, because every Reflector node fetches and aggregates trades (the Stellar DEX
contract is the feed class of the YieldBlox incident of 2026-02-22, when the USTRY feed was
moved from 1.06 to 106.7 by a 0.05 USTRY trade). As watch sources for the indexer, which never
feed a pool: USDT/USD exists in the external contract, MXN in the fiat contract, PYUSD in
neither but the DEX one. Whether the indexer uses the first two is that spec's decision. No
Reflector adapter is built.

### 7.6 Provenance, restated

Every id in `chain-config/` carries, in a comment, the external source of this appendix it was
checked against and the date. An id whose row above says "pending" is not written until the
check is made. `symbol()` and `name()` of a contract, RedStone's mapping, a token list, an
explorer page or `keeper/README.md` are never that source. The security reviewer checks every
entry against this appendix (§6).

### 7.7 Measurements (first data point, 2026-09-13)

The observe-only mode against mainnet, one reading per asset, the issuer's price against
RedStone's feed at the same minute:

| Asset | Difference | Note |
|---|---|---|
| USDY | 0 bp | Ondo's Ethereum oracle against `USDY_FUNDAMENTAL/USD` |
| USTRY | 1 bp | Etherfuse's API against `USTRY` |
| deJTRSY | 2 bp | Centrifuge's event against `deJTRSY_FUNDAMENTAL/USD`; the research had measured 13 bp on 2026-09-12 |

All three are inside the `cross_check` of §7.4. The deJTRSY reading is the first data point of
the waiting condition of ADR-007 point 2 (the two sources within 10 bp on every business day of
one month, with the Monday delay of §3.4 recorded); the week of §6 for USDY and USTRY and the
month for deJTRSY are the owner's to run, and their CSV stays outside the repository.

### 7.8 Measurements (the observe-only week, 2026-09-16 to 2026-09-23)

The keeper in observe-only mode against mainnet, without a key, one cycle every 15 minutes from
2026-09-16 20:30:37 UTC to 2026-09-23 20:38:24 UTC (7 days, 664 cycles). Each reading is the
issuer's price against RedStone's feed in the same cycle, as in §7.7.

| | USDY | USTRY | deJTRSY |
|---|---|---|---|
| Readings | 663 of 664 | 664 of 664 | 656 of 664 |
| Difference, maximum | 1 bp | 2 bp | 4 bp |
| Difference, median | 0 bp | 1 bp | 2 bp |
| Readings above 5 / 10 / 25 bp | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 |
| Feed age, maximum | 11.96 h | 11.97 h | 11.96 h |
| Issuer age, maximum | 0.01 h | 0.00 h | 80.82 h (weekend), 29.52 h inside market hours |

- The condition of §6 is met: USDY and USTRY stayed within their `cross_check` of 5 bp (§7.4)
  in every reading of the week, with a measured maximum of 1 bp and 2 bp.
- Every RedStone feed wrote on its 12-hour heartbeat only; no feed value was older than 12 hours
  against `max_age` 30 h.
- USDY's issuer price stood still for at most 24.1 hours (one step per calendar day), so under
  the post rule of §3.4 the issuer adapter's value is at most about a day old: no heartbeat post
  is needed.
- Missing readings: USDY one (a single cycle, far inside `grace`); deJTRSY eight. All nine were
  "the issuer could not be read: transport: unreachable" on the public Ethereum JSON-RPC
  endpoint. The keeper returned an error and never a guess, as §3.4 requires.
- deJTRSY, the first week of its month (ADR-007 point 2, to at least 2026-10-16): all readings
  within 4 bp against the band of 10 bp; the issuer stamped with Centrifuge's publication time
  on Ethereum; the issuer's age inside market hours came within 29 and 50 minutes of `max_age`
  on two business days; the Monday delay after the opening of 2026-09-21 (04:00 UTC) was
  11 939 s for the issuer and 41 720 s for the feed.
- The delay columns of the CSV held the newest publication since the last opening until
  2026-09-23 10:37 UTC; since the keeper release of that day they hold the first publication of
  each source after the Monday opening, as §3.4 asks (11 939 s and 41 720 s for 2026-09-21).

### 7.9 Endpoints per chain on mainnet (2026-10-01, ADR-014 §6)

Chosen by the owner on 2026-10-01 from a comparison of providers kept in the brain (the
prices, the free tiers and the probe to run; the table below is its result). The first endpoint of each chain is the
public one the keeper reads today; the second is a keyed one on an account of the owner's,
whose URL lives in the keeper's secret file (§3.4) and nowhere in a repository. The same two
lists go into the SDK's and the quote API's `endpoints/mainnet.toml` (A-005 §7.2), the
indexer's `config/mainnet.toml` (A-003 §7.2) and the configuration of I-003 §5,
each by its own small pull request after the keeper's.

| Chain | First endpoint (committed) | Second endpoint (keyed, secret file) | Free tier as read 2026-10-01 |
|---|---|---|---|
| Stellar, Soroban RPC | `https://mainnet.sorobanrpc.com` | Validation Cloud, Stellar RPC on mainnet | 50 M compute units a month, every RPC method 10, 60 days of RPC history |
| Ethereum, JSON-RPC | `https://rpc.mevblocker.io` | Alchemy, Ethereum mainnet | 30 M compute units a month, 300 a second, `eth_call` 26, `eth_getLogs` 60 |

Reserves, if a chosen provider fails its probe: Gateway.fm's public mainnet endpoint on
Stellar, Infura on Ethereum. `endpoint_timeout_seconds` is `10` by reference until the probe's
latencies set it. `eth_getlogs_block_span` is `2000` on Ethereum, on testnet and mainnet
alike (§3.4, 2026-10-05). Before the first mainnet pool the keeper's mainnet journal shows one
reading from each second endpoint (ADR-014 §6 point 25).
