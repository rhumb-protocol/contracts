//! Test 4 of spec A-001 §4: the scenarios of E-003 test 7, driven through the engine's pool
//! Wasm with one `adapter-redstone` on a mock SEP-40 feed and one `adapter-issuer`.
//!
//! Everything here runs against Wasm: the pool's, the two adapters' and the mock's. There is
//! no native mode, because the point of this test is the boundary -- what a pool makes of what
//! an adapter returns -- and natively there would be no boundary to cross. The line coverage
//! A-001 §6 asks for is the adapters' own, and their own crates' tests carry it.
//!
//! The pool's Wasm is required: it sits in `tests/fixtures/` with its origin recorded, and a
//! test that quietly passed without it would report green for the one requirement of this
//! spec that cannot be met by this repository alone. The three Wasm files this workspace
//! builds are read from the target directory, with the notice-and-skip of the other crates'
//! tests unless `ADAPTER_WASM_REQUIRED` is set, which CI sets after the Wasm build.

#![allow(
    clippy::arithmetic_side_effects,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod a002_constructor_args;
pub mod harness;
mod t04_pool;
mod t05_exports;
