# E-007: Pool contract for Soroban

Date: 2026-09-10
Status: done; amended 2026-09-27 by I-003 §3.5 (the sentinel role: `Roles.sentinel`, `pause(caller)`,
`set_roles` with a third argument, `VERSION` 6, tests 1 and 8), to be built as one small pull
request under the review of the exception list; the upgrade rule of §3.2 and test 13 added
the same day from that pull request's review; amended 2026-09-28 by ADR-012 from the exploit
pass of I-001 (the upgrade delay with `announce_upgrade` and `cancel_upgrade`, `DuplicateSource`,
the `init` event's rate data and the `sources()` view, the write-down prose corrected to its
formula, §3.1, §3.2, §3.5, §3.6, tests 1, 11, 12 and 13), built as one pull request, tests
first, as `VERSION` 7, a new deployment (core pull request 24, merged 2026-09-28); amended
2026-09-28 by the amendment of ADR-012 (points 9 to 14) from the review of that pull request
(the governance rate path: `announce_rate`, `set_rate()` and `cancel_rate`; `announce_roles`,
`set_roles()` and `cancel_roles`; `announce_cap`, `set_cap()`, `cancel_cap` and `lower_cap`;
`clear_sentinel`, immediate, point 15; §3.1, §3.2, §3.5, §3.6, tests 7, 8, 11 and 17, the
baseline rule of §4.16.1), to be built
as one pull request directly after version 7, tests first, as `VERSION` 8, a new deployment;
testnet is redeployed once, on version 8, never on 7; test 17 corrected 2026-09-29 from the
review of that pull request (governance may `resume`, as §3.2 and test 9 say), and from the
same review `clear_sentinel` also clears a standing roles announcement that names a
sentinel (§3.1, §3.2, §3.5, test 8), built in the same version 8 pull request, tests first;
version 8 built as core pull request 27, merged 2026-09-30, the optimised pool Wasm `106 019`
bytes (`16 861` under the bound of §3.7), with the event order and the sentinel check of
`clear_sentinel` stated as built (§3.2, §3.5, test 8); its baseline table of §4.16.1 stands
since 2026-10-01, from the `wasm` job of the merge run `36690431683` (E-009 §3.5); queued 2026-09-30 for `VERSION`
9, from the SDK's reading of the version 8 events (A-005 §3.3): the `roles` and `cap` events
carry a field `announced` (`true` when the row applies a standing announcement, from
`set_roles()` and `set_cap()`; `false` from `clear_sentinel` and `lower_cap`, which apply
none), so that a reader can tell an immediate change from the execution of an announcement
without comparing values (§3.5, test 8), to be bundled with the next change that needs a
new deployment, never a deployment of its own; amended 2026-10-12 by ADR-016 as the proposed
`VERSION` 9, §3.8: the cap curve (`CapCurve`, `cap_curve()`, `AboveCurve`), `pause(caller,
rule)` with the rule in the event, `mark_incident()`, the halving, the `incident` and
`cap_curve` events, the view `announcements()` of I-003 §3.1 point 9, bundled with the
`announced` flag above; one pull request in core, tests first (tests 7, 18, 19 and 20), a new
deployment and a new baseline of §4.16.1, after A-010 is done and watchtower pull request 7 is
merged and before watchtower pull request 8, beside E-017 steps 1 and 2; "proposed" until core
starts it, and the status of this spec stays "done" for versions 5 to 8; amended 2026-10-12
(A-010 step 3, ADR-015 §4): this spec is copied into the public `contracts` and into core,
so its text names no folder of the brain; the two references to the brain's cap record (§3.2)
and deploy records (§3.6) are rewritten as what they say, nothing in the rules changed; the
watchtower is still named, as the operator of the public `sentinel` role (A-010 §7.2);
stated 2026-10-12 as built, from a question of I-003 rule 17, no change in the code: a
`withdraw` during the delay of a write-down announcement carries the scaled payout in its
event with `Scaled { reserve, balance }`, while the reserve falls by the unscaled one (§3.2,
§3.5, test 12); amended 2026-10-20 (ADR-014 accepted, `26b3504`): §3.4 `MAX_PROTOCOL_SHARE`
confirmed at `2 500` as compiled
Target: core, crate `pool-soroban` (seventh component in the build order of ADR-002; the
chain-specific crate that E-000 rule 2 allows, named for its chain)
Depends on: E-004 (`band`), E-005 (`stable-pool`), E-003 (`rate`), E-001 (`math`), all as merged in
core, and the Soroban SDK pinned in core (`soroban-sdk 27.0.6`). E-006 (`router`) is not a
dependency: the router contract is its own spec (§5).

## 1. Goal

One Soroban contract that turns the two pool engines of the labs into pools on a chain: it holds
the tokens, stores the state the engines return, and exposes swap, deposit and withdraw to
traders and LPs, the rate functions of E-003 to the world and to the guardian, and the switches
of ADR-001 and ADR-002 (cap, pause, protocol fee) to the roles that may touch them. Every number
the contract stores comes from `band`, `stable-pool` or `rate`; the contract computes nothing
about prices itself. One Wasm serves both pool kinds; each deployed instance is one pool of one
kind, created by a human (ADR-002 point 6).

Contract instance means: one deployed copy of the Wasm with its own storage and address; one
pool. Roles mean: the two addresses that may change a pool's switches, governance (the brain's
key) and the guardian (the pause key). Adapter means: the brand's small contract that reads one
rate source and hands the pool an observation in the shape E-003 expects.

## 2. Context and constraints

- The engines are pure and chain-agnostic; E-004 §3.4, E-005 §3.4 and E-003 §3.4 list what the
  contract around them must do, and ADR-005 point 4 says that is one spec for both pool kinds.
  This is that spec. It adds no arithmetic: amounts move from token units to the engines'
  18-decimal units and back with the scale functions of E-005 §3.2 (exact in, floor out, ceil for
  a computed input), and the dust of that conversion stays in the contract's balance outside the
  state.
- Reserves are state, not balances (E-004 §3.3, E-005 §2). The contract therefore checks, on
  every transfer in, that its balance grew by exactly the amount it recorded; a token that takes a
  fee on transfer or rebases shows up as a mismatch at the moment it would do harm, and the call
  is rejected (brief §4.1 lists both as unsupported by design). Same-asset pools are refused at
  creation.
- Soroban gives the contract what it needs and no more: `env.ledger().timestamp()` is the chain
  time E-003 calls `now`; `env.ledger().network_id()` is the hash of the network passphrase, so
  the contract knows whether it runs on mainnet and can enforce the two-source rule of E-003 §3.3
  in code; `require_auth` on an address authorises a call and every sub-call the signer approved,
  so a router contract (E-006 §3.4) can call a pool on the trader's behalf; contracts cannot
  re-enter each other, so there is no reentrancy path through a token; a contract can replace its
  own Wasm, which is how a bug is fixed during the capped beta.
- Two keys, no more. Governance is the brain's key: it sets roles, caps, fee switch, rate config,
  sources, resumes a rate, collects protocol fees, writes down a reserve after a clawback, and
  upgrades. The guardian is the pause key: it pauses a pool and suspends a rate, and it may also
  resume both, because the guardian is the one watching. Neither key ever moves LP funds:
  withdrawals belong to LPs and nothing in this contract can send a token anywhere but to the
  caller of a swap or withdrawal, or protocol fees to an address governance names. Who holds the
  keys is §3.6 and an ADR before the first deploy (brief §14).
- The capped beta of ADR-002 is enforced here: the cap on deposits, the pause that never blocks
  withdrawals, the protocol fee switch that is off and bounded. The "unaudited" warning is the
  brand's, in every interface and API response; the contract exposes a `version` so that the
  interface can show what is deployed.

## 3. What gets built

### 3.1 Storage

Instance storage (lives and expires with the contract; its TTL is extended by every
state-changing call and by `restore_shares`, never by a view, §3.2):

| Key | Value | Set by |
|---|---|---|
| `Kind` | `Band` or `Stable` | `init_*`, never changed |
| `Tokens` | band: `{ asset: Address, quote: Address }`; stable: `{ a: Address, b: Address }` | `init_*` |
| `Scales` | `{ first: Scale, second: Scale }` (E-005), from each token's `decimals()` at init; `decimals > 18` is refused | `init_*` |
| `Roles` | `{ governance: Address, guardian: Address, sentinel: Option<Address> }` (the third since 2026-09-27, I-003 §3.5; `None` on a pool without a watchtower) | `init_*`, `set_roles` |
| `Config` | `band::PoolConfig` or `stable_pool::PoolConfig` | `init_*`; the stable `cap` through `set_cap` |
| `State` | `band::PoolState` or `stable_pool::PoolState` | the engines, through every swap, deposit, withdraw, write-down |
| `Cap` | `u128`, normalised; band pools only (the stable engine carries its own) | `init_band`, `set_cap` |
| `Paused` | `bool` | `pause`, `unpause` |
| `RateConfig` | E-003 type; band pools only | the constructor, `set_rate` |
| `RateState` | E-003 type; band pools only | the constructor, `refresh`, `suspend`, `resume`; nothing else |
| `Sources` | `Vec<Address>` of adapters, length `rate_config.sources`; band pools only | the constructor, `set_rate` |
| `ProtocolShare` | `Bps`, `0 ..= MAX_PROTOCOL_SHARE` | `set_protocol_share` |
| `ProtocolOwed` | `{ first: u128, second: u128 }`, normalised, fees taken out of the reserves and not yet collected | swaps, `collect_protocol_fees` |
| (not stored) `Version` | `u32`, a constant of the Wasm returned by `version()`; it is not a storage entry, so an upgrade changes it by construction. It is raised by every change to the Wasm's interface: an entry point, an argument, a return type, an error code, an event's topics or its data fields, because a wallet or an indexer that decodes events depends on all of them; the owed amounts added to the write-down events on 2026-09-12 make it `4`; the literal byte types in the exported signatures of 2026-09-13 (§3.7) make it `5`; the sentinel of 2026-09-27 (a constructor argument, `pause`'s argument, `set_roles`' third argument, the `roles()` type and the `init` and `pause` events, I-003 §3.5) makes it `6`; the upgrade delay (`announce_upgrade`, `cancel_upgrade`, `upgrade` without an argument, two events), the `init` event's rate data, `DuplicateSource` and the `sources()` view of 2026-09-28 (ADR-012) make it `7`; the announcement of a rate change, a roles change and a cap raise of the same day (three instance keys, six events, `set_rate`, `set_roles` and `set_cap` without arguments, `announce_rate`, `announce_roles`, `announce_cap`, `cancel_rate`, `cancel_roles`, `cancel_cap`, `lower_cap` and `clear_sentinel`; ADR-012 amended) make it `8` | the Wasm itself |
| `WriteDownAt(token)` | `{ at: u64, reserve: u128, balance: u128 }`, a standing write-down announcement; absent when none | `announce_write_down`, cleared by `write_down` and by `cancel_write_down`; never by `unpause` |
| `UpgradeAt` | `{ at: u64, wasm_hash: BytesN<32> }`, a standing upgrade announcement (since version 7, ADR-012); absent when none | `announce_upgrade`, cleared by `upgrade` and by `cancel_upgrade` |
| `RateAt` | `{ at: u64, config: RateConfig, sources: Vec<Address> }`, a standing rate announcement (since version 8, ADR-012 amended); band pools only; absent when none | `announce_rate`, cleared by `set_rate` and by `cancel_rate` |
| `RolesAt` | `{ at: u64, governance: Address, guardian: Address, sentinel: Option<Address> }`, a standing roles announcement (since version 8); absent when none | `announce_roles`, cleared by `set_roles`, by `cancel_roles`, and by `clear_sentinel` when the announcement names a sentinel (2026-09-29) |
| `CapAt` | `{ at: u64, cap: u128 }`, a standing cap raise (since version 8); absent when none | `announce_cap`, cleared by `set_cap` and by `cancel_cap`; never by `lower_cap` |
| `CapCurve` | `{ base: u128, since: u64 }`, normalised units and ledger time, the cap curve of ADR-016 (version 9, proposed, §3.8); absent on a pool upgraded from version 8 until its first read | the constructor; the halving of §3.8; the one-time backfill of §3.8 on a pool upgraded from version 8 |

Persistent storage (per entry, extended when touched):

| Key | Value |
|---|---|
| `Shares(Address)` | `u128`, the LP's shares; absent means `0` |

The share ledger is internal: shares are not a token and cannot be transferred (§5). TTL: the
instance is extended to `INSTANCE_TTL` ledgers on every call when below `INSTANCE_TTL_THRESHOLD`;
a `Shares` entry is extended to `SHARES_TTL` on every deposit and withdrawal of that address.
Reference values `INSTANCE_TTL = 30 days`, `SHARES_TTL = 120 days` in ledgers at five seconds,
thresholds at half; an LP whose entry expired restores it by `restore_shares`, which any address
may call for any LP (it only extends TTL of an archived entry and moves nothing). The brand's
keeper extends the instance TTL of every pool weekly so that a quiet pool never expires, and
with it the `ContractCode` entry of the Wasm the pool runs on (amended 2026-09-13): an expired
code entry fails every call whatever the instance TTL says. Corrected 2026-09-28: every entry
point that goes through `Pool::load` calls `instance().extend_ttl`, and the host extends the
contract's code entry together with its instance; an extension counts as a read, so the
footprint of the call is unchanged. The code entry is therefore extended only by calls, and a
quiet pool's code entry still expires: the keeper's weekly extension stays. The brand's
deploy script checks the code TTL before the first mainnet pool.

### 3.2 Entry points

Amounts at the boundary are `i128` in token units; inside they are the engines' normalised units.
"caller" is the address that authorises the call (`require_auth`). Every state-changing entry
point re-reads `Config` and `State`, passes them to the engine, and stores exactly the state the
engine returned, minus the protocol part of a fee (§3.4); it never edits a field itself.

Creation (once, by the deployer's script, a human signs). The contract has a Soroban constructor
(`__constructor`), which runs in the same transaction as the deployment with the deployer's
authorisation, so that a pool never exists uninitialised and nobody can initialise it in between;
there is no separate `init` entry point, and no call can ever find a pool without its storage.
The constructor takes one argument, an enum whose variant is the pool kind and whose payload
the arguments of that kind, so that kind and arguments cannot disagree and the deploy script
builds exactly one value:
- band: `(governance, guardian, sentinel: Option<Address>, asset, quote, config: band::PoolConfig,
  cap: u128, rate_config: RateConfig, genesis_rate: Rate, sources: Vec<Address>)` (`sentinel`
  since 2026-09-27, I-003 §3.5): refuses
  `asset == quote`, a token whose `decimals() > 18`, `sources.len() != rate_config.sources`,
  an address that appears twice in `sources` (`DuplicateSource`, since version 7, ADR-012: one
  adapter listed twice is one source, and the two-source rule is checked pairwise, not by
  count), and, when `env.ledger().network_id()` equals the Stellar public network id,
  `rate_config.sources < 2` (E-003 §3.3: two sources are a condition for mainnet; the network id
  constant is the SHA-256 of `Public Global Stellar Network ; September 2015`, written in the
  source with that derivation). Stores everything, `State = PoolState::empty()`,
  `RateState = RateState::genesis(genesis_rate, now)`, `Paused = false`, `ProtocolShare = 0`.
  Emits `init` with the rate config and the sources in its data (since version 7, ADR-012, so
  that an indexer that reads only events knows the adapters from the first ledger). A refusal
  fails the deployment transaction, so a wrongly created pool never exists.
- stable: `(governance, guardian, sentinel: Option<Address>, a, b, config: stable_pool::PoolConfig)`:
  the same checks where they apply; the cap is `config.cap`.
- Neither constructor requires the authorisation of the addresses it stores as roles; the
  deployment carries the deployer's authorisation alone (accepted 2026-09-28, ADR-012, from the
  exploit pass). A pool anyone deploys from the published Wasm naming the brand's governance,
  guardian and tokens is not a brand pool: nothing on chain marks one, the brand's list in
  `chain-config/` is the registry (§5), and the brand's deploy script reads the roles back
  after every deployment (A-002).

Trading and providing (permissionless):
- `swap_exact_in(from: Address, direction, amount_in: i128, min_out: i128, to: Address) -> i128`:
  `from.require_auth()`; `Paused` gives `Paused`; the amount is converted exactly; the engine's
  `swap_exact_in` runs (band: with `now`, the side from `direction`, `RateConfig`, `RateState`;
  stable: with `direction`); on `Err` the call fails with the mapped error (§3.5); the protocol
  part of the fee is split off (§3.4); the output is converted with a floor and must be at least
  `min_out`, else `SlippageExceeded`; then the token in is transferred from `from` to the
  contract with the balance check of §3.3, the token out from the contract to `to`, the state
  stored, `swap` emitted, and the token-unit output returned. `direction` is `AssetIn` /
  `QuoteIn` for a band pool and `AtoB` / `BtoA` for a stable pool; the wrong kind is
  `WrongKind`.
- `swap_exact_out(from, direction, amount_out: i128, max_in: i128, to) -> i128`: the mirror,
  with the input converted with a ceiling, at most `max_in`, and returned.
- `deposit(from: Address, first_max: i128, second_max: i128, min_shares: i128) -> i128`:
  `from.require_auth()`; `Paused` gives `Paused`; the engine's `deposit` runs (band: with `now`
  and the rate read on `PoolBuysAsset` inside the engine; stable: as is); for a band pool the
  contract then checks the cap: `asset' · low + quote' <= Cap` in normalised units with
  `wad_mul_floor` and the rate the engine used, else `CapExceeded` (the stable engine checks its
  own); the amounts taken are converted with a ceiling and transferred in with the balance
  check; shares minted must be at least `min_shares`; `Shares(from)` grows; `deposit` emitted;
  shares returned. The genesis deposit is the creator's and follows `init_*` in the same script.
- `withdraw(from: Address, shares: i128, min_first: i128, min_second: i128, to: Address)
  -> (i128, i128)`: `from.require_auth()`; no pause check, no rate, no cap; `Shares(from)` must
  cover `shares`; the engine's `withdraw` runs; while a write-down announcement stands for a
  token, that token's payout is first scaled with the announced factor (§3.2, governance); payouts
  are converted with a floor and must be at least the minima; transferred to `to`; `withdraw`
  emitted, with the factor's numbers when one applied. Stated 2026-10-12 as built (test 12):
  the reserve in the engine's state falls by the unscaled payout, the engine's own, while the
  LP receives and the event carries the scaled one, `mul_div_floor(payout, balance, reserve)`
  with the announced numbers; after the execution of `write_down` the announcement is gone
  and a withdrawal is unscaled again. This entry point has no way to fail
  on anything but the caller's own inputs and the token transfer, and it is the only one that
  keeps working while paused, on a stale, suspended or lost rate, and on a pool outside its
  engine's domain.
- `refresh(caller: Address) -> RateStatusChange` (band pools; anyone: `caller.require_auth()`
  only names who paid for the call in the event, it grants nothing and no role is checked; a
  keeper that calls through a contract of its own passes that contract as the caller, and the
  event names the contract): for
  every source `i`, calls the adapter's
  `observe()`; a `None` or a call that fails is skipped (a dead adapter is a dead source, E-003
  handles that by liveness); an observation is built with `Observation::from_sep40(price,
  decimals, timestamp, i)` and applied with `rate::update` at `now`; `NotNewer` is ignored, every
  other error is skipped for that source and reported in the event; the resulting `RateState` is
  stored. Emits `rate` with the status before and after when they differ, and `refresh` with
  the per-source outcome always. Never fails on a source; fails only on a stable pool
  (`WrongKind`).

Views (read-only, free of auth, and they write nothing: a view does not extend any TTL, which
only state-changing calls and `restore_shares` do):
- `quote_exact_in(direction, amount_in) -> i128` and `quote_exact_out(direction, amount_out)
  -> i128`: the engine on a copy, with the protocol split applied, no transfer; the numbers a
  swap would return now. `state()`, `config()`, `rate_state()`, `rate_config()`, `sources()`
  (the adapters of a band pool, `Vec<Address>`; since version 7, ADR-012; `WrongKind` on a
  stable pool, as `rate_config()`), `roles()`,
  `cap()`, `paused()`, `protocol_share()`, `protocol_owed()`, `shares(address)`,
  `total_shares()`, `tokens()`, `kind()`, `version()`.

Reserves and balances, stated for every reader of `state()` (amended 2026-09-19, from the
indexer's work on A-003 §3.5): a reserve in `State` is in normalised units and holds dust by
design, because every rounding at the boundary falls in the pool's favour (E-001 §3.3: what
the pool pays out rounds down, what it takes in rounds up) and the difference stays in the
state. A reserve is therefore not a whole number of token units, and the contract's token
balance is at or above the reserve floored to token units: `balance(token) >=
floor_to_units(reserve)` is the invariant, the balance check of §3.3 keeps it on every
transfer in, and `collect_protocol_fees` and the write-down of this section never break it.
A remainder under one token unit is not an error, a flag or a loss; a floored reserve above
the balance would be a loss, and is the one condition a reader reports (A-003 §3.8 rule
thirteen).

Guardian (`guardian.require_auth()`, except `pause`, which takes its caller):
- `pause(caller: Address)` (amended 2026-09-27, I-003 §3.5): `caller.require_auth()`, then `caller`
  must equal the guardian or the sentinel of `Roles`, else `Unauthorised`; a sentinel of `None`
  matches nothing. `unpause()`: the guardian only. Both set `Paused`; for a stable pool also
  the engine's flag through its `pause`/`unpause`, so that the stored state says the same.
  Idempotent. Emit `pause` / `unpause`. The auth check comes before the role check, so a call
  without the caller's signature never reaches `Unauthorised` (§3.5). Version 9 (proposed,
  ADR-016, §3.8): `pause(caller: Address, rule: u32)`, the rule number of I-003 the sentinel
  pauses on, `0` from the guardian; a sentinel pause on an invariant halves the cap.
- `suspend()` (band): `rate::suspend` on the stored state; emits `rate`.
- `resume(rate: Rate)` (band; guardian or governance): `rate::resume` at `now`; emits `rate`.
  E-003 §3.2: with two or more sources the pool serves nothing until two sources have published
  and agreed; the guardian calls `refresh` after.

Governance (`governance.require_auth()`). Since version 8 (ADR-012 amended, from the review
of core pull request 24) a change of the roles, of the rate path or a raise of the cap is
announced first and executed after `UPGRADE_DELAY`, by the pattern of `announce_upgrade`,
`upgrade()` and `cancel_upgrade()` below: the announcement, by governance, is validated exactly
as the direct call was, stores the announced values and `now + UPGRADE_DELAY` under its own
instance key (§3.1) and emits `<name>_announced`; a second announcement of the same kind
replaces the first and restarts the delay; the execution, by governance, takes no argument,
is refused before the announced time (`TooEarly`) and without an announcement
(`NotAnnounced`), applies the announced values, clears the announcement and emits the event
the direct call emitted; `cancel_<name>()`, by the guardian or governance, clears a standing
announcement and emits `<name>_cancelled` with the caller, and is `NotAnnounced` without one.
The finding behind it: with the direct calls, the governance key alone forced any rate on a
band pool in three ledgers (`set_roles` naming itself guardian, `suspend` and `set_rate` with
its own adapters, `resume`), and the 24-hour warning of the write-down and the upgrade
covered the code and the reserve but not the price. Which calls carry the delay is decided
call by call below; `set_protocol_share` and `collect_protocol_fees` stay immediate.
- `announce_roles(governance, guardian, sentinel: Option<Address>)`, `set_roles()` and
  `cancel_roles()` (the argument-less `set_roles` since version 8; before it `set_roles` took
  the three arguments and applied them at once): all three roles at once (the third since
  2026-09-27, I-003 §3.5; `None` removes the sentinel); `set_roles()` emits `roles` and is
  signed by the governance that stands at the execution. The guardian's `cancel_roles()` also
  holds an announcement that would replace the guardian: that veto is the point (§3.6).
  `clear_sentinel()`, immediate (since version 8, ADR-012 point 15): governance sets
  `Roles.sentinel` to `None` at once and emits `roles`; it is the mirror of `lower_cap`, a
  power taken away and none given, so that a stolen sentinel key stops trading for the
  minutes governance needs to sign and not for the day of the roles delay. Naming or
  replacing a sentinel goes through `announce_roles` and `set_roles()` like every other roles
  change. The guardian may call it as well as governance (ADR-012 point 15: it takes a
  power away and gives none, and the guardian is the incident key at hand when a sentinel
  misbehaves); it is the one roles entry point the guardian has. The sentinel cannot clear
  itself (`Unauthorised`). Added 2026-09-29 from the review of the version 8 pull request:
  when a `RolesAt` stands whose `sentinel` is `Some`, `clear_sentinel` clears that
  announcement as well and emits `roles_cancelled` with the caller, beside `roles`. As
  built (core pull request 27): `clear_sentinel` reads the `sentinel` field of the standing
  `RolesAt` and removes the whole announcement only when that field is `Some`; the events
  come in the order `roles`, then `roles_cancelled`.
  Without it, a stolen key that governance had already announced as the next sentinel
  would be named after the delay by the `set_roles()` that was meant to name it, and the
  clearing would have bought minutes and lost the day; a caller who clears the sentinel
  means no sentinel until a fresh announcement. A standing announcement with `sentinel:
  None` is left as it is, because it names no key that `clear_sentinel` takes away, and the
  guardian's `cancel_roles` remains the way to stop it.
- `lower_cap(cap)`, immediate (since version 8; before it `set_cap(cap)` set any cap at once):
  band `Cap` or stable `config.cap` (through a new `PoolConfig` the engine validates), refused
  with `OutOfRange` when `cap` is above the current one; emits `cap`. `announce_cap(cap)`,
  refused with `OutOfRange` when `cap` is not above the current one, `set_cap()` after the
  delay and `cancel_cap()`: a raise exposes more of the LPs' funds and gets the warning, a
  lowering takes exposure away and needs none. Every cap change is an ADR (ADR-002 point 3).
  Version 9 (proposed, ADR-016, §3.8): `announce_cap` and `set_cap()` also refuse a cap above
  the curve with `AboveCurve`, and a step below ADR-016's threshold 1 is a line in the
  brain's cap record (`caps.md`), not an ADR (ADR-002 point 3 as amended).
- `set_protocol_share(share: Bps)`: `0 ..= MAX_PROTOCOL_SHARE`, else `OutOfRange`; emits
  `protocol_share`. Off means `0`. Immediate: the share is bounded and takes from fees, never
  from LP capital.
- `collect_protocol_fees(to: Address) -> (i128, i128)`: transfers `ProtocolOwed` of both tokens
  (floored to token units, the dust stays owed) to `to`, sets the owed amounts to the dust,
  emits `collect`. During the beta `to` is the reserve address of ADR-002 point 2. Immediate:
  it moves only what is owed.
- `announce_rate(config: RateConfig, sources: Vec<Address>)`, `set_rate()` and `cancel_rate()`
  (band; the announcement since version 8, before it `set_rate` took both arguments and
  applied them at once): one announcement for both, because each alone would have to match
  the other and the source count could never change. The announcement validates: the engine
  validates the config (`BadConfig` otherwise); `sources.len()` must equal `config.sources`,
  an address that appears twice is `DuplicateSource` (since version 7, ADR-012), and the
  mainnet rule applies again; it needs no suspension. `set_rate()` emits `rate_config` and
  `sources`. It is accepted only while the rate is `Suspended` (`NotSuspended` otherwise),
  checked at the execution and not at the announcement, so that a legitimate source change
  suspends the pool for the minutes of the execution and not for the day of the warning; and
  it does not touch `RateState`: the suspension stays, and the `resume` that follows (guardian
  or governance, with its own `rate` event) is what clears `last_by_source` through
  `rate::resume`, so that observations of the old source set never confirm the new one (E-003
  §3.2). `suspend` and `resume` stay immediate: they are the guardian's brake, and on a
  mainnet pool a `resume` serves no rate until two sources have agreed. No entry point but
  `refresh`, `suspend` and `resume` changes the rate status, and each of those emits `rate`
  when it does.
- `announce_write_down(token: Address)` and `write_down(token: Address)`: the repair after an
  issuer clawback or freeze took tokens out of the pool (brief §12), in two steps behind a
  timelock. Four rules, all enforced in code: it only lowers a reserve, never raises one; it
  runs only on a pool the guardian has paused (`NotPaused` otherwise); the new reserve is the
  contract's balance of that token in normalised units, never below it and never above it, and
  the call is refused when the balance is not below the reserve (`NothingToWriteDown`); and it
  is announced before it is executed. The owed protocol fees stay whole: the reserve a
  write-down aims at is `balance - ProtocolOwed(token)`, not the balance, so a loss is carried
  by the LPs pro rata over every share and the owed amount is untouched; only when the balance
  is below `ProtocolOwed(token)` is the owed amount lowered to the balance, and the reserve
  becomes `0`. So `collect_protocol_fees` can never bring the balance below the reserve
  (corrected 2026-09-28, ADR-012: this formula was always the rule and the code's behaviour,
  and the sentence that stood here, that the protocol carries the first loss and the LPs lose
  nothing until it has lost everything it was owed, said the opposite and was wrong; the fees
  were taken out of the reserves at the swap, §3.4, and are the treasury's, not LP capital). The
  owed amount is lowered at the announcement, not at the execution, so that the factor and the
  owed amount are fixed at the same moment and a withdrawal during the delay sees the same
  numbers as one after the execution; a `cancel_write_down` does not restore a lowered owed
  amount (the protocol's loss was real when it was recorded). Every "balance" below means this
  net balance. `announce_write_down` records
  `WriteDownAt(token) = now + WRITE_DOWN_DELAY` (reference: 24 hours) and the reserve and
  balance seen at that moment, and emits `write_down_announced` with them; `write_down` is
  refused before that time (`TooEarly`) and without an announcement (`NotAnnounced`), and
  when executed it sets the reserve to the lower of the announced balance and the balance at
  execution (never higher than announced: a balance that rose during the delay stays outside
  the state as dust, and a balance that fell lowers the reserve further); when withdrawals
  during the delay have made the reserve equal to or below that number already, execution
  changes no reserve, and still clears the announcement and emits `write_down` with equal
  numbers, so that a standing announcement can always be closed; `NothingToWriteDown` belongs
  to the announcement, not to the execution. Execution clears the announcement, and emits `write_down` with the
  reserve before, the balance, and the announced numbers. The loss is then shared by every share
  instead of hitting the last LPs to withdraw. The announcement fixes the write-down factor at
  once: `balance / reserve` as seen at the announcement, stored as the two numbers. During the
  delay `withdraw` stays open and pays that token pro rata with the factor, `mul_div_floor(
  payout, announced_balance, announced_reserve)` on the engine's payout, so that an LP who
  leaves during the delay gets exactly what an LP who stays gets after the write-down, and the
  timelock is a warning without an advantage for whoever is fast. Should the balance fall
  further during the delay (a second clawback), execution uses the lower balance and the LPs
  who stayed carry that second loss alone; that is the one thing the factor cannot cover, and a
  second announcement covers it. In that case `write_down` itself applies the same floor
  once more as a safety net: when the balance at execution is below `ProtocolOwed(token)`, the
  owed amount is lowered to that balance and the reserve becomes `0`, exactly as the
  announcement does, so that after any execution the balance is at least the reserve plus the
  owed amount; `collect_protocol_fees` can never bring the balance below the reserve, and a
  stale owed amount never makes an announcement impossible, it only lowers the net balance the
  announcement works with. The delay is the time in which every LP can see what governance
  is about to do; the announcement is loud (the brand's interface shows it on the pool), and the
  ADR that deploys the first mainnet pool confirms `WRITE_DOWN_DELAY`. Only governance can
  withdraw an announcement, with `cancel_write_down(token)` (emits `write_down_cancelled`);
  the guardian's `unpause` never touches it and is refused while an announcement stands
  (`WriteDownPending`), so that a pool cannot trade on a reserve governance has declared
  wrong. The only entry point that edits a state field, and it can only lower a reserve.
- `announce_upgrade(wasm_hash: BytesN<32>)`, `upgrade()` and `cancel_upgrade()` (the delay
  since version 7, ADR-012, from the exploit pass: an undelayed upgrade made the write-down's
  delay a warning the contract could not keep, because the same key could replace the code in
  one transaction). `announce_upgrade` records `UpgradeAt = { at: now + UPGRADE_DELAY,
  wasm_hash }` (reference `UPGRADE_DELAY` = 24 hours, the value of `WRITE_DOWN_DELAY`; the
  ADR that deploys the first mainnet pool confirms both) and emits `upgrade_announced`; a
  second announcement while one stands replaces it and restarts the delay, which can only
  lengthen the warning. `upgrade()` takes no argument: it applies the announced hash, is
  refused before the announced time (`TooEarly`) and without an announcement (`NotAnnounced`),
  clears the announcement, replaces the contract's Wasm and emits `upgrade` with the hash
  applied. `cancel_upgrade()` clears a standing announcement and emits `upgrade_cancelled`;
  the guardian may call it as well as governance, the one governance call the guardian can
  veto (§3.6). No pause is required: the announcement is the warning, the brand's interface
  shows it on the pool as it shows a write-down announcement, and withdrawals run throughout.
  During the
  capped beta this is how a finding is fixed; the ADR that lifts the caps after the audit
  decides whether governance keeps this power or renounces it by setting governance to a burned
  address. Rule (added 2026-09-27, from the review of the sentinel pull request): an upgrade
  replaces the code and keeps the storage, so a new Wasm that changes the shape of a stored
  type (a field added to or removed from `Roles`, `RateConfig`, `WriteDownAt` or any other
  entry of §3.1) reads the old entries with the new type and fails on every entry point that
  touches them; only `version()` still answers. Such a pull request therefore carries, in the
  same pull request, a migration step that runs once inside `upgrade` (or a one-off entry
  point governance calls right after it) and rewrites every entry of the old shape into the
  new one, with a test that writes the old shape into storage, upgrades, and reads it back
  through the new type. Without that step and that test the change is not an upgrade: the
  pool is deployed anew from the new Wasm and the old one is paused and drained. Version 5 to
  version 6 (the third field of `Roles`) is a new deployment: no migration is written, there
  is no mainnet pool, and the testnet pools are redeployed (§3.7). Version 6 to version 7 (a
  new instance key, an error code, two events and the `init` data; ADR-012) is a new
  deployment as well, and so is version 7 to version 8 (three instance keys, six events and
  five entry points; ADR-012 amended): the three testnet pools are redeployed once, from the
  version 8 artefact and never from the version 7 one, by the runbook of A-002 §3.7, with the
  router (whose bytes import the pool's interface, E-009 §3.1), and their version 6 ids move
  to `[superseded]`.

### 3.3 Token transfers

- Tokens are moved with the Soroban token interface (`transfer(from, to, amount)`), which
  requires `from`'s authorisation; a trader's or LP's signature on a swap or deposit therefore
  covers the transfer, and a router's call on a trader's behalf covers it when the trader signed
  the router's call tree.
- On every transfer into the contract: `before = balance(contract)`, transfer, `after =
  balance(contract)`; `after - before` must equal the amount, else `TransferMismatch` and the
  call fails whole. On a transfer out no check is needed; a short balance makes the transfer
  itself fail.
- Token units are `i128`; a token with more than 18 decimals is refused at init and an amount
  above `2^100` in normalised units is refused by the engines. A negative amount at the boundary
  is `OutOfRange` before anything else.
- The contract never holds an allowance and never calls `approve`; it never calls a token in a
  way that could call back (there is no such path on Soroban).

### 3.4 Protocol fee switch (ADR-001 point 3)

- `ProtocolShare` is `0` at creation and can only be raised by governance up to
  `MAX_PROTOCOL_SHARE`, a constant of the Wasm. The value of the constant is fixed by the ADR that
  turns the switch on for the first time, before the mainnet beta; this spec proposes `2 500`
  (a quarter of the fee), and the testnet builds carry that number until the ADR confirms or
  changes it. Confirmed 2026-10-20 (ADR-014 §2 point 5, accepted that day, `26b3504`):
  `MAX_PROTOCOL_SHARE` is `2 500` as compiled into the version 8 artefact. The deploy ADR
  fixes the constant because it binds the bytes that hold it; the ADR that turns the switch
  on sets the share under it, and the share is `0` at creation (ADR-014 §2 point 4). Both
  readings of "the ADR" above are right about their part.
- On every swap the engine returns a state with the whole fee inside the reserve of the token
  in; the contract takes `protocol = floor(fee · ProtocolShare / 10 000)` (E-001 `fee_floor`,
  rounding for the LPs) out of that reserve and adds it to `ProtocolOwed` of that token before
  storing the state. The tokens stay in the contract; only the accounting moves.
- This never breaks an engine's invariant: with the protocol part removed the reserve still
  holds the net input plus at least three quarters of the fee, and both engines guarantee their
  invariant already at zero fee (E-002 property 3, E-004 test 4). The state stored is a state
  the engine could have produced with a lower fee.
- `quote_exact_in` and `quote_exact_out` apply the same split, so a quote is what a swap gives.

### 3.5 Errors and events

Errors (`contracterror`, `u32`, stable across upgrades; the first hundred are reserved for the
contract, the engines' errors are mapped one to one after them):

| Code | Name | When |
|---|---|---|
| 1 | reserved | was `AlreadyInitialised`; a constructor cannot run twice |
| 2 | reserved | was `NotInitialised`; a pool cannot exist without its constructor having run |
| 3 | `WrongKind` | a band-only call on a stable pool or the reverse, or a direction of the wrong kind |
| 4 | `Unauthorised` | the address that authorised the call is not the role the entry point requires; a missing or wrong signature never reaches this code, it fails in the host's authorisation before the contract runs |
| 5 | `Paused` | swap or deposit while paused |
| 6 | `SameAsset` | `init_*` with one token twice |
| 7 | `TooManyDecimals` | a token with `decimals() > 18` |
| 8 | `SourcesMismatch` | `sources.len() != rate_config.sources` |
| 9 | `MainnetNeedsTwoSources` | `rate_config.sources < 2` on the public network |
| 10 | `OutOfRange` | a negative amount, a share above the caller's, a protocol share above the maximum; since version 8 also a `lower_cap` above the current cap and an `announce_cap` not above it |
| 11 | `SlippageExceeded` | `min_out`, `max_in`, `min_shares`, `min_first` or `min_second` not met |
| 12 | `CapExceeded` | the band cap of §3.2 (the stable engine's own is mapped from the engine) |
| 13 | `TransferMismatch` | the balance check of §3.3 |
| 14 | `NothingToWriteDown` | `announce_write_down` or `write_down` when the balance is not below the reserve |
| 15 | `NotPaused` | `announce_write_down` or `write_down` on a pool that is not paused |
| 16 | `NotAnnounced` | `write_down` without a standing announcement for that token; `upgrade` without a standing upgrade announcement (since version 7); `set_rate()`, `set_roles()` and `set_cap()` without a standing announcement of their kind (since version 8, ADR-012 amended) |
| 17 | `TooEarly` | `write_down` before the announced time; `upgrade` before the announced time (since version 7); `set_rate()`, `set_roles()` and `set_cap()` before theirs (since version 8) |
| 18 | `NotPoolToken` | a token address in a write-down call that is not one of the pool's two |
| 19 | `WriteDownPending` | `unpause` while a write-down announcement stands |
| 20 | `NotSuspended` | `set_rate` while the rate is active |
| 21 | `StorageMissing` | a required instance entry is absent; unreachable by construction (the constructor writes every entry before the contract exists, and no entry point removes one), reachable only after a broken upgrade migration, which is why it has its own code and is never mapped to another |
| 22 | `DuplicateSource` | the band constructor or `announce_rate` with one adapter address twice in `sources` (since version 7, ADR-012; at the announcement since version 8, because `set_rate()` takes no arguments and applies what was announced) |
| 23 | `AboveCurve` | `announce_cap` with a cap above `curve(now + CAP_DELAY)`, `set_cap()` with the announced cap above `curve(now)` (version 9, proposed, ADR-016, §3.8) |
| 10, also | `OutOfRange` | version 9 (proposed, §3.8): a `pause` whose `rule` is not one the caller may use (the sentinel `16 ..= 23`, the guardian `0`) |
| 16, also | `NotAnnounced` | `cancel_write_down` without a standing announcement, `cancel_upgrade` without one, and `cancel_rate`, `cancel_roles` and `cancel_cap` without one of their kind (since version 8); never a silent success |
| 100+ | engine errors | `band::PoolError`, `stable_pool::PoolError` and, inside them, `rate::RateError`, `stable::StableError`, `math::MathError`, each variant its own code, listed in the crate's README |

Events (topics in order, then data), every one carrying the pool's kind. Amounts in the data
are the engine's normalised units (18 decimals), as the contract holds them, except `collect`,
whose amounts are the token units transferred (noted 2026-09-16, as built; an indexer converts
with the token's decimals from its own configuration, never from the event):

| Event | Topics | Data |
|---|---|---|
| `init` | `("init", kind)` | tokens, roles (the three fields of `Roles` since version 6), config, cap, and for a band pool the rate config and the sources (since version 7, ADR-012) |
| `swap` | `("swap", direction, from)` | `amount_in`, `amount_out`, `fee`, `protocol_fee`, and for a band pool the rate edge used, for a stable pool `d_before` and `d_after` |
| `deposit` | `("deposit", from)` | first, second, shares |
| `withdraw` | `("withdraw", from)` | shares, first, second: the amounts paid, normalised; while a write-down announcement stands for a token, that token's amount is the scaled payout and its factor field (`first_factor`, `second_factor`) is `Scaled { reserve, balance }`, the announced numbers, else `Unscaled`; the reserve in the state falls by the unscaled payout (§3.2, stated 2026-10-12 as built, test 12) |
| `pause`, `unpause` | `("pause", caller)`: the topic field is named `caller` and holds the guardian or the sentinel that paused (since version 6, as built); `("unpause", guardian)`: the field is named `guardian` | `kind`, like every other event |
| `write_down_cancelled` | `("write_down_cancelled", token)` | the announced reserve and balance |
| `rate` | `("rate", status_after)` | status before, `low`, `high`, `updated_at`, and the reason when suspended |
| `refresh` | `("refresh", caller)` | per source: `Accepted`, `NotNewer`, `Ignored(reason)`, `NoObservation` |
| `roles`, `cap`, `protocol_share`, `rate_config`, `sources`, `collect`, `upgrade` | `(name, governance)` | the new value, and for `collect` the amounts; for `upgrade` the hash applied, the announced one (since version 7); `roles` is emitted by `set_roles()` and by `clear_sentinel`, whose topic field holds the caller, governance or the guardian; `cap` by `set_cap()` and by `lower_cap` (since version 8); queued for version 9 (2026-09-30, from A-005 §3.3): `roles` and `cap` carry `announced: bool`, `true` from `set_roles()` and `set_cap()`, `false` from `clear_sentinel` and `lower_cap`, because on version 8 a reader cannot tell from the row whether it ended a standing announcement |
| `upgrade_announced` | `("upgrade_announced", governance)` | the Wasm hash and the earliest execution time (since version 7, ADR-012) |
| `upgrade_cancelled` | `("upgrade_cancelled", caller)` | the announced hash; `caller` is the guardian or governance (since version 7, ADR-012) |
| `rate_announced`, `roles_announced`, `cap_announced` | `(name, governance)` | the announced values (the rate config and the sources; the three roles; the cap) and the earliest execution time (since version 8, ADR-012 amended) |
| `rate_cancelled`, `roles_cancelled`, `cap_cancelled` | `(name, caller)` | the announced values; `caller` is the guardian or governance (since version 8); `roles_cancelled` is also emitted by `clear_sentinel` when it clears an announcement that names a sentinel (§3.2, 2026-09-29), after its `roles` event |
| `write_down_announced` | `("write_down_announced", token)` | reserve, balance, the earliest execution time, and the owed protocol amount of that token before and after the lowering of §3.2 |
| `write_down` | `("write_down", token)` | reserve before, balance (the new reserve), the announced reserve and balance, and the owed protocol amount before and after the safety net of §3.2 |
| `incident` | `("incident", caller)`; `caller` is the sentinel, governance or the guardian that caused it, or the pool's own address for a `write_down` | `rule` (the I-003 rule of a sentinel pause; `0` otherwise), the cap before and after the halving, the new `base` and `since` (version 9, proposed, ADR-016, §3.8) |
| `cap_curve` | `("cap_curve", caller)` | `base` and `since`, emitted once by the backfill on a pool upgraded from version 8 (version 9, proposed, §3.8) |
| `pause`, version 9 | as above | `kind` and `rule`, the second argument of `pause` (version 9, proposed, §3.8) |

The indexer (brand §4.4) reads only events and views; it never derives a number another way.

### 3.6 Roles and keys

- Governance is the brain's key. During the capped beta it is the owner's cold key, kept
  offline and used only for the calls listed under governance, each of which is an ADR or a
  deploy script signed by hand (ADR-002 point 6). Before the caps of ADR-002 are lifted,
  governance moves to a multisig of at least 2-of-3 (`set_roles` to the multisig's address; a
  Soroban multisig contract or a multi-signer account); the ADR that lifts the first cap records
  the signers, and no cap is lifted while governance is a single key.
- The guardian is the pause key: a separate key on a hardware wallet the owner carries, so that
  a pause can be signed within minutes from anywhere. It can pause, suspend and resume, and
  since version 7 it can cancel an announced upgrade (ADR-012), since version 8 every
  standing announcement of governance: an upgrade, a rate change, a roles change and a cap
  raise (ADR-012 amended), so that a key watching the pool can stop a change to the code,
  the price path, the keys or the exposure during its delay, and it can clear the sentinel
  at once (`clear_sentinel`, ADR-012 point 15: a power taken away by the incident key); it
  cannot change a cap, a fee, a source or the Wasm, name a role, and it cannot move funds.
  The veto holds the guardian's own
  replacement too, which is the point (governance cannot appoint itself guardian in one
  ledger and cancel nothing) and the cost: a stolen or unwilling guardian key blocks every
  change to the roles, the rate, the cap and the code for good, with the pool trading or
  paused as it stands and withdrawals open throughout. Accepted for the capped beta, where
  the guardian is a hardware wallet in the owner's hand and the sum at risk is the caps; the
  ADR that lifts the caps revisits it together with the multisig below. Two roles may hold one
  address: the constructor and `set_roles` refuse nothing there, by design (stated 2026-09-28,
  ADR-012), because the testnet setup is one key for deployer, governance and guardian, and on
  mainnet the deploy ADR records three distinct keys, which is the rule. If the guardian key is
  lost, governance sets a new one. If the governance key is lost, every switch freezes for
  good: no new roles, no cap, no fee change, no source change, no write-down, no upgrade, and
  no collection of protocol fees; the pool keeps trading and paying withdrawals exactly as it
  is, and the guardian can still pause it. There is no recovery path in the contract by
  design, which is why that key stays offline, why its backup is arranged outside the contract,
  and why it becomes a multisig before the caps are lifted.
- The sentinel is the pause-only key (added 2026-09-27, I-003 §3.5): an operational hot key held
  by the watchtower of I-003 on the machine that runs it, read from a file outside every
  repository, rotated by governance (since version 8 in two steps, ADR-012 point 15: removed
  at once with `clear_sentinel`, the new key named through `announce_roles` and `set_roles()`
  after the delay), and `None` on a pool that has no watchtower yet. It can call `pause` and
  nothing else: not `unpause`, not `suspend`, not `resume`, no switch, no fund. Stolen, it
  can stop trading until governance or the guardian clears it, at once, and the guardian
  unpauses, and that
  bound is why a hot key may hold this role and no other; the day of the roles delay applies
  to naming its successor, when no stolen key holds the role any more. It
  is not the guardian key, not the governance key, and never held by an agent. The guardian
  keeps `unpause`, because the one who unpauses must have read why the pool was paused, and a
  program cannot.
- The ADR that deploys the first mainnet pool records the three addresses (or that the
  sentinel is `None`), where the keys are held, and who else (if anyone) may hold the guardian
  key. Brief §14's open item closes with that ADR.
- Agents never hold any of the three keys and never sign a transaction (E-000 rule 8). An
  agent may build the transaction a human signs; the watchtower is a program, not an agent.
- The bytes a deploy script rolls out are the artefact CI produces (amended 2026-09-13): the
  release Wasm the workflow builds and optimises on its Linux runner, uploaded by the workflow
  as a build artefact together with its sha256, on a push to `main` only, never from a pull
  request run, so that one artefact name never carries the bytes of an unmerged branch. Every
  action the workflow uses is pinned to a full commit SHA, not a floating tag, because the
  actions run with write access to the workspace and a replaced action could swap the bytes
  after the verification step. The deploy ADR binds each contract to three things: the commit
  SHA on `main`, the run id of the green workflow run, and the sha256 of the Wasm as printed
  in that run's job log, never taken from the `.sha256` files inside the artefact, which the
  upload action itself could have replaced. The script checks the downloaded file against the
  hash of the ADR before it uploads the Wasm. What the hash proves is the transport from CI
  to the deploy, not the build: it is taken in the job that runs the build scripts and proc
  macros of every dependency, so the review of `Cargo.lock` is the guard for the build, and
  the ADR does not describe the hash as proof of provenance. Artefacts expire after 90 days,
  the platform default, which stays as it is (decided 2026-09-13): the ADR does not depend on
  the artefact, because the operator archives the deployed Wasm with its hash and the three
  values in the brain's deploy records right after every deployment, and a re-run of the
  same commit must reproduce the hash. A Wasm built on anyone's machine is never deployed, because the
  optimiser's output differs by platform (the crate's README records 3 bytes between the macOS
  and the Linux build of the same binaryen version). A-002 carries this rule for Rhumb's
  scripts.

### 3.7 Code hygiene

- The crate is a Soroban contract (`#![no_std]`, `contractimpl`, `contracttype`,
  `contracterror`), built for `wasm32v1-none` with the workspace lints; `soroban-sdk` pinned
  to the version the test wrappers use, bumped only in its own commit.
- Dependencies: `pool-abi`, `band`, `stable-pool`, `stable` (for `Amp` and `StableError` at
  the boundary), `rate`, `math`, `soroban-sdk`. Test-only: `soroban-sdk` with `testutils`,
  `proptest`.
- The types other contracts need to talk to a pool live in a small shared library crate
  `pool-abi` (amended 2026-09-11 for E-008): the byte codec of the E-003 types (`RateConfig`
  as `BytesN<119>`, `RateState` as `BytesN<153>`, the layout of the E-003 test wrapper),
  `Direction`, the pool kind, and the contract error enum with its codes. `pool-abi` is
  `no_std`, depends on `rate`, `math` and `soroban-sdk` only, exports no contract, and
  changes only together with this spec; `pool-soroban` re-exports nothing and stays a
  `cdylib`. The router contract (E-008) depends on `pool-abi`, never on `pool-soroban`. The
  rule for every shared crate (amended 2026-09-12): a shared crate carries no exported contract
  type, because a `contracttype` or `contracterror` in a library lands in the spec section of
  every Wasm that links it; the error codes and the pool kind are plain enums written once
  behind macros, and each contract builds its own `contracterror` wrapper from them, so that a
  Wasm's spec section holds exactly its own interface. `Direction` is the one shared
  `contracttype`, because both contracts take it at their boundary and it must be one type.
  Exported signatures name byte types literally (amended 2026-09-13, from the first rollout
  of A-002): the constructor, `set_rate`, `rate_config()` and `rate_state()` are written with
  `BytesN<119>` and `BytesN<153>` in the entry point's own signature, never through a type
  alias such as `RateConfigBytes` or `RateStateBytes`, because `contractimpl` takes the
  alias's name into the published spec as a user-defined type that nothing defines, and a
  client that reads the spec, the `stellar` CLI first of all, cannot build or decode those
  calls. The alias may exist in `pool-abi` for the code; it never appears in a signature the
  Wasm exports. CI proves it on the artefact: after `wasm-opt` and before the upload, the
  workflow runs `stellar contract info interface --wasm <file>` (the CLI pinned by version)
  on both contract Wasms and fails when the published spec names a type it does not define,
  or when the output is not the interface `crates/<crate>/interface.rs` states: that file is
  the interface of record of each contract, the one the check compares the artefact against,
  and the crate's README carries a paragraph that points at it instead of restating it
  (amended 2026-09-14, as built); the sentinel's pull request of I-003 updates `interface.rs`
  in its test commit, and `VERSION` to `6`. The change to the
  pool's published argument and return types raises `VERSION` to `5` under the rule of §3.1,
  test first, although the bytes on the wire do not change; the router's spec is checked by
  the same step and is expected unchanged. A new Wasm means a new hash, a new artefact row
  in every `chain-config/` of A-002 and a new fixture for A-001 test 4, each in its own
  commit, and nothing deployed from the old bytes is kept: the testnet contracts are
  redeployed from the new artefact.
- The Wasm size and the CPU cost of every entry point are in the benchmark of §4; a swap must fit
  a Stellar mainnet transaction with room for a router around it. The Wasm must stay below
  `122 880` bytes (120 KB, against the network's limit of `131 072`), and CI fails a build
  above it, so that an upgrade during the beta always has room. An upgrade that would cross the
  bound is a separate decision: an ADR, not a change to the bound in the workflow. Size is won
  by byte views for the E-003 types, by turning the doc comments of contract types, events and
  the error enum into ordinary comments (the SDK copies every doc string of a contract type into
  the Wasm's spec section; the documentation stays in the source, and every entry point keeps
  its doc comment), or by `wasm-opt` in the release build; never by dropping an event or a
  field the indexer reads, and never by splitting the Wasm per kind (one Wasm serves both kinds,
  §1). The first two levers are used. `wasm-opt` is the third, now due (amended 2026-09-12: the
  margin is `2 126` bytes, `78` above the threshold of `2 048`), as its own pull request and
  its own rule: CI runs `wasm-opt` (binaryen, exact version pinned in the workflow, `-Oz` with
  the flags the SDK documents as safe for Soroban) on the two contract Wasms, `pool_soroban`
  and `router_soroban`, after the build and before the size check, the byte-identity tests and
  the benchmark, so that what is measured and what is deployed are the same bytes; the test
  wrappers and the mocks stay outside `wasm-opt` (amended 2026-09-13), because they are never
  deployed and their byte-identity tests measure the code as `cargo` builds it; the workflow
  reports both sizes and, on a push to `main` only, uploads every optimised contract Wasm with
  its sha256 as the build artefact of §3.6 (amended 2026-09-13), with every action pinned to a
  commit SHA. Every `cargo` invocation of the workflow passes `--locked` (amended 2026-09-13):
  §3.6 names the review of `Cargo.lock` as the guard for the build, and without `--locked`
  Cargo may rewrite the lockfile in CI when a `Cargo.toml` requirement changes, so that what
  was reviewed is not what was built; with it, a lockfile that does not match fails the job.
  A pinned SHA never moves on its own, so a fix in a later version of an action arrives only
  through a bump: Dependabot for GitHub Actions runs monthly (`.github/dependabot.yml`,
  `package-ecosystem: github-actions`), every bump is a pull request, and the security
  reviewer checks the new SHA against the action's release before it is merged (amended
  2026-09-13); no other path changes a pinned SHA. The push-on-`main` condition of the upload
  is held by two checks, one static on the workflow text and one at run time against the
  event and the ref; a condition of the shape `always() || <the condition>` on one line
  passes the static check and is caught by the run-time check, which turns the job red
  (noted 2026-09-13). The static pattern is not tightened for it without a real CI run that
  confirms the tighter pattern, because the expression braces it would need are expanded by
  Actions before a script sees them.
  The bound and the threshold apply to the optimised Wasm from then on; the pull request that
  introduces it re-measures the benchmark baseline (§4.16.1) and states both sizes in the
  README.
- No code is ported. Where a pattern follows a known Soroban contract (the token interface, the
  upgrade call, TTL handling), the doc comment names the SDK documentation, not a contract.

### 3.8 Version 9: the cap curve, the pause reason and the announcements view (proposed amendment, ADR-016, 2026-10-12)

Proposed: one pull request in core, tests first, as `VERSION` 9, a new deployment, after
A-010 is done and watchtower pull request 7 is merged (its builder of the `pause` call reads
the pool's version), before watchtower pull request 8, beside E-017 steps 1 and 2 (ADR-016 §8
point 19). It bundles the two items queued for version 9 in the header (the `announced` flag
of `roles` and `cap`; the view of the standing announcements, I-003 §3.1 point 9) with
ADR-016. Everything of versions 5 to 8 stands; this section lists only what changes. The
numbers are ADR-016's: `CURVE_PERIOD = 2 592 000` seconds (thirty days), `CAP_FLOOR = 10^21`
normalised, both constants of the Wasm.

Storage (§3.1): `CapCurve { base: u128, since: u64 }`, written by the constructor with `base`
the cap of the `init` and `since` its ledger time. The curve is
`curve(t) = base · 2^floor((t − since) / CURVE_PERIOD)`, the shift saturating at `u128::MAX`.
On a pool upgraded from version 8 the entry is absent: the first entry point or view that
reads it writes `base` equal to the current cap (band `Cap`, stable `config.cap`) and
`since = now`, emits `cap_curve` once and never again (ADR-016 §1 point 4; a pool's age
before the upgrade is not credited, there is no `set_curve`).

Entry points (§3.2):
- `pause(caller: Address, rule: u32)` replaces `pause(caller)`. After the auth and role checks
  of version 6, `rule` must be `16 ..= 23` from the sentinel and `0` from the guardian, else
  `OutOfRange`. A sentinel `pause` with `rule` in `{16, 17, 18, 19, 20, 23}` is an incident
  and halves (below); with `21` or `22`, and every `pause` by the guardian, it pauses as
  before and leaves `CapCurve` untouched (ADR-016 §3 point 8a: rules 21 and 22 are policy,
  not invariants). `Paused` stays idempotent; the halving happens on every such call, paused
  already or not, so a second sentinel pause on an invariant halves again (ADR-016 §4 point 12).
- `announce_cap(cap)` is refused with `AboveCurve` when `cap > curve(now + CAP_DELAY)`, after
  the `OutOfRange` check of version 8. `set_cap()` checks `cap <= curve(now)` again at
  execution and is `AboveCurve` otherwise, the announcement left standing so that a cancel is
  explicit; on success it applies as before. `lower_cap(cap)` is unchanged and leaves
  `CapCurve` as it is (ADR-016 §6).
- `mark_incident()`, immediate, by governance or the guardian (`caller.require_auth()` then
  the role check, as `pause`); anyone else `Unauthorised`. It halves and emits `incident`; it
  does not pause (ADR-016 §3 point 10).
- The halving, in one transaction, on a sentinel `pause` with a rule of the set above, on
  `write_down` after the write-down's own work, and on `mark_incident`:
  `cap' = max(cap / 2, CAP_FLOOR)` (band `Cap`, or the stable `config.cap` through the
  engine's `PoolConfig` as `lower_cap` does), `CapCurve = { base: cap', since: now }`; a
  standing `CapAt` is removed with `cap_cancelled` as `cancel_cap` emits it; then `cap` with
  the new value and `announced: false`, then `incident`. A halving never raises: when
  `cap / 2` is above `curve(now)` the cap is `cap / 2` all the same and the curve continues
  from it (ADR-016 §4 point 13).
- Views: `cap_curve() -> (CapCurve, u128)`, the entry and `curve(now)`, writing the backfill
  when the entry is absent; `announcements() -> Announcements { upgrade: Option<UpgradeAt>,
  rate: Option<RateAt>, roles: Option<RolesAt>, cap: Option<CapAt> }`, the four standing
  announcements, read-only, so the watchtower reads them at its start and the 24-hour
  exception of I-003 rule 23 goes (I-003 §3.1 point 9); and so that the watchtower's own
  record of announcements is a cache and the chain the witness: a database that does not
  know the chain's time cannot refuse a row with a self-chosen `at`, and the view can (added
  2026-10-13, I-003 header (ae)).

Errors (§3.5): `23` `AboveCurve`; `OutOfRange` (10) also a `pause` whose `rule` the caller
may not use. Events (§3.5): `pause` carries `rule` in its data; `roles` and `cap` carry
`announced: bool` (the header); `incident` and `cap_curve` as the table has them. `VERSION`
is `9`; the interface of record of §3.7 changes with it, and the pull request declares a new
baseline of §4.16.1 (ADR-012 point 14's rule) with new rows `pause` (with its rule, the
halving path and the plain path), `mark_incident`, `cap_curve` and `announcements`, and
`announce_cap` and `set_cap` remeasured.

Readers, after version 9 is on testnet, each a small amendment of its own spec: the SDK and
the indexer decode `pause` with its `rule`, `incident`, `cap_curve` and the `announced` field
(`POOL_VERSIONS` gains `9`, A-005 §3.3; the decoder of A-003 §3.3 and I-003 §7.3); the app
shows `curve(now)` beside the cap on the pool page (A-006). E-017 checks this section as its
row C from version 9.

## 4. Reference tests (written before the code)

All tests run in the Soroban test environment with the contract's Wasm, the SDK's Stellar Asset
Contract as tokens (7 decimals) and a mock token with 18 decimals, a mock adapter per source that
returns what the test tells it, and the ledger's timestamp and network id set by the test.

1. Creation: the constructor stores what §3.1 says for both kinds and a refused deployment
   leaves no contract: `asset == quote`, a 19-decimal token, a source count that does not match,
   and, with the ledger's network id set to the public network's, `sources = 1` all fail the
   deployment; with the test network id `sources = 1` is accepted; there is no `init` export.
   Since 2026-09-27 (I-003 §3.5): both kinds are created with `sentinel: Some(address)` and with
   `None`, `roles()` returns what was given, and the `init` event carries the three fields.
   Since 2026-09-28 (ADR-012): one adapter address listed twice in `sources` fails the
   deployment with `DuplicateSource` (the test of the exploit pass, taken in with its
   assertion on the refusal); the `init` event of a band pool carries the rate config and the
   sources, and `sources()` returns them; on a stable pool `sources()` is `WrongKind`.
2. Swaps equal the engines: for a generated set of pools, states and requests (the generators of
   E-004 and E-005), `swap_exact_in` and `swap_exact_out` move exactly the token amounts the
   engine's result gives after conversion (input exact or ceiled, output floored), store the
   engine's state minus the protocol part, and emit the event with the same numbers; the known-
   answer vectors of E-004 tests 12 to 14 and E-005 tests 12 and 13 reproduce through the
   contract with `ProtocolShare = 0`; every engine error surfaces with its mapped code and no
   token moves. The generators draw only inside the domains the engines guarantee (amended
   2026-09-14, from the 1 000-case runs of E-009): for a stable pool both normalised balances
   at least `10^18` and their ratio at most `1024` (E-002 §3.1), for a band pool the domain of
   E-004 §3; a harness never unwraps an engine error, it asserts the mapped code, so that a
   drawn case outside a domain is a failing assertion on the generator and never a panic in
   the harness.
3. Deposits and withdrawals equal the engines: the same for `deposit` and `withdraw`, including
   the share ledger (`Shares(from)` and `total_shares()` equal the engine's counts), the vectors
   of E-004 test 17 and E-005 test 14, and the dust rule (a normalised amount that is not a token
   unit is taken with a ceiling and paid with a floor, and the contract's balance minus the
   reserves never falls below zero and never exceeds one token unit per token per operation).
4. Withdraw always: on a paused pool, on a band pool whose rate is stale, suspended or lost, and
   on a stable pool below one token per side, `withdraw` pays what the engine says; every other
   state-changing call on those pools fails with the expected code.
5. Slippage: `min_out`, `max_in`, `min_shares`, `min_first` and `min_second` are checked exactly
   at the boundary (equal passes, one unit worse fails), and a failed check moves no token.
6. Transfers: a mock token that takes a fee on transfer makes every swap and deposit fail with
   `TransferMismatch` and no state change; a rebasing mock likewise; the token's authorisation is
   required (a call without `from`'s auth fails); a call by a router-like contract on a trader's
   behalf succeeds when the trader authorised the call tree and fails when they did not.
7. Cap: a band pool's deposit is refused exactly when `asset' · low + quote'` exceeds the cap
   (boundary tested), a stable pool's by its engine; swaps ignore the cap; since version 8
   (ADR-012 amended): `lower_cap` by governance moves the boundary down at once and is
   `OutOfRange` above the current cap; `announce_cap` is `OutOfRange` at or below the current
   cap and above it records the value and the time and emits `cap_announced`; `set_cap()` one
   second before the time is `TooEarly`, at the time it moves the boundary up, clears the
   announcement and emits `cap`; `cancel_cap` by the guardian and by governance clears it and
   emits `cap_cancelled`; each of the four by anyone else fails.
8. Pause (amended 2026-09-27, I-003 §3.5): `pause` by the guardian or by the sentinel, `unpause`
   by the guardian only; `pause` by governance, by a stranger, and by the address a pool of
   `sentinel: None` would have had, fails with `Unauthorised`, and a `pause` without the
   caller's auth fails in the host before the contract runs; the sentinel cannot `unpause`,
   `suspend`, `resume`, `lower_cap`, `announce_cap`, `announce_roles`, `cancel_roles` or call
   any other role entry point (each fails with `Unauthorised` or in the host); a roles
   announcement with a new sentinel, executed after the delay (since version 8), takes effect
   for the next `pause`; `clear_sentinel` by governance removes it at once, emits `roles`,
   and the removed key's next `pause` is `Unauthorised`; `clear_sentinel` by the guardian
   does the same, and the event names the guardian as the caller; by the sentinel and by a
   stranger it fails (ADR-012 point 15); added 2026-09-29 from the review of the version 8
   pull request: after `announce_roles` naming a sentinel, `clear_sentinel` by governance,
   and in a second run by the guardian, clears the announcement too, emits `roles` and then
   `roles_cancelled`, both with the caller, in that order, and `set_roles()` at the announced time is `NotAnnounced`,
   so the announced key never gets the role; after `announce_roles` with `sentinel: None`,
   `clear_sentinel` leaves the announcement standing and `set_roles()` at the time applies
   it; the `pause` event's topic names the caller. While
   paused, swaps and deposits fail with `Paused` before any engine call (a paused stable
   pool's stored engine state has the flag set too), `withdraw` works, `refresh` works;
   unpausing restores everything without a state change.
9. Rate: `refresh` by anyone applies each adapter's observation through `rate::update` in source
   order at the ledger timestamp and skips a `None`, a failing adapter, `NotNewer` and gate
   failures, storing the state E-003's reference gives for the same sequence (E-003 test 7's
   two-source scenarios and test 10's YieldBlox case, driven through adapters); `suspend` by the
   guardian and `resume` by the guardian or governance behave as E-003 says; a suspended pool
   refuses swaps and deposits with the mapped `Suspended(reason)`; `rate` events carry the
   transition.
10. Protocol fee: with `ProtocolShare = 2 500` a swap of E-005 test 12 stores the reserve `125 ·
    10^15` lower than the engine's state and `ProtocolOwed` that much higher; the engine's
    invariant check still holds on the stored state (`compute_d` of the stored reserves is not
    below `d_before`); `collect_protocol_fees` by governance transfers the floored amount to the
    named address and leaves the dust owed; `set_protocol_share` above `MAX_PROTOCOL_SHARE` fails;
    quotes equal swaps with the switch on.
11. Roles: every governance and guardian entry point fails for the other role and for a
    stranger, the cancels excepted, which both may call; since version 8 (ADR-012 amended)
    `announce_roles` records the three roles and the time and emits `roles_announced`,
    `set_roles()` one second before the time is `TooEarly`, at the time it applies them,
    clears the announcement, emits `roles` and takes effect for the next call, and without an
    announcement it is `NotAnnounced`; `cancel_roles` by the guardian, also of an
    announcement that would replace the guardian, and by governance clears it and emits
    `roles_cancelled`; a second `announce_roles` replaces the first and restarts the delay;
    the mainnet rule is re-checked by `announce_rate`, and a repeated address is
    `DuplicateSource` there (since 2026-09-28, ADR-012); `announce_rate` is accepted while the
    rate is active and emits `rate_announced`; `set_rate()` is refused while the rate is
    active (`NotSuspended`) and before the time (`TooEarly`), after both it changes the
    source count when config and sources agree, leaves `RateState` untouched (the suspension
    and the stored observations stay), and the `resume` after it clears the observations and
    emits `rate`; `cancel_rate` by either role clears the announcement and emits
    `rate_cancelled`.
12. Write-down: after the test burns tokens from the contract's balance (the SAC's clawback),
    `announce_write_down` on a paused pool records the time and the numbers and emits the
    announcement; `write_down` one second before the announced time fails with `TooEarly`, at
    the time it sets the reserve to the balance (and to a lower balance when more was burned in
    between, never to a higher one), clears the announcement, emits the event, and every LP's
    withdrawal shrinks pro-rata; both fail on an unpaused pool and when nothing is missing;
    with `ProtocolOwed` of that token above zero the announced and executed reserve is the
    balance minus the owed amount: a loss smaller than the owed amount leaves the owed amount
    untouched and lowers the reserve by the whole loss, so every LP's payout shrinks pro rata
    (stated 2026-09-28, ADR-012; the test of the exploit pass is taken in with its assertion
    turned to this rule, and the existing test that pinned it under the name "the protocol
    carries the first loss" is renamed to what it pins), and when the balance is below the
    owed amount the owed amount
    is lowered to the balance and the reserve becomes `0`, after which `collect_protocol_fees`
    transfers what is owed and every LP can still withdraw its full share;
    `write_down` without an announcement fails with `NotAnnounced`; `unpause` by the guardian
    fails with `WriteDownPending` while an announcement stands and clears nothing;
    `cancel_write_down` by governance clears it and emits the event, by the guardian it fails.
    Factor: after an announcement with reserve `R` and balance `B`, a
    withdrawal of `s` shares during the delay pays `floor(floor(s · R / S) · B / R)` of that
    token, the same as the engine's payout after the write-down up to one unit of the double
    floor, and the other token in full; the event carries `R` and `B`; after
    `cancel_write_down` the factor is gone and a withdrawal pays the engine's payout again;
    when the balance rose during the delay the executed reserve is the announced one, not the
    higher balance (nothing an LP was paid on can be exceeded by staying); a second clawback during the
    delay lowers the executed reserve below `B` and is not covered by the factor; when that
    second clawback leaves the balance below the owed protocol amount, `write_down` lowers the
    owed amount to the balance and sets the reserve to `0`, after which
    `collect_protocol_fees` transfers the lowered amount, a new `announce_write_down` is not
    blocked, and the balance is never below the reserve plus the owed amount.
13. Upgrade (rewritten 2026-09-28, ADR-012): `announce_upgrade` by governance records the hash
    and the time and emits `upgrade_announced`; by the guardian or a stranger it fails;
    `upgrade` one second before the announced time fails with `TooEarly`, at the time it
    replaces the Wasm with the announced hash, the state survives, the announcement is
    cleared and `upgrade` is emitted; `upgrade` without an announcement fails with
    `NotAnnounced`, and so does `cancel_upgrade`; `cancel_upgrade` by the guardian and by
    governance clears the announcement and emits `upgrade_cancelled` with the caller, after
    which `upgrade` is `NotAnnounced` again; a second `announce_upgrade` while one stands
    replaces the hash and restarts the delay from that moment; `withdraw` and every other
    entry point work unchanged while an announcement stands. Since 2026-09-27 (§3.2): a pull request that changes the shape of a stored type
    adds a test that writes the old shape into storage, upgrades, and reads every entry back
    through the new type; a pull request without a migration step adds no such test and
    states in its description that the change is a new deployment.
14. TTL: the instance TTL is extended by every state-changing call and by `restore_shares` when
    below the threshold, and by no view (every view is checked); a `Shares` entry is extended on
    deposit and withdraw; `restore_shares` restores an archived entry and moves nothing.
15. Events: every entry point emits exactly the event of §3.5 with the listed topics and data,
    checked field by field.
16. Cost: a benchmark table in `crates/pool-soroban/README.md` with CPU instructions, memory
    bytes, the ledger footprint (bytes read and written, entries touched) and the Wasm size for
    every entry point, with the inputs of tests 2 and 3 for both kinds, a `refresh` with four
    sources, and the balance check counted; the swap rows must
    leave room for a router around them inside the mainnet transaction limit. The first table is
    the baseline; the 10% rule of E-001 §4.11 applies.
17. The governance rate path (added 2026-09-28, ADR-012 amended; the test of the branch
    `exploit-pass/pool-governance-rate-path`, the first commit of the version 8 pull request,
    its assertion turned to the rule): on a band pool with two sources, the governance key
    alone announces roles that name its own address as guardian, announces a rate config with
    two adapters of its own, and at every ledger before the announced times every execution
    (`set_roles()`, `set_rate()`) is `TooEarly`, `suspend` is refused for it (it is not the
    guardian yet), and every swap prices with the rate the two original adapters confirm.
    `resume` is open to governance throughout, as §3.2 and test 9 say (corrected 2026-09-29
    from the review of the version 8 pull request: the test claimed "no `resume` for
    governance", which no rule of §3.2 gives; a resume is the safe direction, it serves no
    rate until two sources have agreed), and before `set_rate()` it only brings back the
    rate the two original adapters confirm; the guardian's `cancel_roles` and `cancel_rate` clear both
    announcements, after which the executions are `NotAnnounced`; and without the cancel,
    at the announced time, `set_roles()` makes it guardian, `suspend`, `set_rate()` and
    `resume` go through, and only then do its adapters set the price: the path exists, and it
    takes no less than `UPGRADE_DELAY` with a warning event at its start. The same with a
    cap: `announce_cap` and `set_cap()` before the time is `TooEarly`, `lower_cap` at once.
18. The cap curve (version 9, proposed, ADR-016, §3.8): on a fresh pool `cap_curve()` returns
    `base` equal to the init cap and `curve(now)` equal to it; after `CURVE_PERIOD − 1`
    seconds still equal, at `CURVE_PERIOD` double, at `k · CURVE_PERIOD` `base · 2^k`, and
    `u128::MAX` at a jump that would overflow; `announce_cap` at exactly
    `curve(now + CAP_DELAY)` succeeds and one unit above is `AboveCurve`; a `set_cap()` whose
    announced cap is above `curve(now)` at execution (a halving in the delay) is `AboveCurve`
    and the announcement stands until `cancel_cap`; `lower_cap` leaves `CapCurve`; a pool
    created from the version 8 Wasm and upgraded to version 9 along test 13's path has no
    `CapCurve` until the first read, which writes `base` equal to the cap at that moment and
    `since` to that ledger and emits `cap_curve` once and never again; the same for a stable
    pool with `config.cap`.
19. Incidents and the halving (version 9, proposed, ADR-016, §3.8): `pause(sentinel, r)` for
    each `r` in `16 ..= 20` and `23` halves the cap, sets `base` and `since`, clears a standing
    `CapAt` with `cap_cancelled`, emits `cap` (`announced: false`) and `incident` in that
    order, and pauses; `pause(sentinel, 21)`, `pause(sentinel, 22)` and `pause(guardian, 0)`
    pause and leave the cap and `CapCurve` unchanged; `pause(sentinel, 0)`,
    `pause(sentinel, 24)` and `pause(guardian, 16)` are `OutOfRange` and change nothing; two
    invariant pauses in a row quarter the cap; from `2 · CAP_FLOOR − 1` a halving lands on
    `CAP_FLOOR` and a further one stays there; `write_down` halves after its own work in the
    same transaction; `mark_incident` by governance and by the guardian halves, emits
    `incident` with the caller and does not pause, by anyone else is `Unauthorised`; a halving
    when `cap / 2 > curve(now)` still sets `cap / 2`, and the next `announce_cap` is bounded
    by the new curve.
20. The announcements view and the `announced` flag (version 9, proposed, §3.8):
    `announcements()` returns `None` four times on a fresh pool, each standing announcement
    after its `announce_*`, and `None` again after its `set_*`, `upgrade` or `cancel_*`;
    `clear_sentinel` clears the roles one that names a sentinel; `roles` from `set_roles()`
    and `cap` from `set_cap()` carry `announced: true`, from `clear_sentinel`, `lower_cap`
    and a halving `false`.

### 4.16.1 Benchmark baseline

Measured in the `wasm` job of run `36690431683` on `main`, the merge of core pull request 27
(commit `a499892`, pool `VERSION` 8, 2026-09-30), as E-009 §3.5 provides: soroban-sdk 27.0.6
in a Soroban test `Env`, Rust 1.98.1, target `wasm32v1-none`, release profile of the
workspace (`opt-level = "z"`, `lto = true`, `overflow-checks = true`), then `wasm-opt` from
binaryen `version_132` with the flags the workflow runs; the pool Wasm measured is the CI
artefact of §3.6 itself, `106 019` bytes (sha256 `fa3f548b…f3b1`, A-002 §7.2). Method as
E-001 §4.12: the contract is deployed from its Wasm with its constructor; for each row the
budget is reset, the entry point is invoked once through the Wasm with the input shown, CPU
and memory are read from the budget, and the ledger footprint from the host's resources of
that invocation (entries read, live and from disk; entries written; bytes read from disk, `0`
for live state; bytes written). The numbers include the fixed cost of one invocation and of
the token calls the entry point makes (the balance check of §3.3 counted); the tokens are
Stellar Asset Contracts; the band pool has four sources; the protocol share is `2 500`; the
constructor rows measure the whole deployment (the host function that creates the contract
and runs the constructor). This table replaces the table of 2026-09-12 (the first on the
optimised Wasm, measured on a local build) as the baseline, declared by the version 8 pull
request (ADR-012 point 14): the announced calls (`announce_rate`, `announce_roles`,
`announce_cap`, `announce_upgrade`, their `cancel_` and the `set_` or `upgrade` that applies
them), `lower_cap`, `clear_sentinel` and `sources` are new rows, every pool is created with a
sentinel since version 6, and the storage grew with the announcements, so the older rows
stand a few per cent above the table of 2026-09-12 in CPU and about 45 000 bytes above it in
memory; `upgrade` stood at +9.57 % after version 7 and the delay. The mainnet limit is
100 000 000 CPU instructions per transaction; the costliest swap is under 4 000 000, so a
router around it has room for many legs.

| Entry point | Input | CPU instructions | Memory bytes | Entries read | Entries written | Bytes read (disk) | Bytes written |
|---|---|---:|---:|---:|---:|---:|---:|
| `__constructor, band` | `(KAT config, four sources)` | 1794071 | 3000787 | 6 | 2 | 0 | 1544 |
| `refresh, four sources` | `(four fresh observations)` | 3337921 | 6617891 | 9 | 2 | 0 | 1544 |
| `refresh, four sources, nothing new` | `(the same observations)` | 3031848 | 6617263 | 9 | 2 | 0 | 1544 |
| `set_protocol_share` | `(2 500)` | 917485 | 1687506 | 4 | 2 | 0 | 1544 |
| `deposit, genesis, band` | `(1e6 / 1e6)` | 2077915 | 1919502 | 13 | 7 | 184 | 2584 |
| `deposit, proportional, band` | `(5e5 / 6e5 offered)` | 2245864 | 1900777 | 11 | 7 | 0 | 2584 |
| `swap_exact_in, band, pool buys asset` | `(1 000 asset in)` | 2496210 | 1873566 | 10 | 6 | 0 | 2440 |
| `swap_exact_in, band, pool sells asset` | `(1 000 quote in)` | 2501542 | 1872330 | 10 | 6 | 0 | 2440 |
| `swap_exact_out, band, pool sells asset` | `(1 000 asset out)` | 2528626 | 1873274 | 10 | 6 | 0 | 2440 |
| `swap_exact_out, band, pool buys asset` | `(1 000 quote out)` | 2669517 | 1874090 | 10 | 6 | 0 | 2440 |
| `quote_exact_in, band` | `(1 000 asset in)` | 1706060 | 1657696 | 2 | 0 | 0 | 0 |
| `quote_exact_out, band` | `(1 000 asset out)` | 1710976 | 1657696 | 2 | 0 | 0 | 0 |
| `collect_protocol_fees` | `(the fees of the four swaps owed)` | 1493503 | 1870950 | 12 | 6 | 184 | 2440 |
| `withdraw, band` | `(a third of the shares)` | 1701713 | 1868755 | 11 | 7 | 0 | 2584 |
| `state` | `()` | 829416 | 1658368 | 2 | 0 | 0 | 0 |
| `rate_state` | `()` | 811500 | 1657672 | 2 | 0 | 0 | 0 |
| `sources` | `()` | 808822 | 1657672 | 2 | 0 | 0 | 0 |
| `shares` | `(the LP)` | 837852 | 1667695 | 3 | 0 | 0 | 0 |
| `version` | `()` | 685266 | 1653562 | 2 | 0 | 0 | 0 |
| `suspend` | `()` | 1093882 | 1698245 | 4 | 2 | 0 | 1544 |
| `announce_rate` | `(four sources, four adapters)` | 1023567 | 1706300 | 4 | 2 | 0 | 1920 |
| `cancel_rate` | `(by the guardian)` | 960323 | 1699256 | 4 | 2 | 0 | 1544 |
| `set_rate` | `(four sources, four adapters, announced)` | 1072003 | 1701491 | 4 | 2 | 0 | 1544 |
| `resume` | `(1.0)` | 1130865 | 1696253 | 4 | 2 | 0 | 1544 |
| `pause` | `()` | 931731 | 1694502 | 4 | 2 | 0 | 1544 |
| `announce_write_down` | `(a tenth of the asset clawed back)` | 1104760 | 1754648 | 6 | 2 | 0 | 1724 |
| `withdraw, band, with a write-down factor` | `(a third of the shares)` | 1808558 | 1890807 | 11 | 7 | 0 | 2764 |
| `cancel_write_down` | `(the asset)` | 989380 | 1700699 | 4 | 2 | 0 | 1544 |
| `write_down` | `(the asset)` | 1148902 | 1752197 | 6 | 2 | 0 | 1544 |
| `unpause` | `()` | 954691 | 1696940 | 4 | 2 | 0 | 1544 |
| `announce_roles` | `(the same roles)` | 968060 | 1707208 | 4 | 2 | 0 | 1760 |
| `cancel_roles` | `(by the guardian)` | 973609 | 1701754 | 4 | 2 | 0 | 1544 |
| `set_roles` | `(the same roles, announced)` | 982338 | 1702838 | 4 | 2 | 0 | 1544 |
| `lower_cap` | `(2^100)` | 952678 | 1698230 | 4 | 2 | 0 | 1544 |
| `announce_cap` | `(2^101)` | 968839 | 1704072 | 4 | 2 | 0 | 1620 |
| `cancel_cap` | `(by the guardian)` | 962607 | 1700804 | 4 | 2 | 0 | 1544 |
| `set_cap` | `(2^101, announced)` | 966134 | 1701618 | 4 | 2 | 0 | 1544 |
| `restore_shares` | `(the LP)` | 871981 | 1674777 | 3 | 0 | 0 | 0 |
| `announce_upgrade` | `(the same Wasm)` | 961774 | 1705116 | 4 | 2 | 0 | 1648 |
| `cancel_upgrade` | `(by the guardian)` | 964978 | 1702108 | 4 | 2 | 0 | 1544 |
| `upgrade` | `(the same Wasm, announced)` | 1030221 | 1721807 | 4 | 2 | 0 | 1544 |
| `clear_sentinel` | `(by the guardian)` | 958972 | 1700972 | 4 | 2 | 0 | 1508 |
| `clear_sentinel, announced` | `(by the guardian, a sentinel announced)` | 1009777 | 1707561 | 4 | 2 | 0 | 1508 |
| `__constructor, stable` | `(KAT config)` | 1677698 | 3035814 | 6 | 2 | 0 | 1024 |
| `deposit, genesis, stable` | `(1e6 / 1e6)` | 2108170 | 1998570 | 13 | 7 | 184 | 2064 |
| `deposit, proportional, stable` | `(5e5 / 6e5 offered)` | 2125413 | 1949857 | 11 | 7 | 0 | 2064 |
| `swap_exact_in, stable, AtoB` | `(1 000 in)` | 3375737 | 1913974 | 10 | 6 | 0 | 1920 |
| `swap_exact_out, stable, BtoA` | `(1 000 out)` | 3801226 | 1908270 | 10 | 6 | 0 | 1920 |
| `quote_exact_in, stable` | `(1 000 in)` | 2781899 | 1649874 | 2 | 0 | 0 | 0 |
| `quote_exact_out, stable` | `(1 000 out)` | 2782531 | 1649874 | 2 | 0 | 0 | 0 |
| `withdraw, stable` | `(a third of the shares)` | 1748113 | 1897031 | 11 | 7 | 0 | 2064 |
| `pause, stable` | `()` | 950723 | 1694218 | 4 | 2 | 0 | 1024 |
| `unpause, stable` | `()` | 972910 | 1695024 | 4 | 2 | 0 | 1024 |
| `lower_cap, stable` | `(10^22)` | 950998 | 1694618 | 4 | 2 | 0 | 1024 |
| `set_cap, stable` | `(2^101, announced)` | 969339 | 1697390 | 4 | 2 | 0 | 1024 |

The 10% rule of E-001 §4.11 applies to every row from here on, on the entry point's own cost,
and it watches growth only: a row that falls, by more than 10% included, is never a regression
(amended 2026-09-13); the pull request notes it and the new row is the baseline.
The table is re-measured, and the README's copy replaced, on every change to `pool-soroban` or
to `pool-abi` (an event field, an error code, a codec byte, a dependency bump), in the pull
request that makes the change; this table stays the baseline the rule is applied against until
a pull request that changes the method (as the constructor did) declares a new one here. A
change that touches only the interface, the spec section of the Wasm and not its code section
(the literal byte types of 2026-09-13 were one), needs no re-measurement; the pull request
states instead that the code section is unchanged, byte for byte, and how that was checked
(amended 2026-09-14). Amended 2026-09-28 (ADR-012 amended, point 14): after version 7 (core
pull request 24) the `upgrade` row stands at +9.57 % over the table above, because it reads
and clears an announcement now, inside the rule but with no room left, and version 8 changes
what the `set_rate`, `set_roles` and `set_cap` rows measure in the same way (each reads and
clears an announcement, and `lower_cap`, the three announcements and the three cancels are
new rows). The version 8 pull request therefore declares a new baseline table here, in the
place of the table above: the reference measurement of the `wasm` job of the run on `main`
that merges it, copied from that job's log with the run id beside the table (E-009 §3.5),
never a laptop's measurement. The rule is applied to version 8 against that table from then
on; in the version 8 pull request itself the rows the announcement pattern does not touch
(the constructors, swaps, deposits, withdrawals, quotes, `refresh`, the views, the write-down
rows, `pause`, `unpause`, `suspend`, `resume`) are checked against the table above as for
any change, so that the re-declaration hides nothing. Version 8 merged on 2026-09-30 (core
pull request 27); its table from the merge run is not here yet and comes in core's next pull
request, so until then no row of version 8 has a baseline to be held against.

## 5. What is not built here

- The router contract: it calls this contract's swaps with a plan and a limit (E-006 §3.4); its
  own spec, E-008.
- The adapters: brand-a code, one contract per source, with the interface `observe() ->
  Option<(i128, u32, u64)>` (price, decimals, timestamp in E-003's SEP-40 shape) and the source
  rules of E-003 §3.4 and ADR-005 point 5. The pool only calls that function.
- A pool factory or registry: pools are deployed one by one by a human with a script that also
  makes the genesis deposit; the brand's indexer keeps the list.
- Transferable LP tokens (a SEP-41 token per pool): shares are an internal ledger; a token can
  wrap it later without changing this contract's accounting.
- Permissioned pools (brief §4.5): an allowlist check on swap and deposit is the extension point;
  this contract has none, and the extension is its own spec when an issuer commits.
- Fee-on-transfer and rebasing tokens: detected and refused, not supported.
- A multisig contract of our own: the roles are addresses, and an address can be a multisig
  contract or a multi-signer account; §3.6 requires one for governance before the caps are
  lifted, and nothing here needs to change for it.
- The "unaudited" warning, the deploy scripts, key handling: brand and ADR.
- The Solidity path.

## 6. Done when

- Crate `pool-soroban` exists in core, depends only on `pool-abi`, `band`, `stable-pool`,
  `stable`, `rate`, `math` and the pinned `soroban-sdk` (§3.7), builds for `wasm32v1-none`
  with the workspace lints, and its Wasm size and per-entry-point cost are in its README; crate
  `pool-abi` exists with the content of §3.7 and the codec tests of the contract (test 18 of
  the follow-up pull request) moved to it.
- Every entry point of §3.2, every error of §3.5 and every event of §3.5 exists and nothing
  beyond them is exported.
- Every test in §4 exists, was committed before the implementation, and passes; line coverage
  of the crate's own source is 100%, with the lines the SDK's macros generate (`contractimpl`,
  `contracttype`, `contracterror`) excluded from the measure, because they are not the crate's
  lines.
- The security reviewer has approved the pull request. Review points: that no entry point but
  `write_down` edits a state field and that one can only lower a reserve; the balance check on
  every transfer in; that `withdraw` cannot be blocked by any switch or rate state; the auth on
  every role call and on every transfer; the mainnet rule against the network id; the protocol
  split against the engines' invariants; the TTL handling; that no call can send tokens
  anywhere but to `to`, the caller, or the address governance names for protocol fees.
- The pull request that first implemented this spec was approved with five should-fix findings
  and ten questions; the answers are the amendments of 2026-09-11 (the write-down's net
  balance, `balance - ProtocolOwed`, whose prose was corrected on 2026-09-28 by ADR-012, the
  formula having been the rule; `set_rate`, the constructor, the Wasm bound, the announcement
  rules). A follow-up pull request that brings the crate in line with them, reviewed like any
  other, is a condition of this spec, and no mainnet pool is created before it is merged.
  Fulfilled: the follow-up pull requests of 2026-09-11 (the amendments, the `pool-abi` split)
  and 2026-09-12 (the write-down safety net, merged with E-008) were reviewed and approved; the
  condition is met and the pool contract may be deployed under the ADR of §3.6.
- This spec is copied to `core/spec/E-007-pool-contract.md` unchanged. Only then does work on
  E-008 start.
