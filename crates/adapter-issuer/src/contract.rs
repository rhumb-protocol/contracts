//! The entry points of spec A-001 §3.3.
//!
//! Each entry point's doc comment says what it refuses and, where it matters more, what it
//! deliberately does not refuse, so that a later change has to argue against a written reason
//! rather than fill in a gap that looks accidental.

// The SDK's `contracttype`, `contracterror`, `contractevent` and `contractimpl` macros
// generate items without doc comments, which the workspace's `missing_docs = "deny"` would
// reject; the items this file writes by hand are documented above each of them.
#![allow(missing_docs)]

use crate::error::Error;
use crate::events::{GovernanceChanged, Init, KeeperChanged, Post};
use crate::types::{DataKey, Posting};
use crate::{DECIMALS_MAX, INSTANCE_TTL, INSTANCE_TTL_THRESHOLD, VERSION};
use soroban_sdk::{Address, Env};

/// The adapter: one deployed instance is one issuer source, shared by every pool of its asset.
#[cfg_attr(target_family = "wasm", soroban_sdk::contract)]
pub struct IssuerAdapter;

/// The stored role at `key`, or `Error::BadDecimals` when the instance holds nothing.
///
/// The error branch cannot be observed on a live network: an instance holds no role only if
/// its entry expired, and an expired instance is archived, so every invocation fails before
/// this runs. It is kept because refusing is the only safe answer if it is ever reached --
/// there would be no address that could have authorised anything.
///
/// Reusing `BadDecimals` gives that code a second meaning, which is a real cost and is
/// accepted only because spec §6 asks that nothing beyond the four errors of §3.3 is
/// exported. A reader who ever sees `303` from `post` or a role setter is looking at this
/// branch and not at a scale.
fn role(env: &Env, key: &DataKey) -> Result<Address, Error> {
    env.storage().instance().get(key).ok_or(Error::BadDecimals)
}

#[cfg_attr(target_family = "wasm", soroban_sdk::contractimpl)]
impl IssuerAdapter {
    /// Fixes the two roles and the scale (spec §3.3).
    ///
    /// `decimals` above [`DECIMALS_MAX`] is refused: the pool refuses more (E-003 §3.2), so
    /// such an adapter could only ever be a source the pool skips. Every source of Rhumb is
    /// deployed at 18 and the keeper scales every reading to 18 before posting (spec §3.4),
    /// so that one number means the same thing at every source of every pool; a coarser scale
    /// is accepted here because the pool accepts it, and is reported unchanged so that the
    /// pool does the scaling.
    ///
    /// `Latest` is not written: the adapter holds no price until the keeper posts one, and
    /// `observe` answers `None` until then, which a pool reads as a source without an
    /// observation (E-007 §3.2). A placeholder price would be a rate nobody published.
    ///
    /// Neither role signs: the roles are stored with the deployer's authorisation alone (spec
    /// §3.3, the rule the engine's pool and router follow too). Anyone can deploy an
    /// adapter naming the brand's addresses, and such an adapter is not the brand's: the brand
    /// records the adapters it deployed and reads both roles back after each deployment, and no
    /// pool reads an adapter governance did not sign into it. It publishes `init` with the two
    /// roles and the scale, so that the roles can be followed from the deployment on.
    ///
    /// Errors: [`Error::BadDecimals`].
    pub fn __constructor(
        env: Env,
        governance: Address,
        keeper: Address,
        decimals: u32,
    ) -> Result<(), Error> {
        if decimals > DECIMALS_MAX {
            return Err(Error::BadDecimals);
        }
        env.storage()
            .instance()
            .set(&DataKey::Governance, &governance);
        env.storage().instance().set(&DataKey::Keeper, &keeper);
        env.storage().instance().set(&DataKey::Decimals, &decimals);
        Init {
            governance,
            keeper,
            decimals,
        }
        .publish(&env);
        Ok(())
    }

    /// The keeper's one entry point: store what it read at the issuer (spec §3.3).
    ///
    /// `timestamp` is the issuer's own publication time in seconds -- for Ondo the block time
    /// of the Ethereum read, for Etherfuse `current_time` of the response, for Centrifuge the
    /// block time of the event -- never the moment the keeper read and never the NAV's
    /// valuation time. This contract cannot check which of the three it was given; nothing on
    /// chain can. The keeper's readers carry that rule and its tests hold them to it (spec
    /// §3.4, §4 test 5).
    ///
    /// It refuses three things and three only. A price at or below zero is not a price. A
    /// timestamp older than the stored one would let a keeper looping on a cached read
    /// overwrite a fresher price with an older one. A timestamp later than the ledger's is not
    /// a publication time, and would poison the entry, because no honest later reading could
    /// then be newer.
    ///
    /// An equal timestamp is accepted (spec §3.3, amended 2026-09-16). It used to be refused
    /// with the older ones, which meant an issuer that corrected a price without moving its
    /// publication time could not reach the pool at all: the adapter would hold a price the
    /// issuer no longer publishes until that issuer's clock happened to move. Whether such a
    /// post is worth a fee is the keeper's question and not this contract's -- the adapter
    /// compares times, the keeper compares news (A-001 §3.4) -- and the append-only rule this
    /// relaxes is unchanged in the direction that matters: nothing older ever lands.
    ///
    /// It applies no step bound, no drift band against the other source and no staleness rule.
    /// Those are the engine's (E-003), and a bound here would hide from the pool what the
    /// keeper posted -- which is the one thing a second source exists to reveal. It rounds
    /// nothing and scales nothing: the price is stored as given, at the scale the constructor
    /// fixed.
    ///
    /// The instance TTL is extended when it is below the threshold, so that an adapter whose
    /// keeper is posting cannot expire and take every pool's second source with it.
    ///
    /// Errors: [`Error::BadPrice`], [`Error::NotNewer`], [`Error::FromTheFuture`].
    pub fn post(env: Env, price: i128, timestamp: u64) -> Result<(), Error> {
        let keeper = role(&env, &DataKey::Keeper)?;
        keeper.require_auth();
        if price <= 0 {
            return Err(Error::BadPrice);
        }
        if timestamp > env.ledger().timestamp() {
            return Err(Error::FromTheFuture);
        }
        if let Some(latest) = env.storage().instance().get::<_, Posting>(&DataKey::Latest) {
            if timestamp < latest.timestamp {
                return Err(Error::NotNewer);
            }
        }
        env.storage()
            .instance()
            .set(&DataKey::Latest, &Posting { price, timestamp });
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL);
        Post {
            keeper,
            price,
            timestamp,
        }
        .publish(&env);
        Ok(())
    }

    /// The adapter interface of E-007 §5: the newest posting with the adapter's scale (spec
    /// §3.3).
    ///
    /// `None` before the first post, which is a source without an observation as far as a pool
    /// is concerned (E-007 §3.2). The missing-`Decimals` branch answers `None` too and cannot
    /// be observed, for the reason given on `role`. It converts nothing and bounds nothing:
    /// the three numbers are what the keeper posted and what the constructor fixed.
    pub fn observe(env: Env) -> Option<(i128, u32, u64)> {
        let decimals: u32 = env.storage().instance().get(&DataKey::Decimals)?;
        let latest: Posting = env.storage().instance().get(&DataKey::Latest)?;
        Some((latest.price, decimals, latest.timestamp))
    }

    /// Rotation of the operational key, the one key that signs unattended (spec §3.3).
    ///
    /// The keeper's key sits on an unattended machine, so this is the one transaction that
    /// ends an incident with it: after it the old key can do nothing at all, because `post` is
    /// the only thing a keeper could ever do and it is checked against the stored address on
    /// every call. It does not touch `Latest`: a price the old keeper posted stays, and
    /// whether it is still to be trusted is a matter for the guardian and the engine's gates,
    /// not for a key rotation.
    ///
    /// Errors: none of its own; a caller who is not governance fails on the authorisation.
    pub fn set_keeper(env: Env, keeper: Address) -> Result<(), Error> {
        let governance = role(&env, &DataKey::Governance)?;
        governance.require_auth();
        let old = role(&env, &DataKey::Keeper)?;
        env.storage().instance().set(&DataKey::Keeper, &keeper);
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL);
        KeeperChanged {
            governance,
            old,
            new: keeper,
        }
        .publish(&env);
        Ok(())
    }

    /// Handover of the governance role (spec §3.3): the old governance signs, for the move to
    /// a multisig (E-007 §3.6).
    ///
    /// Errors: none of its own; a caller who is not the current governance fails on the
    /// authorisation.
    pub fn set_governance(env: Env, governance: Address) -> Result<(), Error> {
        let old = role(&env, &DataKey::Governance)?;
        old.require_auth();
        env.storage()
            .instance()
            .set(&DataKey::Governance, &governance);
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL);
        GovernanceChanged {
            old,
            new: governance,
        }
        .publish(&env);
        Ok(())
    }

    /// The address that may `post` (spec §3.3). `None` only for an instance that holds
    /// nothing, which cannot be observed; see `role`.
    pub fn keeper(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Keeper)
    }

    /// The address that may rotate the keeper and hand its own role on (spec §3.3). `None`
    /// under the same unobservable condition as [`Self::keeper`].
    pub fn governance(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Governance)
    }

    /// The scale every posted price is at (spec §3.3). `None` under the same unobservable
    /// condition as [`Self::keeper`].
    pub fn decimals(env: Env) -> Option<u32> {
        env.storage().instance().get(&DataKey::Decimals)
    }

    /// The newest posting, or `None` before the first post (spec §3.3).
    pub fn latest(env: Env) -> Option<Posting> {
        env.storage().instance().get(&DataKey::Latest)
    }

    /// The spec revision of this Wasm (spec §3.3).
    pub fn version(_env: Env) -> u32 {
        VERSION
    }
}
