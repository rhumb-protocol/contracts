//! Test 4 of spec `spec/A-001-rate-adapters.md` §4: the two adapters driven through the
//! engine's real pool.
//!
//! This crate exports no contract and holds no chain code. It exists so that the adapters of
//! §3.2 and §3.3 can be put where they will actually stand -- behind a band pool's `refresh`,
//! with the engine judging every reading -- and so that the scenarios of E-003 test 7 are run
//! against the pool Wasm a deployment would use, not against a restatement of what the pool is
//! believed to do. The other tests of this repository prove that an adapter returns what it
//! holds; this one proves what a pool makes of it.
//!
//! # The pool is not built here
//!
//! The engine is not built in this repository. The pool arrives as the optimised Wasm of
//! core's CI, placed by hand in `tests/fixtures/` with its origin, its run and its hash
//! recorded there. No engine source is copied in, and nothing here links an engine crate.
//!
//! # Then why is there an ABI in [`abi`]?
//!
//! Because a caller of a contract needs one. The pool's constructor takes a struct, `refresh`
//! returns an enum, and the rate config crosses the boundary as a fixed byte string; to call
//! any of it, this crate must state the shapes in its own types, exactly as a deployment and
//! any wallet will have to. That statement is written from the pool's published
//! interface (E-007 §3.2 and §3.7) and not copied from the engine's crates, and it is checked
//! the only way an interface can be: the calls in these tests reach the pool's Wasm and are
//! answered, which a wrong field name, a wrong order or a wrong byte count would not survive.
//!
//! # Provenance
//!
//! Nothing here is ported. The pool's interface and the layout of the encoded rate config and
//! rate state are read in the engine's repository (E-007 §3.2, §3.7) and written out here in
//! this crate's own words; the reference values of the configurations are the ones spec A-001
//! §7.4 states for USDY.

#![no_std]
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
#![deny(clippy::arithmetic_side_effects)]

#[cfg(test)]
extern crate std;

pub mod abi;
#[cfg(any(test, feature = "cli"))]
pub mod constructor_args;
pub mod rate_bytes;

#[cfg(test)]
mod test;
