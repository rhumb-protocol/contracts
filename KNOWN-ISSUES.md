# Known issues

This is the list of known issues that https://rhumb.finance/security refers to. A report of an
issue listed here is out of scope for the bug bounty (A-008 §3.1, §7.1).

Date: 2026-10-31

- The contracts are unaudited; no audit report and no pre-audit report have been published yet (E-007 §2, A-008 §3.1, §7.1).
- Every pool runs in a capped beta with a hard cap of $10 000 (10 000 tokens) on deposits (E-007 §2, A-008 §7.1).
- The guardian, and the sentinel where one is set, can pause a pool: swaps and deposits stop, withdrawals continue (E-007 §3.2, §3.6).
- The guardian can suspend a band pool's rate; a band pool refuses swaps and deposits on a stale or suspended rate, and withdrawals continue (E-003 §3.2, §3.4, E-007 §3.2).
- A reading outside the step bound or the drift band, or two sources that disagree beyond the cross-check tolerance, suspends a band pool's rate even when the reading is true, and only a human resume clears it (E-003 §3.2, §3.3).
- A band pool with two sources reads no rate after genesis and after a resume until both sources have published and agreed, and refuses with `SourceLost` once fewer than two sources are live (a source drops out after `max_age` without a new reading) for longer than the grace period (E-003 §3.2, §3.3).
- A confirmed two-source rate can be served up to `max_age` old, and up to `max_age + grace` while one source is lost; its time is that of the newer reading of the pair (E-003 §3.2, §3.3).
- A rate that is wrong by less than the pool's band can cost LPs up to the band, and a trader gets the worse edge of the two sources within the cross-check tolerance (E-003 §2, §3.2, §3.3).
- The adapters apply no price bound (step, drift band, staleness) to what they read or what the keeper posts; those bounds are the pool's (A-001 §3.2, §3.3).
- An archived RedStone adapter is a lost source until a human restores it (A-001 §3.2).
- An LP's share entry expires after `SHARES_TTL` (reference value in the spec: 120 days) without a deposit or withdrawal and is restored by `restore_shares`, which any address may call; a quiet pool's code entry expires unless it is extended (E-007 §3.1).
- A same-asset swap is refused, and a pool refuses the same token on both sides at creation (A-008 §3.1, E-007 §2).
- A transfer in that does not raise the pool's balance by exactly the recorded amount is refused, so tokens that take a fee on transfer or rebase are not supported (E-007 §2, §3.3, §5).
- After an issuer clawback or freeze, governance writes a reserve down to the pool's balance net of owed protocol fees, on a paused pool after an announcement; the loss falls on the LPs pro rata (E-007 §3.2).
- A withdrawal while a write-down announcement stands is paid at the announced factor, while the reserve falls by the unscaled payout (E-007 §3.2).
- Governance can upgrade the Wasm, change the rate path and the roles, and raise the cap, each after an announcement the guardian can cancel (E-007 §3.2).
- A stolen or unwilling guardian key can veto every announced change of the roles, the rate path, the cap and the Wasm for good; trading or the pause stays as it is and withdrawals stay open (E-007 §3.6).
- A lost governance key freezes every switch for good, with no recovery path in the contract (E-007 §3.6).
- A stolen sentinel key can pause a pool until governance or the guardian clears it and the guardian unpauses (E-007 §3.6).
- A pool or adapter deployed by anyone from the published Wasm is not a Rhumb contract; the registry is the list of mainnet contract ids on the security page (E-007 §3.2, A-001 §3.3, A-008 §7.1).
