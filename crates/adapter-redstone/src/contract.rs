//! The entry points of spec A-001 §3.2.
//!
//! The whole contract is here because there is almost nothing to it: a constructor that fixes
//! the feed and the asset after one check and publishes them, a read that forwards what the
//! feed says, and three views. What it does not contain is the point of it -- no scaling, no
//! bound, no comparison with a previous reading, no fallback to another source -- and the doc
//! comments say so at each entry point so that a later change has to argue against a written
//! reason.

// The SDK's `contracttype`, `contracterror`, `contractclient` and `contractimpl` macros
// generate items without doc comments, which the workspace's `missing_docs = "deny"` would
// reject; the items this file writes by hand are documented above each of them.
#![allow(missing_docs)]

use crate::error::Error;
use crate::events::Init;
use crate::sep40::{Asset, Sep40Client};
use crate::types::DataKey;
use crate::{DECIMALS_MAX, VERSION};
use soroban_sdk::{Address, Env};

/// The adapter: one deployed instance is one SEP-40 feed as one source of one band pool.
#[cfg_attr(target_family = "wasm", soroban_sdk::contract)]
pub struct RedstoneAdapter;

#[cfg_attr(target_family = "wasm", soroban_sdk::contractimpl)]
impl RedstoneAdapter {
    /// Fixes the feed and the asset for the life of the contract (spec §3.2).
    ///
    /// It reads the feed's `decimals()` once, and refuses when the call fails or the answer is
    /// above [`DECIMALS_MAX`]: the pool refuses more than 18 decimals (E-003 §3.2), so such an
    /// adapter could only ever be a source the pool skips, and a source that can never work is
    /// better never created than discovered once a pool is live on it. The check is on the
    /// feed's answer at this moment only; a feed that changes its scale later is reported as
    /// it then is, because what the pool accepts is the pool's to decide on every `refresh`.
    ///
    /// There is no setter and no upgrade: a source is replaced by deploying a new adapter and
    /// calling the pool's `set_rate` (E-007 §3.2), so what a pool reads cannot change under it
    /// without governance having signed for the change.
    ///
    /// Nothing is signed but the deployment: the feed and the asset are stored with the
    /// deployer's authorisation alone (spec §3.2, as amended). It publishes `init` with
    /// the two, because the deployment is the one moment this adapter has anything to say.
    ///
    /// Errors: [`Error::BadDecimals`]. A feed whose `decimals()` fails fails the deployment
    /// through the failing call itself.
    pub fn __constructor(env: Env, oracle: Address, asset: Asset) -> Result<(), Error> {
        if Sep40Client::new(&env, &oracle).decimals() > DECIMALS_MAX {
            return Err(Error::BadDecimals);
        }
        env.storage().instance().set(&DataKey::Oracle, &oracle);
        env.storage().instance().set(&DataKey::Asset, &asset);
        Init { oracle, asset }.publish(&env);
        Ok(())
    }

    /// The adapter interface of E-007 §5: the feed's newest reading for the asset, unchanged.
    ///
    /// It asks the feed for its scale and for its reading and hands back the three numbers
    /// exactly as received: `None` when the feed holds no reading, and a failure of the
    /// adapter's own call when the feed fails, which the pool treats as a source without an
    /// observation (E-007 §3.2). It rounds nothing, scales nothing, bounds nothing and
    /// converts no unit -- in particular not the timestamp, whatever unit the feed publishes
    /// it in -- and writes nothing to storage, because every judgement about a reading belongs
    /// to `Observation::from_sep40` and the gates of E-003, and a bound applied here would
    /// turn a reading the pool would have reported as ignored into a source that merely looks
    /// silent.
    ///
    /// The feed's scale is read on every call rather than taken from the constructor's check,
    /// because a feed may change it and the pool must see what the feed says now.
    ///
    /// The two `?` on the stored oracle and asset answer `None` for an instance that holds
    /// nothing. That cannot be reached on a live network -- an expired instance is archived
    /// and the invocation fails before this body runs -- but `None` is the right answer if it
    /// ever is, because a pool reads it as a source without an observation (E-007 §3.2).
    pub fn observe(env: Env) -> Option<(i128, u32, u64)> {
        let oracle: Address = env.storage().instance().get(&DataKey::Oracle)?;
        let asset: Asset = env.storage().instance().get(&DataKey::Asset)?;
        let feed = Sep40Client::new(&env, &oracle);
        let decimals = feed.decimals();
        let reading = feed.lastprice(&asset)?;
        Some((reading.price, decimals, reading.timestamp))
    }

    /// The SEP-40 contract this adapter reads (spec §3.2).
    ///
    /// `None` only when the instance holds nothing, which cannot be observed: an instance entry can only be gone because its TTL
    /// lapsed, and when that happens the contract instance itself is archived and every
    /// invocation fails before this body runs, so the branch is unreachable on a live network.
    /// It is kept because it is the safe answer if it is ever reached,
    /// and because there is no fifth error code to say it with (spec §6).
    pub fn oracle(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Oracle)
    }

    /// The asset it asks that contract for (spec §3.2).
    ///
    /// `None` under the same unobservable condition as [`Self::oracle`].
    pub fn asset(env: Env) -> Option<Asset> {
        env.storage().instance().get(&DataKey::Asset)
    }

    /// The spec revision of this Wasm (spec §3.2).
    pub fn version(_env: Env) -> u32 {
        VERSION
    }
}
