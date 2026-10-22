# A-008: The bug bounty

Date: 2026-09-26
Status: in progress since 2026-10-19; §3.4 amended 2026-10-12 by ADR-016 (the ceiling as
funded bounds the cap); §3.2 and §6 amended 2026-10-19 by the owner's decision (the own
programme on the self-hosted route of §3.3 from the first day, Immunefi added when its answer
allows a listing; ADR-014 §8 point 29); §7.1 filled 2026-10-22: `<funded>` is $2 500, the
bridge of ADR-014 §8 point 29 (amended 2026-10-19), which the owner sets apart before the
beta (the go/no-go list of `strategy/state.md` §3); `<contract ids>` and `<link>` stay open
until the first mainnet pool exists and the list of known issues has an address; filled the
same day: `<cap>` $10 000 per pool (ADR-014 §1 and §9), `<ceiling>` $3 000 per award (§3.4,
unchanged by ADR-016), `<route>` `security@rhumb.finance` and GitHub's private vulnerability
reporting (§3.2 and §3.3, decided 2026-10-19); the same day `<contract ids>` and `<link>`
replaced by two interim sentences in the text of record (no contract in scope yet, the ids
listed from the first pool on; the list of known issues published before the first pool),
each replaced at the first mainnet pool in a commit of its own, and §3.1 knows no contract
in scope until then; §4 test 1 amended the same day: the page carries §7.1 byte for byte,
each `SECURITY.md` the route, the five promises in short and the link to the page, not the
text of §7.1 (as A-006 §3.12)
Target: the published rules of Rhumb's bug bounty (§7.1, the text of record), in
`rhumb-protocol/app` as the page `/security`; in `rhumb-protocol/contracts`, `sdk` and `app`
a `SECURITY.md` with the route of §3.3, the five promises of §3.3 in short and the link to
`https://rhumb.finance/security` (§4 test 1, A-006 §3.12); the reporting route (§3.3); the triage runbook in `governance/reports/bounty/`
with one record per report; and, if §3.2 chooses it, the program on Immunefi. Before the capped
mainnet beta: ADR-002 point 1 requires the bounty from its first day.
Depends on: ADR-002 points 1 and 6 (a public bounty from day one; humans sign), ADR-010 (awards
from the beta reserve under a ceiling per award, and the bridge until the reserve can carry
it), ADR-009 (the Rhumb repositories become public at the start of the beta), E-007 §3.2 (the
guardian's pause, which never blocks withdrawals), A-006 (the app, which carries the page).
Blocks: the start of the capped mainnet beta.

## 1. Goal

Pay outsiders to find what is wrong before an attacker does, from the first day of the beta,
with rules a researcher can read in five minutes: what is in scope, what each severity pays and
from what, how to report, what we promise in return. The award ceilings follow the funds at
risk, which ADR-002's cap keeps small, and grow with the caps; every award is paid from money
that exists (ADR-010) and signed by a human.

Bug bounty means: a published promise to pay whoever reports a vulnerability privately under the
rules. Funds at risk means: what an attacker could take or freeze with the reported defect.
Severity means the class of harm, by Immunefi's classification (§7.2), the one most Web3
researchers work with.

## 2. Current state (2026-09-26)

- No bounty exists. No Rhumb contract is on mainnet; the three testnet pools hold test tokens.
- ADR-002 caps every mainnet pool at $10 000 in the contract, three pools at the start: the
  funds at risk are at most $30 000 until a cap is raised.
- The beta reserve starts at zero (the protocol fee switch is off, brief §4.3); ADR-010 lets it
  pay awards and nothing else, and bridges the start with own funds set aside or a platform that
  needs no prefunding.
- Research `strategy/research/bug-bounty.md` (2026-09-26), copied where needed into §7: Immunefi
  takes 10 % of each payout and lists a Soroban program (OpenZeppelin on Stellar: critical 10 % of
  funds at risk, $5k floor, $25k cap); whether it requires a funded vault was not found. Blend
  and Aquarius run their bounties themselves. SEAL's Safe Harbor has no confirmed Stellar route.

## 3. What gets built

### 3.1 Scope

In scope, on mainnet, at the contract ids of `chain-config/mainnet.toml` once deployed (the rules
page lists them, and a contract that is not listed is not in scope):

- The pool contract (both kinds), the router, the issuer adapter and the RedStone adapter:
  every severity of §7.2.
- The keeper and the deploy scripts in `rhumb-protocol/contracts`: a defect by which a third
  party could obtain or misuse the keeper's hot key, or make the keeper post a wrong price that
  the adapter accepts. Rated by its effect on the contracts (a wrong accepted price that lets
  funds be taken is critical).
- The SDK and the app: a defect by which the transaction a user signs differs from the one the
  app showed them, or moves funds the user did not choose. High at most, critical when it takes
  funds without a further user error.

Out of scope, stated on the page:

- Testnet, and every contract not listed.
- Third parties: the token contracts and their issuers, RedStone, the issuers' price
  publications, the Stellar network and its RPC providers, Soroswap and other venues, wallets.
- The known issues, listed on the page and kept current: the findings of the published audit
  and pre-audit reports (I-001, I-002), and the behaviour the specs describe as intended (a
  same-asset swap refused, the write-down of a token whose balance fell, a band pool refusing
  to trade on a stale or suspended rate, the cap, the guardian's pause).
- The indexer, the quote API and the numbers they serve (they hold no funds and sign nothing).
- Front-running or sandwiching that the slippage bounds a user sets already limit; phishing,
  social engineering, attacks on our staff or accounts; denial of service by volume; findings
  that need a compromised governance or guardian key; anything found by testing on mainnet
  against funds that are not the researcher's own.

### 3.2 The platform

Decided 2026-10-19 by the owner: the bounty is Rhumb's own programme, self-hosted as Blend and
Aquarius run theirs, on the reporting route of §3.3 (`security@rhumb.finance` and GitHub's
private vulnerability reporting), live on the first day of the beta. Immunefi was asked and had
not answered by that day; the answer no longer decides the platform and no longer holds the
acceptance of ADR-014 (§8 point 29). When the answer comes and allows a listing without a
funded vault for a programme of this size, the listing is added beside the own programme as an
amendment of this section, with its report form as a second route in §3.3; the rules of §7.1
and the awards of §3.4 are the same either way. Immunefi's reason stands for that later
listing: it is where Web3 researchers look, it runs a Soroban program already, and its 10 % is
taken from a payout that happened, which fits a reserve that starts at zero. The commission,
when there is one, is paid beside the award, from the same source as the award, and counts
against the ceiling of that award (§3.4). Until 2026-10-19 this section read: "Proposed:
Immunefi, if its listing needs no funded vault for a program of this size; otherwise
self-hosted. The owner asks Immunefi before the beta and the answer decides."

### 3.3 The reporting route

- On Immunefi: its report form, the only route for an award there.
- Self-hosted: the address `security@rhumb.finance` (a mailbox the owner creates before the
  beta, read by the owner) and GitHub's private vulnerability reporting, switched on in every
  public Rhumb repository. The page gives both; no report is taken in a public issue, a chat or
  a social network, and one that arrives there is answered with the route and nothing else.
- Our promises, on the page: an acknowledgement within 48 hours; a first assessment with the
  severity within 7 days; a fix, or the guardian's pause while one is made, as fast as the harm
  requires; coordinated disclosure after the fix or after 90 days, whichever is first, unless the
  researcher agrees otherwise; no legal action against a researcher who followed the rules
  (the safe harbour paragraph of §7.1).
- A critical or high report starts the guardian's runbook at once: the owner decides within
  the hour of reading it whether to pause the affected pools (deposits and swaps stop,
  withdrawals continue, E-007 §3.2).

### 3.4 Awards and the ceiling per award

Awards follow the funds at risk, and never exceed the ceiling of their severity at the time of
the report (§7.3):

| Severity | Award |
|---|---|
| Critical | 10 % of the funds at risk, at least $1 000, at most the critical ceiling |
| High | $1 000 |
| Medium | $500 |
| Low | $100 |

The critical ceiling is 10 % of the sum of the caps of the pools in scope on the day of the
report: $3 000 with three pools at $10 000. It grows with every cap increase ADR-002 allows, and
the page states it as a sum in dollars, updated in the same commit as the cap change. The
commission of §3.2, when there is one, is within the ceiling. An award is paid in USDC on
Stellar to an address the researcher names, after KYC when the platform requires it.

Amended 2026-10-12 by ADR-016 §5: the ceiling binds the cap as well as the award. A step on
the cap curve of ADR-016 is allowed only when the sum this page states as funded (the reserve
of ADR-010 plus the owner's bridge, read on the day of the announcement) is at least 10 % of
the pool's new cap, so `new cap <= 10 × funded critical ceiling`; the step's line in
`reports/caps.md` quotes the funded sum. This is the first half of direction §6 point 6, the
bounty's size tied to the cap; the rest waits for its own ADR.

Funding (ADR-010): an award is paid from the beta reserve, never above its balance at the time;
until the reserve holds one critical ceiling, from the own funds set aside for the bridge, whose
amount the owner sets before the beta and the page states ("awards up to $... are funded"). A
report whose award exceeds what is funded is paid in full as soon as the funds exist, in the
order the reports were confirmed, and the page says so. Every payout is signed by a human and
recorded (§3.5).

### 3.5 Records

`governance/reports/bounty/<yyyy>-<nn>.md`, one per report received, written by the owner or by
the brain from the owner's notes: the date received, the route, the severity claimed and
assessed, the scope item, the pause decision and its time, the fix (pull request, deploy
record), the award, its source (reserve or bridge), the transaction hash of the payout, and the
disclosure date. No researcher's personal data beyond the name or handle they want published.
The monthly report of the reporter counts reports and awards from these files.

### 3.6 SEAL Safe Harbor

Not adopted at the start: no Stellar route was confirmed. The owner asks SEAL whether Soroban
contracts can be named in scope, and a positive answer is an amendment of this spec, with the
72-hour return and the 10 % recovery bounty weighed against ADR-010 first.

## 4. Verification

1. The page `/security` carries the text of §7.1 byte for byte, and from the first mainnet
   pool on the contract ids equal to `chain-config/mainnet.toml`; a test of the app compares
   the page's contract ids with the constant of the SDK. Amended 2026-10-22, as A-006 §3.12
   states it: each `SECURITY.md`, in `app`, `sdk` and `contracts`, carries three parts and
   not the text of §7.1: the route of §3.3 (`security@rhumb.finance` and GitHub's private
   vulnerability reporting), the five promises of §3.3 in short, and the link
   `https://rhumb.finance/security`. The test checks the page literally against §7.1 and each
   `SECURITY.md` on those three parts. Until 2026-10-22 this test asked each `SECURITY.md`
   to carry the text of §7.1 byte for byte as well.
2. A test report sent by the owner through each route arrives, and the acknowledgement goes out
   within the promise.
3. The first record under `reports/bounty/` is the test report, marked as a test.

## 5. What is not built here

- A bounty on testnet, or on the indexer and the quote API.
- A contest (research `audit-firms.md` §5) or an audit: other routes.
- Awards in a token; there is none (brief §11).
- Any spending from the reserve beyond awards (ADR-010).

## 6. Done when

- The platform is decided (§3.2): met 2026-10-19 by the owner's decision for the own
  programme on the self-hosted route; a later Immunefi listing is an amendment, not a condition.
- The rules of §7.1 are live on `/security`, with the funded amount and the ceiling filled
  in, on the first day of the beta; the three `SECURITY.md` files carry the route, the five
  promises of §3.3 in short and the link to `https://rhumb.finance/security` (§4 test 1,
  A-006 §3.12).
- The reporting route works end to end (§4 test 2).
- `governance/reports/bounty/` exists with the test record.

## 7. Appendix: the data this spec needs

### 7.1 The rules (the text of record)

> **Rhumb bug bounty.** Rhumb's contracts are unaudited and run in a capped beta: every pool has a
> hard cap of $10 000 and a guardian can halt deposits and swaps; withdrawals are always possible.
> We pay for vulnerabilities reported privately under these rules.
>
> **In scope:** the contracts listed below on Stellar mainnet; the keeper and deploy scripts where
> a defect exposes the keeper's key or makes a wrong price reach a pool; the SDK and the app where
> a user signs something other than what was shown. No contract is in scope yet; the mainnet
> contract ids are listed here from the first pool on.
>
> **Out of scope:** testnet; third-party contracts, oracles, issuers, wallets and the Stellar
> network; the known issues (the list of known issues is published here before the first
> pool); the indexer and quote API; attacks that need a compromised
> governance or guardian key; phishing and social engineering; denial of service by volume; any
> test on mainnet against funds that are not your own.
>
> **Awards:** critical 10 % of the funds at risk, at least $1 000, at most $3 000; high $1 000;
> medium $500; low $100. Severity by Immunefi's classification v2.3. Paid in USDC on Stellar.
> Awards up to $2 500 are funded today; a larger award is paid in full as soon as funds exist,
> in the order reports were confirmed.
>
> **Report:** to `security@rhumb.finance`, or through GitHub's private vulnerability reporting in
> any public Rhumb repository. Never in a public issue or chat. We acknowledge within 48 hours and assess
> within 7 days, and disclose together with you after the fix or after 90 days.
>
> **Safe harbour:** if you act in good faith, stay within scope, do not take or keep funds, do
> not harm users, and report to us first, we will not pursue legal action against you for your
> research.

Filled 2026-10-22: `<funded>` is $2 500, the bridge of own funds the owner sets apart
before the beta. `<contract ids>` and `<link>` stay open: the first is filled when the
first mainnet pool exists, the second when the list of known issues has an address.
Replaced 2026-10-22 by two interim sentences in the text of record, so that the page
`/security` (A-006 §3.12) can carry this text literally from today: for `<contract ids>`
"No contract is in scope yet; the mainnet contract ids are listed here from the first
pool on", and for `<link>` "the list of known issues is published here before the first
pool". Both sentences are replaced at the first mainnet pool, in a commit of their own,
by the ids of `chain-config/mainnet.toml` and the address of the list; until then §3.1
knows no contract in scope.
Filled the same day: `<cap>` is $10 000, the cap of every pool in the beta (10 000 tokens);
`<ceiling>` is $3 000, the critical ceiling of §3.4 (10 % of three caps of $10 000), which
ADR-016 leaves unchanged (it binds a cap step to the funded ceiling, §3.4); `<route>` is the
self-hosted route of §3.2 and §3.3 since 2026-10-19. The ceiling and the cap on the page
change in the same commit as any cap change (§3.4).

### 7.2 Severity (Immunefi's Severity Classification System v2.3, smart contracts, read 2026-09-26)

- Critical: direct theft of user funds (other than unclaimed yield), permanent freezing of funds,
  protocol insolvency, manipulation of a governance vote result.
- High: theft or permanent freezing of unclaimed yield, temporary freezing of funds.
- Medium: a contract that cannot operate for lack of token funds, griefing, block stuffing,
  unbounded resource use.
- Low: a contract that fails to deliver promised returns without losing value.

### 7.3 Reference values

The ceiling rule of 10 % of funds at risk is Immunefi's scaling guidance (its blog of
2024-09-02: at most 10 % of funds at risk); the one Soroban program on a platform uses it with a
$5k floor and a $25k cap for funds far larger than ours. Our floor of $1 000 and the fixed
awards for high ($1 000), medium ($500) and low ($100) are this spec's proposal, sized to a
beta whose total at risk is $30 000; they change with an amendment, never by the page alone.
Caps at the start: $10 000 per pool, three pools (ADR-002). Acknowledgement 48 hours,
assessment 7 days, disclosure 90 days.
