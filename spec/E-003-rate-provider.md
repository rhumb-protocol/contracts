# E-003: Rate provider for NAV and exchange-rate pools

Date: 2026-09-08
Status: done; amended 2026-09-28 by ADR-012 from the exploit pass of I-001: the parenthetical
of gate 6 in §3.2 corrected to what the code does, and a remark on an observation dated after
`now` in `read`; no code changes
Target: core, crate `rate` (third component in the build order of ADR-002)
Depends on: E-001 (crate `math`, as merged in core). Nothing else.

## 1. Goal

One small, fully tested library that decides, for a pool whose price centre follows an external
reference value, which rate the pool may use right now, and when it must refuse to trade. The
reference value is the net asset value (NAV) of a yield-bearing token (USDY, USTRY, CETES, the
Centrifuge wrappers) or an exchange rate (EURC against USDC, ADR-004). The library takes
observations from whatever source the brand connects (a SEP-40 oracle contract or the issuer's
published price) and applies four gates to them: staleness, deviation bounds, market hours and a
human-controlled suspension. It holds no keys, reads no chain, and knows no token; the brand's
adapter feeds it observations, the concentrated pool (E-004) asks it for the rate.

Rate provider means: the component that tells the pool "one USDY is worth 1.0873 USDC right now",
or "do not trade". Staleness means the last known value is too old to trust. Deviation bound means
a limit on how much the value may move per update. Market hours means the period in which the
value is reliable at all.

## 2. Context and constraints

- On 22 February 2026 the YieldBlox pool on Blend lost about ten million dollars because its
  collateral price for USTRY came from an oracle that averaged trades on the Stellar DEX. The
  USTRY/USDC market there was almost empty, so one attacker's trades moved the reported price from
  about 1.06 to about 107 dollars, a hundredfold, and the pool lent against it. Nothing in the
  lending contract was broken; the price source was. The lesson for this spec: a rate must never
  come from trades, ours or anyone's, and a rate that moves in a way the asset cannot move must be
  refused even when the source is honest.
- The assets in the brief move in known ways. A NAV token accrues about 1 to 1.5 basis points per
  day and jumps at most a few basis points when the issuer publishes a new NAV; it never moves by a
  percent in a day. EURC/USDC moves like the euro against the dollar: fractions of a percent on an
  ordinary day, rarely more than two percent. These facts are the deviation bounds.
- Sources on Stellar today: SEP-40 oracle contracts (the Stellar oracle standard) from RedStone,
  which publishes USDY, USDC, EURC, PYUSD and the Centrifuge assets from off-chain sources and
  updates on a deviation threshold with a daily heartbeat, and from Reflector, whose feeds are
  volume-weighted averages of Stellar DEX trades. Issuers publish NAV daily (Ondo) or per business
  day (Etherfuse). A SEP-40 contract returns `lastprice(asset) -> Option<PriceData { price: i128,
  timestamp: u64 }>`, with `decimals()` fixing the scale and `resolution()` the update interval.
- Prices of bonds priced in a domestic market (CETES in Mexico, EU T-bills) are only reliable
  during that market's hours; outside them a source may repeat, extrapolate or drift. The brief
  (§4.1) asks for market-hours awareness, RedStone and Templar name it as the blocker for RWA
  collateral.
- The engine is chain-agnostic (rule 3 of the brain). This crate is pure `no_std` logic on top of
  `math`; it never calls an oracle. The brand's adapter (brand-a) reads the source and hands over
  an observation; the pool contract stores the state this crate returns. Anything that touches a
  chain lives in the adapter or in E-004.
- The pool's own protection against a bad rate is its narrow liquidity band (5 to 20 bp around
  the rate, brief §4.1): a rate that is wrong by less than the band costs LPs at most the band. The
  gates here exist to keep a rate that is wrong by more than the band from ever reaching the pool.

## 3. What gets built

### 3.1 Types (newtypes and plain structs, no implicit conversions, conventions of E-001)

| Type | Representation | Meaning | Valid range |
|---|---|---|---|
| `Rate` | `Wad` (E-001) | reference value of one unit of the asset in units of the quote asset | `10^12 ..= 10^24` (`1e-6` to `1e6`), enforced by `Rate::try_new` |
| `Timestamp` | `u64` | seconds since the Unix epoch | `0 ..= 2^40` (year 36 812), enforced by `Timestamp::try_new` |
| `SourceId` | `u8` | which configured source an observation came from | `0 ..= 3`; a config has at most 4 sources |
| `Observation` | struct | one reading: `rate: Rate`, `at: Timestamp` (the source's own time), `source: SourceId` | built by `Observation::new` |
| `DriftBand` | struct | expected movement of a NAV asset: `min_per_day: i32`, `max_per_day: i32` in basis points, `max_jump: Bps` per accepted update | `-100 ..= 100` per day, `min <= max`, `max_jump <= 500`, and `max_jump < max_step` of the config that holds it (§3.3) |
| `MarketHours` | struct | `utc_offset_minutes: i16` (`-840 ..= 840`), `open_minute: u16`, `close_minute: u16` (`0 ..= 1440`, `open < close`), `days: u8` (bitmask Monday..Sunday), `holidays: [u32; 16]` (days since epoch, `0` = unused) | enforced by `MarketHours::try_new` |
| `RateConfig` | struct | `max_age: u64` seconds; `max_closed_age: u64` seconds; `max_step: Bps`; `drift: Option<DriftBand>`; `hours: Option<MarketHours>`; `sources: u8` (`1 ..= 4`); `cross_check: Option<Bps>`; `grace: u64` seconds (how long `read` keeps serving after the second-to-last live source is lost) | enforced by `RateConfig::try_new`, see §3.3 |
| `RateState` | struct | `low: Rate`, `high: Rate` (the accepted interval, equal with one source), `updated_at: Timestamp`, `accepted_at: Timestamp` (chain time when accepted), `status: Status`, `last_by_source: [Option<Observation>; 4]` | produced by this crate, persisted and rebuilt field by field by E-004, re-checked on every use; `low <= high` always |
| `Side` | enum | `PoolBuysAsset` (the trader sells the asset to the pool), `PoolSellsAsset` (the trader buys it) | |
| `Status` | enum | `Active`, `Suspended(Reason)` | |
| `Reason` | enum | `StepTooLarge`, `OutsideDriftBand`, `SourcesDisagree`, `Manual` (staleness is not a suspension; `read` reports it as `RateError::Stale` and it clears itself with the next accepted observation) | |

- `raw()` on `Rate`, `Timestamp` and `SourceId`. The structs are plain data with public fields,
  because E-004 must persist a config and a state field by field and rebuild them from storage.
  `try_new` is the constructor that checks the ranges above, and since public fields cannot stop
  a caller from building a struct another way, `update` and `read` re-check every range and
  invariant of the config and the state they receive (including `low <= high`,
  `cross_check` present exactly when `sources >= 2`, `max_jump < max_step`,
  `max_closed_age >= max_age`) and return `BadConfig` before doing anything else. A config or
  state that fails these checks is never used. `RateState::genesis(rate, at)` builds the first
  state of a pool, set by a human at pool creation (E-004, ADR-002 point 6), with
  `low = high = rate` and an empty `last_by_source`.
- `Rate` bounds: `1e-6` to `1e6` cover every pair in the brief and ADR-004 (NAV tokens near 1,
  EURC/USDC near 1.1, CETES/MXNe near 1.2) with room for pairs with different decimals; anything
  outside is a broken source, not a price.

### 3.2 Functions

All functions are pure: same inputs, same output, no clock, no storage. `now` is the chain time the
caller passes in.

- `update(config: &RateConfig, state: &RateState, obs: Observation, now: Timestamp)
  -> Result<RateState, RateError>`: applies one observation and returns the new state. It never
  panics and never returns a state with a rate that failed a gate. Order of the gates:
  1. `obs.source` must be below `config.sources`, else `UnknownSource`.
  2. `obs.at <= now` and `obs.at > state.updated_at`, else `NotNewer` (a repeated or older reading
     changes nothing and is not an error for the caller to act on). Then, if the state is
     `Suspended`, `update` returns `Ok(state)` unchanged and stores nothing; the remaining gates
     run only on an `Active` state.
  3. Market hours (§3.3): if `config.hours` is set and `obs.at` falls outside them, the observation
     is ignored: `Ok(state)` unchanged. It is not stored, so it can never count in a cross-check.
  4. Step bound: the change of the observation against the accepted interval is
     `obs.rate - state.high` when above it, `state.low - obs.rate` when below it, and `0` inside
     it. The observation is accepted by this gate iff
     `wad_div_floor(change, edge) <= Wad::from_bps(config.max_step)`, where `edge` is the
     interval edge the change is measured from; one unit of `Wad` above is refused. Otherwise
     the state becomes `Suspended(StepTooLarge)` and the function returns `Ok` with that state.
     Suspension is a state, not an error: the caller must store it. A suspending observation is
     never stored in `last_by_source`.
  5. Drift band (NAV assets, `config.drift` set): let
     `days = floor((obs.at - state.updated_at) · 10^18 / 86 400)`, a `Wad` number of days; the
     signed change of gate 4, expressed in basis points scaled by `10^18` and computed in `i128`
     throughout, must lie within `[min_per_day · days - max_jump · 10^18,
     max_per_day · days + max_jump · 10^18]`, both ends inclusive, else
     `Suspended(OutsideDriftBand)`.
  6. Cross-check (`config.sources >= 2` and `config.cross_check` set): the observation is stored
     in `last_by_source`. A source is live when its own last observation is fresh by the same
     rule `read` applies to the rate: `now - at <= max_age`, or `<= max_closed_age` when
     `config.hours` is set and no session has opened after `at`. Liveness is always judged
     against `now`, never against another source, and a source older than its bound simply
     drops out; a stored observation with `at` after `now` (a chain clock that went backwards)
     is not live either. The interval advances only when the new
     observation and at least one other live source agree, that is, the rates of the new
     observation and of every live source differ pairwise by at most `cross_check` basis points,
     each pair tested as in gate 4 with `cross_check` as the bound and the lower rate as the edge.
     With three or four configured sources the whole live set must agree, not just one pair.
     The accepted interval is then `low = min`, `high = max` over the new observation and the
     live sources, `updated_at = max(at)` over that same set (corrected 2026-09-28, ADR-012:
     the partners of the set are those live at `now`, and the new observation's own liveness
     at `now` is not checked before it confirms them, so `updated_at` is the newer of the two
     times and the pair is then served until `lost_at + grace`; nothing is ever served older
     than `max_age + grace`, and the rate carries the time of its newest ingredient), and
     `read` hands out the edge that is worse for the
     trader (§3.3).
     If any live source disagrees by more, the state becomes `Suspended(SourcesDisagree)` and the
     observation is not stored.
  7. Otherwise (one configured source) the state becomes `Active` with `low = high = obs.rate`,
     `updated_at = obs.at`, `accepted_at = now`, and the observation stored in `last_by_source`
     as in gate 6, so that the state has one shape whatever the number of sources.
  A `Suspended` state stays suspended through every later `update`; only `resume` clears it.
- `read(config: &RateConfig, state: &RateState, now: Timestamp, side: Side)
  -> Result<Rate, RateError>`: the rate the pool may use now for a trade on that side. Returns
  `Err(Suspended(reason))` when suspended, `Err(Stale)` when the rate is stale (§3.3), otherwise
  `Ok(state.low)` for `PoolBuysAsset` and `Ok(state.high)` for `PoolSellsAsset`: the pool values
  what it takes in low and what it gives out high, so within the tolerance of the cross-check
  the worse edge for the trader is always used. Freshness is wall-clock time and nothing else:
  `age = now - updated_at`, and no event restarts that clock. Stale means `age > bound`, where
  `bound = max_age`, except that `bound = max_closed_age` when `config.hours` is set and the
  market has been closed for the whole time since the session in which the rate was accepted
  ended, that is, no session has opened after `updated_at`. As soon as a session opens, `max_age`
  applies to the full age: a rate from Friday is refused at the opening on Monday until the source
  publishes again, and a NAV three days old is never handed out inside market hours. Without
  market hours `max_closed_age` is unused. With `config.sources >= 2`, `read` also counts the
  live sources as gate 6 does; when fewer than two are live it returns `Err(SourceLost)` once
  `now` is more than `config.grace` past the moment the second-to-last source stopped being live
  (`lost_at`, the end of that source's liveness bound, that is the last second at which it was
  live: `at + max_age` for a source that ages out, and `opening - 1` for a source whose bound
  falls from `max_closed_age` to `max_age` because a session opens, so that its grace window
  runs to `opening - 1 + grace` and `read` returns `SourceLost` from `opening + grace`).
  Between `lost_at` and `lost_at + grace`
  the last confirmed rate is served, and for that window alone the staleness bound is raised by
  `grace`: the rate was at most `bound` old at `lost_at` (its `updated_at` is the newest member
  of a set that was live then), so it is served throughout the window and refused with
  `SourceLost` at its end. The grace period bridges a source that skips one update; a pool does
  not run on one source for longer than that, because one source is what YieldBlox had. When
  fewer than two observations are stored at all, there is no `lost_at` and `read` returns
  `SourceLost` at once: a two-source pool serves nothing after `genesis` and after `resume`
  until two sources have published and agreed. A stored observation whose `at` lies after
  `now` is not live, and its liveness end still counts as a candidate `lost_at` (noted
  2026-09-28, ADR-012, from the exploit pass; accepted, no code): Soroban ledger time never
  goes backwards, and a source that publishes a timestamp in the future is the adapter's to
  refuse under §3.4. The human's rate in `resume` is the reference
  the next observations are gated against, not a rate to trade on by itself. Errors of `read`
  take precedence in this order: `Suspended`, `Math(Overflow)` from the age subtraction,
  `SourceLost`, `Stale`; the first that applies is returned.
  `read` never mutates; staleness and liveness are judged at every read so that a pool cannot
  trade on a rate that went stale between updates. A `now` before `updated_at` (a chain clock
  that went backwards) makes the age subtraction fail and `read` returns `Err(Math(Overflow))`;
  every `Err` refuses the trade, so a clock anomaly never yields a rate.
- `resume(state: &RateState, rate: Rate, now: Timestamp) -> RateState`: clears a suspension by
  setting `low = high = rate`, `updated_at = accepted_at = now`, and an empty `last_by_source`
  (observations from before the suspension are not trusted as partners afterwards). This is the
  only way out of `Suspended`, and E-004 exposes it only to the guardian or governance (a human
  signs, ADR-002 point 6). It is deliberately not gated: the human takes responsibility for the
  rate they set. With two or more sources the resumed pool still serves nothing until two
  sources have published and agreed (§3.2, `read`); with one source it serves the human's rate
  until the next observation.
- `suspend(state: &RateState) -> RateState`: sets `Suspended(Manual)` on an `Active` state; for
  the guardian's pause of a single pool's rate. On an already suspended state it keeps the
  existing reason and changes nothing, so the original cause stays visible.
- `Observation::from_sep40(price: i128, decimals: u32, timestamp: u64, source: SourceId)
  -> Result<Observation, RateError>`: converts a SEP-40 `PriceData` to an observation:
  `price > 0`, `decimals <= 18`, the rate is `price · 10^(18 - decimals)` through checked
  arithmetic and must satisfy the `Rate` range; otherwise `BadObservation`. Timestamps from SEP-40
  are floored to the feed's resolution by the oracle; that is fine, the gates use them as they are.
- `enum RateError { Math(MathError), UnknownSource, NotNewer, BadObservation, Stale, SourceLost,
  Suspended(Reason), BadConfig }`, with `From<MathError>`.

### 3.3 Gate semantics, fixed here

- Staleness. `max_age` is the longest a pool may trade on an unchanged rate; `max_closed_age`
  the longest it may carry a rate across a closed period (weekend plus holiday) while the market
  stays closed, and `max_closed_age >= max_age` is a config rule. Both are per asset, both are
  measured from the observation's own time, and neither is ever reset by a session opening (§3.2).
  Reference values the brand starts from, to be confirmed per asset in an ADR before the pool is
  created:

  | Asset class | Source cadence | `max_age` | `max_closed_age` | `grace` | `max_step` | drift band | `hours` |
  |---|---|---|---|---|---|---|---|
  | NAV token, daily NAV (USDY, Centrifuge) | daily | 30 h | 30 h | 2 h | 10 bp | `0 ..= 5` bp/day, `max_jump` 5 bp | none |
  | NAV token, business-day NAV in a domestic market (CETES, USTRY) | per business day | 30 h | 100 h | 2 h | 10 bp | `-1 ..= 5` bp/day, `max_jump` 5 bp | issuer's market |
  | Exchange rate (EURC/USDC, rate = EURC/USD) | on 0.5 to 1% deviation, daily heartbeat | 26 h | 100 h | 2 h | 300 bp | none | none |

  `max_age` above the daily cadence, not below it, because a quiet day produces no update; a
  tighter value would suspend a healthy pool every night.
- Step bound. It is applied to every observation, also inside a drift band, and is the first
  gate after freshness so that a hundredfold move is refused before anything else is computed.
  The YieldBlox move (1.06 to 107) is a step of about one million basis points against any bound
  in the table.
- Drift band. Only for assets whose value accrues. `min_per_day` may be slightly negative to
  allow fee days and rounding; `max_jump` covers the day the issuer publishes a new NAV. A
  reading that accrues faster than the band is refused even if it is true: the pool then waits
  for a human, which is the correct outcome for a bond token whose value jumped 20 bp in a day.
  `max_jump` must be strictly below `max_step` (`RateConfig::try_new` returns `BadConfig`
  otherwise): with `max_jump >= max_step` and `min_per_day <= 0` every change that passes the
  step bound also lies inside the band, and the band can never fire. With the table above a
  same-day reading may move at most 5 bp by the band and 10 bp by the step bound, so the band is
  the binding gate for a NAV asset and the step bound is the backstop for a broken source.
- Market hours. Windows are in the market's local time with a fixed UTC offset; daylight-saving
  changes are a config change made by a human, as is the holiday list. The arithmetic, fixed
  here: `local = at + utc_offset_minutes · 60` as a signed value; `local_day = local div 86 400`
  and `second_of_day = local mod 86 400` with Euclidean division, so that negative local times
  still give a day and a second in range; the window is open iff `open_minute · 60 <=
  second_of_day < close_minute · 60`; the weekday is `(local_day + 3) mod 7` with bit 0 of
  `days` meaning Monday; holidays are local day numbers, matched against `local_day`. A session
  is open at `at` iff the window, the weekday bit and the holiday list all allow it. Outside the window, new
  observations are not applied and the last accepted rate stays usable until its age reaches
  `max_closed_age`, provided no session has opened in the meantime. The moment a session opens,
  `max_age` applies to the full age of the rate: Friday's NAV is traded through the weekend but
  not on Monday; at the opening the pool waits, refusing swaps and deposits, until the source
  publishes Monday's value. This is the price of market-hours assets and it is deliberate: a
  bond's value does not change while its market is closed, and a bond that reprices at the
  opening must not be traded at Friday's price.
- Cross-check. Two sources of different kinds (an oracle and the issuer's published NAV) are the
  brief's answer to a single manipulated or broken source. When two are configured, one source
  alone never moves the rate. Two sources that agree within the tolerance rarely agree exactly;
  the crate keeps both edges and the pool uses, per trade, the edge that is worse for the trader:
  the lower rate when the pool buys the asset, the higher when it sells. The difference is at most
  the tolerance and is paid by the trader, never by the LPs; no trader can pick the better edge
  by choosing a direction.
- A dead source is not a partner. A source counts only while its own last observation is fresh
  by the rule of §3.2, `max_age`, or `max_closed_age` while the market has stayed closed since
  that observation; after that it drops out of the cross-check and out of `read`'s count. With
  market hours this means that a two-source pool trades through the weekend on the last
  confirmed NAV, as a single-source pool would: both sources stay live while the market is
  closed, and both go stale together at the next opening until they publish again. That is the
  intended behaviour; a bond's value does not change while its market is closed, and a rule that
  refused every weekend would only make the pool unusable without making it safer. Only when
  a session has opened since a source's last observation does that source drop out after
  `max_age`. The
  accepted rate carries the time of its newest ingredient; the older one had to be live to
  count, so a confirmation is never older than the liveness bound. When only one live source
  remains, `read` keeps serving the last confirmed rate for `grace` (reference: two hours),
  with the staleness bound raised by `grace` for that window, and then refuses with
  `SourceLost` until a second source is live again. There is no suspension and no human step:
  the pool resumes by itself when the source returns, because nothing was wrong with the rate,
  only with its confirmation.
- Two sources are a condition for mainnet. A pool whose config has `sources = 1` may run on
  testnet only; it is not eligible for the capped beta of ADR-002 either, because a single source
  is exactly what YieldBlox had. The crate allows `sources = 1` so that testnet pools and tests
  can run; E-004 refuses to create a pool with one source on a mainnet deployment, and the brain
  records the two sources per asset in an ADR before the pool is created.
- EURC/USDC uses the EURC/USD price as its rate, not a separate EUR/USD rate: EURC is the asset
  in the pool and its own price already contains the euro rate and any premium or discount of the
  token. Today only one SEP-40 source publishes EURC/USD on Stellar, so EURC/USDC waits until a
  second source of a different kind exists; it is not a launch pool.
- Suspension. Every automatic suspension carries its reason so that the indexer (brand-a) can
  report it and the guardian can see why. There is no automatic resumption: a rate that was refused
  once is not trusted again because the next reading looks fine.

### 3.4 What the consumers must do (requirements on E-004 and the brand adapter)

- E-004 calls `read` in every swap and every deposit and rejects the operation on any `Err`. A
  swap passes the side of the trade; a deposit passes `PoolBuysAsset`, so that the asset an LP
  brings in is valued at the low edge and shares are never minted against the high one.
  Withdrawals never call `read`: LPs can always leave, also on a stale or suspended rate.
- E-004 stores the `RateState` this crate returns and nothing else; it never edits fields.
- E-004 exposes a permissionless `refresh` that reads the configured sources through the brand's
  adapter and calls `update` with what they return; nobody hands the pool a rate by hand except
  through `resume`. Anyone may call `refresh` (an observation that fails a gate cannot harm the
  pool, and keeping the rate fresh must not depend on one keeper). `suspend` is exposed to the
  guardian, `resume` to the guardian and governance. Every status change emits an event with the
  reason.
- E-004 uses `state.updated_at` for the temporary fee bump around NAV updates (brief §4.1);
  this crate only records the time.
- The brand's adapter never derives an observation from trades on any venue, including our own
  pools and the Stellar DEX. Allowed sources are push oracles with off-chain inputs (RedStone
  SEP-40 feeds) and the issuer's published NAV or redemption price. Reflector feeds are not
  allowed for these assets while they are trade-derived. This rule is enforced by review of the
  adapter, not by code; it is written here so that the reviewer has something to point at.

### 3.5 Code hygiene

- `#![no_std]`, the workspace lints of core (the `[workspace.lints]` table of its `Cargo.toml`, enforced by the clippy command in its README), no floating point, no unchecked arithmetic, no
  `unwrap`, `expect`, `panic` or indexing outside tests. Fixed-size arrays only; no allocation.
- The only dependency is `math`. Test-only dependencies: `proptest`.
- No code is ported for this crate; the file headers say so. Where a rule is taken from an
  existing design (Chainlink's heartbeat and deviation model, Balancer's rate-provider interface),
  the doc comment names it.

## 4. Reference tests (written before the code)

Property tests, at least 10 000 cases per property, seeded so failures are reproducible. The
generator draws configs from the valid ranges of §3.1 and observation sequences of up to 64 steps
with timestamps that increase, at random intervals from one second to ten days.

1. Determinism: replaying the same sequence from the same genesis gives the same final state, on
   host and on Wasm (test 12).
2. Gates are total: for every config, state and observation, `update` returns `Ok` or one of the
   named errors, never panics; `read` likewise. Includes observations at every range edge (`Rate`
   bounds, `Timestamp` bounds, `decimals = 0` and `18` in `from_sep40`, `price = 0`, negative
   price).
3. Step bound: an observation whose relative change exceeds `max_step` by one unit of `Wad`
   suspends; one exactly at `max_step` is accepted. Same for the drift band edges and the
   cross-check tolerance.
4. Suspension is absorbing: after any `Suspended`, every further `update` leaves the status
   unchanged whatever the observation, and `read` returns `Err(Suspended)`; only `resume` returns
   to `Active`.
5. Staleness is exact and never resets: `read` at `updated_at + max_age` succeeds and at
   `updated_at + max_age + 1` returns `Stale`; with market hours and no session opened since,
   the same around `updated_at + max_closed_age`; and for a rate accepted before a closed period,
   `read` succeeds one second before the next opening (age below `max_closed_age`) and returns
   `Stale` at the opening whenever the age is above `max_age`. A generated schedule with a rate
   accepted in one session and a `read` in a later session never succeeds with an age above
   `max_age`, for any offset, day mask or holiday list.
6. Market hours: observations outside the window never change the interval or `updated_at`; the
   window test covers the day bitmask, the holiday list, the UTC offset (including negative
   offsets and windows that cross midnight in UTC), and the minute of opening and closing,
   against an independent calendar reference. The generator draws the timestamps of this
   property from the same years as the holidays it draws, so that the holiday branch is reached
   by the property test and not only by the known-answer test.
7. Cross-check: with two sources, a single source never changes the interval; two agreeing
   sources set `low = min` and `high = max`, `updated_at = max(at)`, `read` returns `low` for
   `PoolBuysAsset` and `high` for `PoolSellsAsset`, and `high - low` never exceeds the tolerance;
   two live sources further apart than the tolerance suspend; a source whose last observation is
   older than its liveness bound at `now` is not live and neither confirms nor contradicts. With
   market hours, two sources that both published on Friday are live through the weekend and
   `read` serves the confirmed rate until the next opening; both sources drop out together at
   the opening, so `read` refuses `Stale` during the grace window (the age is above
   `max_age + grace`) and `SourceLost` from `opening + grace` on, until both publish again.
   With one
   configured source, `low == high` after every update. Liveness: with two sources of which one
   stops publishing, `read` succeeds with the last confirmed rate until `grace` after the end of
   that source's liveness bound and returns `SourceLost` one second later, whatever the other
   source does in between; for a source that drops out at an opening, `lost_at` is
   `opening - 1` and `SourceLost` starts at `opening + grace`; it succeeds again as soon as the lost source publishes an
   observation that passes the gates and agrees. Outside the grace window the rate served is
   never older than its bound measured from the newer observation of the confirming pair.
8. Drift band: a sequence of daily NAV readings accruing at any constant rate inside the band is
   accepted for a year of steps; the same sequence with one reading outside the band suspends at
   that reading and not before.
9. `from_sep40` round trip: for every `decimals` in `0 ..= 18` and every price in range,
   `from_sep40(price, decimals, ..).rate` equals `price · 10^(18 - decimals)` exactly.

Known-answer tests:
10. The YieldBlox case: genesis rate `1.06`, observation `107` one hour later, under the exchange-
    rate config of §3.3 and under the NAV config; both suspend with `StepTooLarge`, and `read`
    refuses from then on. A second observation of `1.061` after that does not resume.
11. The brief's assets, one year each with the reference configs of §3.3 and `sources = 1` (the
    table names no sources; the two-source rules are covered by test 7): USDY accruing 1.2 bp
    per day with a 4 bp jump on the first business day of each month, accepted throughout; CETES
    with Mexican hours (09:00 to 15:00 local, UTC offset `-360`, Monday to Friday, holidays
    2026-02-02, 2026-03-16 and 2026-11-16) accepted throughout, a reading on a Sunday ignored,
    and Friday's rate refused as `Stale` at Monday's opening until Monday's reading arrives;
    EURC/USDC moving from `1.08` to `1.17` in steps of at most 0.9% accepted, a single 3.5% step
    suspended. Each case also checks the stale boundary after the last accepted reading.
12. Determinism and cost: the property suite and the known-answer tests run on the host and,
    through a wrapper contract `crates/rate-soroban-test` built like E-001's, compiled to
    `wasm32v1-none`; results are byte-identical. The wrapper passes `RateConfig` and `RateState`
    as fixed-size byte arrays, every field a raw little-endian integer in field order with a
    presence byte for each `Option`; the same encoder runs on the host side, and both decoders
    reject payload bytes behind a cleared presence byte. A benchmark table in `crates/rate/README.md`, same
    method and columns as E-001 §4.12, with one row for `update` on each gate outcome, one for
    `read`, and one for `from_sep40`. The first table is the baseline; the 10% rule of E-001
    §4.11 applies.

## 5. What is not built here

- Any oracle, any adapter, any contract that reads a SEP-40 feed or an issuer API. The adapter is
  brand-a code; its interface to this crate is `Observation`.
- The choice of sources per asset (brief §14, open item). This spec fixes what a source may be
  (§3.4), that mainnet needs two of them (§3.3), and the config each asset starts from (§3.3);
  the brain records the two sources per asset in an ADR before the first NAV pool is created.
- The guardian: who holds the key, how it is held (brief §14). The crate only defines what
  `suspend` and `resume` do.
- The temporary fee bump around NAV updates and the price band of the pool (E-004).
- Averaging of any kind (TWAP, VWAP, median over time). A time average of a manipulated thin
  market is still manipulated, as YieldBlox showed; the defence here is bounds and a second source,
  not smoothing.
- Rates for volatile pairs. The bounds in §3.3 are for assets that move by basis points; a pair
  that moves by percents in minutes is out of scope by ADR-001.
- The Solidity path. This spec and its vectors are the reference for it.

## 6. Done when

- Crate `rate` exists in core, depends only on `math`, builds for the host and for `wasm32v1-none`
  with the workspace lints, and allocates nothing.
- Every type and function in §3 is implemented, documented with its gate order and error cases,
  and nothing beyond §3 is public.
- Every test in §4 exists, was committed before the implementation, and passes on host and Wasm;
  line coverage of the crate is 100%.
- The benchmark table is committed in the crate's README.
- The security reviewer has approved the pull request. Review points: the gate order (freshness
  before bounds, bounds before cross-check), the absorbing suspension, the staleness boundaries,
  the market-hours arithmetic across midnight and offsets, integer conversion in `from_sep40`,
  and that no path sets a rate that failed a gate.
- This spec is copied to `core/spec/E-003-rate-provider.md` unchanged. Only then does work on
  E-004 start.
