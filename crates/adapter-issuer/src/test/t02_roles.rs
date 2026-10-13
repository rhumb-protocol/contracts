//! Spec A-001 §4 test 2, the roles half: who may `post`, who may rotate the keeper, who may
//! hand governance on, and what each of them cannot do.
//!
//! These are the checks that decide what a stolen key is worth. The keeper's key sits on an
//! unattended machine, so the test that matters most is the one showing the
//! keeper cannot rotate itself or reach governance; and the one after it, showing that a
//! rotation takes the old keeper's ability away in the same transaction that gives it to the
//! new one.
//!
//! A refused authorisation is asserted in `Wasm` mode: natively a failing `require_auth` is a
//! panic rather than an error the host returns.

use super::harness::{Fixture, JAN_1_2026};
use super::Mode;
use crate::events::{GovernanceChanged, KeeperChanged};
use soroban_sdk::testutils::{Address as _, MockAuth, MockAuthInvoke};
use soroban_sdk::{Address, IntoVal};

/// A price of one, at 18 decimals.
const ONE: i128 = 1_000_000_000_000_000_000;

both!(governance_can_rotate_the_keeper, |f: &Fixture| {
    let new = Address::generate(&f.env);
    assert_eq!(f.set_keeper(&new), Ok(()));
    assert_eq!(f.keeper_view(), Some(new));
    // Governance is untouched by a keeper rotation.
    assert_eq!(f.governance_view(), Some(f.governance.clone()));
});

both!(governance_can_hand_its_own_role_on, |f: &Fixture| {
    let new = Address::generate(&f.env);
    assert_eq!(f.set_governance(&new), Ok(()));
    assert_eq!(f.governance_view(), Some(new));
    // The keeper is untouched by a governance handover.
    assert_eq!(f.keeper_view(), Some(f.keeper.clone()));
});

both!(
    the_new_keeper_posts_and_the_old_one_is_done,
    |f: &Fixture| {
        let new = Address::generate(&f.env);
        assert_eq!(f.set_keeper(&new), Ok(()));
        // With every authorisation mocked, what still separates the two is the stored keeper: the
        // contract requires the *current* keeper's signature, so the post below is the new one's.
        assert_eq!(f.post(ONE, JAN_1_2026 - 60), Ok(()));
        assert_eq!(f.keeper_view(), Some(new));
    }
);

/// The Wasm client with exactly one authorisation mocked: `who` signing `name` on the adapter.
fn authorised_as<'a>(
    f: &'a Fixture,
    who: &'a Address,
    name: &'a str,
    args: soroban_sdk::Vec<soroban_sdk::Val>,
) -> super::IssuerClient<'a> {
    f.env.set_auths(&[]);
    f.env.mock_auths(&[MockAuth {
        address: who,
        invoke: &MockAuthInvoke {
            contract: &f.adapter,
            fn_name: name,
            args,
            sub_invokes: &[],
        },
    }]);
    f.client()
}

#[test]
fn a_stranger_cannot_post() {
    let Some(f) = Fixture::new(Mode::Wasm) else {
        return;
    };
    let stranger = Address::generate(&f.env);
    let at = JAN_1_2026 - 60;
    let client = authorised_as(&f, &stranger, "post", (ONE, at).into_val(&f.env));
    assert!(
        client.try_post(&ONE, &at).is_err(),
        "only the keeper may post"
    );
    assert_eq!(f.latest(), None);
}

#[test]
fn governance_cannot_post() {
    // The roles do not overlap in the one direction that matters less, and not in this one
    // either: governance is a cold key and must not be asked to sign a routine post.
    let Some(f) = Fixture::new(Mode::Wasm) else {
        return;
    };
    let at = JAN_1_2026 - 60;
    let governance = f.governance.clone();
    let client = authorised_as(&f, &governance, "post", (ONE, at).into_val(&f.env));
    assert!(client.try_post(&ONE, &at).is_err(), "only the keeper posts");
}

#[test]
fn the_keeper_cannot_rotate_itself() {
    // The whole point of the two roles: a key taken off the keeper's machine can post a bad
    // price until governance rotates it, and cannot make itself permanent.
    let Some(f) = Fixture::new(Mode::Wasm) else {
        return;
    };
    let attacker = Address::generate(&f.env);
    let keeper = f.keeper.clone();
    let client = authorised_as(
        &f,
        &keeper,
        "set_keeper",
        (attacker.clone(),).into_val(&f.env),
    );
    assert!(
        client.try_set_keeper(&attacker).is_err(),
        "the keeper may only post"
    );
    assert_eq!(f.keeper_view(), Some(f.keeper.clone()));
}

#[test]
fn the_keeper_cannot_take_governance() {
    let Some(f) = Fixture::new(Mode::Wasm) else {
        return;
    };
    let attacker = Address::generate(&f.env);
    let keeper = f.keeper.clone();
    let client = authorised_as(
        &f,
        &keeper,
        "set_governance",
        (attacker.clone(),).into_val(&f.env),
    );
    assert!(
        client.try_set_governance(&attacker).is_err(),
        "the keeper may only post"
    );
    assert_eq!(f.governance_view(), Some(f.governance.clone()));
}

#[test]
fn a_stranger_cannot_rotate_the_keeper_or_governance() {
    let Some(f) = Fixture::new(Mode::Wasm) else {
        return;
    };
    let stranger = Address::generate(&f.env);
    let client = authorised_as(
        &f,
        &stranger,
        "set_keeper",
        (stranger.clone(),).into_val(&f.env),
    );
    assert!(client.try_set_keeper(&stranger).is_err());
    let client = authorised_as(
        &f,
        &stranger,
        "set_governance",
        (stranger.clone(),).into_val(&f.env),
    );
    assert!(client.try_set_governance(&stranger).is_err());
    assert_eq!(f.keeper_view(), Some(f.keeper.clone()));
    assert_eq!(f.governance_view(), Some(f.governance.clone()));
}

#[test]
fn the_old_keeper_cannot_post_after_a_rotation() {
    let Some(f) = Fixture::new(Mode::Wasm) else {
        return;
    };
    let new = Address::generate(&f.env);
    assert_eq!(f.set_keeper(&new), Ok(()));
    let at = JAN_1_2026 - 60;
    let old = f.keeper.clone();
    let client = authorised_as(&f, &old, "post", (ONE, at).into_val(&f.env));
    assert!(
        client.try_post(&ONE, &at).is_err(),
        "a rotation takes the old key's one ability away"
    );
    assert_eq!(f.latest(), None);
}

#[test]
fn the_old_governance_cannot_act_after_a_handover() {
    let Some(f) = Fixture::new(Mode::Wasm) else {
        return;
    };
    let new_governance = Address::generate(&f.env);
    assert_eq!(f.set_governance(&new_governance), Ok(()));
    let attacker = Address::generate(&f.env);
    let old = f.governance.clone();
    let client = authorised_as(&f, &old, "set_keeper", (attacker.clone(),).into_val(&f.env));
    assert!(client.try_set_keeper(&attacker).is_err());
    assert_eq!(f.keeper_view(), Some(f.keeper.clone()));
}

#[test]
fn a_rotation_emits_both_sides_of_it() {
    let Some(f) = Fixture::new(Mode::Wasm) else {
        return;
    };
    let new = Address::generate(&f.env);
    assert_eq!(f.set_keeper(&new), Ok(()));
    f.assert_events(&[&KeeperChanged {
        governance: f.governance.clone(),
        old: f.keeper.clone(),
        new,
    }]);
}

#[test]
fn a_handover_emits_both_sides_of_it() {
    let Some(f) = Fixture::new(Mode::Wasm) else {
        return;
    };
    let new = Address::generate(&f.env);
    assert_eq!(f.set_governance(&new), Ok(()));
    f.assert_events(&[&GovernanceChanged {
        old: f.governance.clone(),
        new,
    }]);
}

#[test]
fn the_new_keeper_posts_with_its_own_signature_alone() {
    // The other half of a rotation: the address the setter stored is the one whose signature
    // `post` checks. With every authorisation mocked, `the_new_keeper_posts_and_the_old_one_is_done`
    // cannot tell the new key's signature from anyone's; this one mocks exactly the new
    // keeper's and nothing else, so a setter that stored the old address again (the argument
    // ignored) would fail here.
    let Some(f) = Fixture::new(Mode::Wasm) else {
        return;
    };
    let new = Address::generate(&f.env);
    assert_eq!(f.set_keeper(&new), Ok(()));
    let at = JAN_1_2026 - 60;
    let client = authorised_as(&f, &new, "post", (ONE, at).into_val(&f.env));
    assert_eq!(
        client.try_post(&ONE, &at),
        Ok(Ok(())),
        "the rotated-in keeper's signature alone must carry a post"
    );
    assert_eq!(f.observe(), Some((ONE, 18, at)));
}

#[test]
fn the_new_governance_rotates_with_its_own_signature_alone() {
    // The same for the handover: after `set_governance(new)` it is `new`'s signature and no
    // other that `set_keeper` checks.
    let Some(f) = Fixture::new(Mode::Wasm) else {
        return;
    };
    let new_governance = Address::generate(&f.env);
    assert_eq!(f.set_governance(&new_governance), Ok(()));
    let new_keeper = Address::generate(&f.env);
    let client = authorised_as(
        &f,
        &new_governance,
        "set_keeper",
        (new_keeper.clone(),).into_val(&f.env),
    );
    assert_eq!(
        client.try_set_keeper(&new_keeper),
        Ok(Ok(())),
        "the handed-over governance's signature alone must carry a rotation"
    );
    assert_eq!(f.keeper_view(), Some(new_keeper));
}
