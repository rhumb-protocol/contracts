//! What a `post` writes: the adapter's instance entry and nothing else (A-001 §3.3), the bound
//! the signer of a `post` holds its write set to.
//!
//! The keeper bounds the write set of every `post` it signs to the adapter contract's entries
//! and refuses anything else there, a nonce or an account entry included, because the entries a
//! transaction writes are the entries its key pays to write. That bound is only right if the
//! adapter keeps to it, so this pins the adapter's side: every value `post` stores lives in
//! instance storage, and one post writes exactly one ledger entry, which is that instance.
//!
//! The keeper is the transaction's source account on a chain and authorises its `post` with a
//! source-account credential, which consumes no nonce. The test does the same: the adapter's
//! keeper is the source account the test environment runs as, and the post carries a
//! source-account authorisation rather than a mocked one. A mocked authorisation is an address
//! credential, and an address credential writes a nonce entry of its own; that second entry is
//! what the keeper refuses, and the last test shows it.
//!
//! Wasm mode only: the count comes from the host's metering of a top-level invocation, which a
//! native call inside `as_contract` is not. The count is the number of read-write entries in the
//! footprint the host recorded for the call; that the one entry is the instance follows from the
//! posting being readable from instance storage afterwards.

use super::harness::{Fixture, JAN_1_2026};
use super::Mode;
use crate::types::{DataKey, Posting};
use crate::INSTANCE_TTL_THRESHOLD;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::xdr::{
    InvokeContractArgs, ScAddress, ScSymbol, ScVal, SorobanAuthorizationEntry,
    SorobanAuthorizedFunction, SorobanAuthorizedInvocation, SorobanCredentials, StringM, VecM,
};
use soroban_sdk::{Address, Env, IntoVal, TryFromVal, Val};

/// A price of one, at 18 decimals.
const ONE: i128 = 1_000_000_000_000_000_000;

/// The source account the test environment runs every invocation as: the ed25519 key of
/// thirty-two zero bytes, which nobody holds.
const SOURCE_ACCOUNT: &str = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";

fn source_account(env: &Env) -> Address {
    Address::from_str(env, SOURCE_ACCOUNT)
}

/// A fixture whose keeper is the transaction's source account, as on a chain.
fn keeper_is_source() -> Option<Fixture> {
    Fixture::with_keeper(Mode::Wasm, source_account)
}

/// `post(price, timestamp)` authorised the way the keeper authorises it: a source-account
/// credential for exactly this call, and nothing mocked.
fn post_as_source(f: &Fixture, price: i128, timestamp: u64) {
    let arg = |v: Val| ScVal::try_from_val(&f.env, &v).unwrap();
    let args: VecM<ScVal> = std::vec![arg(price.into_val(&f.env)), arg(timestamp.into_val(&f.env))]
        .try_into()
        .unwrap();
    let name: StringM<32> = "post".try_into().unwrap();
    f.env.set_auths(&[SorobanAuthorizationEntry {
        credentials: SorobanCredentials::SourceAccount,
        root_invocation: SorobanAuthorizedInvocation {
            function: SorobanAuthorizedFunction::ContractFn(InvokeContractArgs {
                contract_address: ScAddress::from(&f.adapter),
                function_name: ScSymbol(name),
                args,
            }),
            sub_invocations: VecM::default(),
        },
    }]);
    f.client().post(&price, &timestamp);
}

/// The number of ledger entries the last top-level invocation wrote.
fn written(f: &Fixture) -> u32 {
    f.env.cost_estimate().resources().write_entries
}

/// The posting as instance storage holds it, read directly rather than through `latest()`.
fn stored_in_instance(f: &Fixture) -> Option<Posting> {
    f.env.as_contract(&f.adapter, || {
        f.env.storage().instance().get(&DataKey::Latest)
    })
}

#[test]
fn the_first_post_writes_the_instance_entry_and_nothing_else() {
    let Some(f) = keeper_is_source() else {
        return;
    };
    let at = JAN_1_2026 - 60;
    post_as_source(&f, ONE, at);
    assert_eq!(written(&f), 1, "one entry is written");
    assert_eq!(
        stored_in_instance(&f),
        Some(Posting {
            price: ONE,
            timestamp: at
        }),
        "and it is the instance"
    );
}

#[test]
fn a_later_post_writes_the_instance_entry_and_nothing_else() {
    let Some(f) = keeper_is_source() else {
        return;
    };
    post_as_source(&f, ONE, JAN_1_2026 - 120);
    post_as_source(&f, ONE + 1, JAN_1_2026 - 60);
    assert_eq!(written(&f), 1);
    assert_eq!(
        stored_in_instance(&f),
        Some(Posting {
            price: ONE + 1,
            timestamp: JAN_1_2026 - 60
        })
    );
}

#[test]
fn a_post_that_extends_the_instance_writes_nothing_else() {
    // The TTL extension a post carries below the threshold (spec A-001 §3.3) is an extension of
    // the entry the post writes anyway, not a second entry.
    let Some(f) = keeper_is_source() else {
        return;
    };
    let spend = f.instance_ttl() - INSTANCE_TTL_THRESHOLD + 1;
    f.advance(spend);
    post_as_source(&f, ONE, f.now() - 60);
    assert_eq!(written(&f), 1);
}

#[test]
fn a_keeper_that_is_not_the_source_account_adds_a_nonce_to_the_write_set() {
    // The case the keeper refuses (`keeper/tests/write_set.rs`): the same post, authorised by an
    // address credential of a keeper that is not the source account, writes the instance and a
    // nonce entry of that keeper's.
    let Some(f) = Fixture::with_keeper(Mode::Wasm, Address::generate) else {
        return;
    };
    assert_eq!(f.post(ONE, JAN_1_2026 - 60), Ok(()));
    assert_eq!(written(&f), 2, "the instance and the keeper's nonce");
}
