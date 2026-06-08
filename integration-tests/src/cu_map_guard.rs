//! Guards that the CU-tracking discriminator map in the async_vault client stays
//! in sync with the program IDL. If an instruction is added, removed, or renamed,
//! this test fails until `DISCRIMINATORS` in `clients/rust/async_vault/src/lib.rs`
//! is updated.

use async_vault_client::{lite::instruction_label, ASYNC_VAULT_ID};

#[test]
fn cu_discriminator_map_matches_idl() {
    let idl_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../idl/async_vault.json");
    let raw = std::fs::read_to_string(idl_path).unwrap_or_else(|e| panic!("read {idl_path}: {e}"));
    let idl: serde_json::Value = serde_json::from_str(&raw).expect("parse IDL");

    let instructions = idl["instructions"]
        .as_array()
        .expect("idl.instructions array");
    assert!(!instructions.is_empty(), "no instructions in IDL");

    for ix in instructions {
        let name = ix["name"].as_str().expect("instruction name");
        let disc: Vec<u8> = ix["discriminator"]
            .as_array()
            .expect("instruction discriminator")
            .iter()
            .map(|b| b.as_u64().expect("discriminator byte") as u8)
            .collect();

        assert_eq!(
            instruction_label(&ASYNC_VAULT_ID, &disc),
            Some(name),
            "CU discriminator map out of sync for `{name}` — update DISCRIMINATORS in clients/rust/async_vault/src/lib.rs",
        );
    }
}
