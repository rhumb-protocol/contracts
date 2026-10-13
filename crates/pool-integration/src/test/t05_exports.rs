//! What the two adapter Wasms export, read from the binaries: exactly the entry points of spec
//! A-001 §3.2 and §3.3, the constructor, and nothing else.
//!
//! Spec §6 asks that "nothing beyond them is exported". The other tests prove that each listed
//! entry point exists and behaves; none of them can show that no unlisted one exists -- an
//! `upgrade`, a setter for the oracle, an `init` that could be called on a live adapter -- and
//! that absence is what the two contracts' immutability and their two-key design rest on. So
//! this test reads the export section of each Wasm as the host will read it and compares the
//! set of exported functions with the spec's list.
//!
//! The parser below reads the Wasm binary format's module structure (magic and version, then
//! sections of an id byte, a LEB128 size and a payload; the export section is id 7 and holds a
//! LEB128 count of entries, each a LEB128-prefixed name, a kind byte and a LEB128 index). It is
//! written here rather than taken from a crate so that the test adds no dependency to the
//! lockfile; it reads what it needs and refuses, loudly, anything it does not expect.

use std::path::PathBuf;
use std::string::String;
use std::vec::Vec;

/// A Wasm this workspace builds, or `None` when it has not been built, with the same notice
/// and the same `ADAPTER_WASM_REQUIRED` rule as the other tests of this repository.
fn built_wasm(name: &str) -> Option<Vec<u8>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/wasm32v1-none/release")
        .join(name);
    match std::fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(_) => {
            if std::env::var("ADAPTER_WASM_REQUIRED").is_ok() {
                panic!(
                    "{} is missing; build it with: cargo build --workspace --release --target wasm32v1-none",
                    path.display()
                );
            }
            std::eprintln!(
                "notice: {} is missing, skipping; build it with: cargo build --workspace --release --target wasm32v1-none",
                path.display()
            );
            None
        }
    }
}

/// A LEB128-encoded unsigned 32-bit integer at `*at`, advancing `*at` past it.
fn leb128(bytes: &[u8], at: &mut usize) -> u32 {
    let mut result: u32 = 0;
    let mut shift = 0;
    loop {
        let byte = bytes[*at];
        *at += 1;
        result |= u32::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return result;
        }
        shift += 7;
        assert!(shift < 32, "a LEB128 integer longer than 32 bits");
    }
}

/// The names of the functions a Wasm module exports (export kind 0), in the module's order.
fn exported_functions(wasm: &[u8]) -> Vec<String> {
    assert_eq!(&wasm[0..4], b"\0asm", "not a Wasm module");
    assert_eq!(&wasm[4..8], &[1, 0, 0, 0], "not Wasm version 1");
    let mut at = 8;
    let mut names = Vec::new();
    while at < wasm.len() {
        let id = wasm[at];
        at += 1;
        let size = leb128(wasm, &mut at) as usize;
        let end = at + size;
        if id == 7 {
            let count = leb128(wasm, &mut at);
            for _ in 0..count {
                let len = leb128(wasm, &mut at) as usize;
                let name = String::from_utf8(wasm[at..at + len].to_vec())
                    .expect("an export name that is not UTF-8");
                at += len;
                let kind = wasm[at];
                at += 1;
                let _index = leb128(wasm, &mut at);
                if kind == 0 {
                    names.push(name);
                }
            }
            assert_eq!(
                at, end,
                "the export section's size does not match its entries"
            );
        }
        at = end;
    }
    names
}

/// Asserts that `wasm` exports exactly `expected` as functions, whatever the order.
fn assert_exports(wasm: &[u8], expected: &[&str]) {
    let mut found = exported_functions(wasm);
    found.sort();
    let mut want: Vec<String> = expected.iter().map(|s| String::from(*s)).collect();
    want.sort();
    assert_eq!(
        found, want,
        "the exported functions are not the spec's entry points"
    );
}

#[test]
fn adapter_issuer_exports_exactly_the_entry_points_of_the_spec() {
    let Some(wasm) = built_wasm("adapter_issuer.wasm") else {
        return;
    };
    // Spec A-001 §3.3: the constructor, `post`, `observe`, `set_keeper`, `set_governance` and
    // the five views.
    assert_exports(
        &wasm,
        &[
            "__constructor",
            "post",
            "observe",
            "set_keeper",
            "set_governance",
            "keeper",
            "governance",
            "decimals",
            "latest",
            "version",
        ],
    );
}

#[test]
fn adapter_redstone_exports_exactly_the_entry_points_of_the_spec() {
    let Some(wasm) = built_wasm("adapter_redstone.wasm") else {
        return;
    };
    // Spec A-001 §3.2: the constructor, `observe` and the three views. No setter, no upgrade.
    assert_exports(
        &wasm,
        &["__constructor", "observe", "oracle", "asset", "version"],
    );
}
