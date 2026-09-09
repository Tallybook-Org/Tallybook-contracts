#![cfg(test)]

// The crate is #![no_std]; tests need std for the standard test harness and
// for building throwaway data like the leaf array below.
extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Bytes, BytesN, Env, String};

use crate::{Error, StatementRegistry};

/// sha256(min(a, b) || max(a, b)) — a reference implementation independent
/// of merkle::fold, so tests assert against a computation that doesn't
/// share fold's own logic.
fn hash_pair(env: &Env, a: &BytesN<32>, b: &BytesN<32>) -> BytesN<32> {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    let mut concatenated = Bytes::from(lo.clone());
    concatenated.append(&Bytes::from(hi.clone()));
    env.crypto().sha256(&concatenated).to_bytes()
}

/// Registers a fresh, real price_book contract (never mocked — the whole
/// point of §6's PriceVersionStale check is to test against real
/// on-chain price-book behaviour) and a fresh StatementRegistry pointed at
/// it. Auth is not mocked here; individual tests opt into
/// `env.mock_all_auths()` or explicit `mock_auths` themselves.
fn setup() -> (Env, Address, Address, Address) {
    let env = Env::default();
    let admin = Address::generate(&env);
    let price_book_admin = Address::generate(&env);
    let price_book_id = env.register(crate::price_book::WASM, (price_book_admin,));
    let contract_id = env.register(StatementRegistry, (admin.clone(), price_book_id.clone()));
    (env, contract_id, admin, price_book_id)
}

/// Publishes a price schedule for `operator`, effective immediately at
/// `effective_ledger`, against the real price_book at `price_book_id`, and
/// returns the resulting version number. Caller must have already
/// authorized `operator` (e.g. via `env.mock_all_auths()`).
fn publish_schedule(
    env: &Env,
    price_book_id: &Address,
    operator: &Address,
    effective_ledger: u32,
) -> u32 {
    let client = crate::price_book::Client::new(env, price_book_id);
    let schedule_hash = BytesN::from_array(env, &[1u8; 32]);
    let uri = String::from_str(env, "https://example.com/schedule.json");
    client.publish(operator, &schedule_hash, &uri, &effective_ledger)
}

mod constructor {
    use super::*;

    #[test]
    fn happy_path_sets_admin_and_price_book() {
        // No public getters for Admin or PriceBook by design (admin holds
        // no power; price_book is only ever read internally by anchor()) —
        // reach into storage.rs directly (crate-internal) to prove the
        // writes landed.
        let (env, contract_id, _admin, price_book_id) = setup();
        let (admin_is_set, stored_price_book) = env.as_contract(&contract_id, || {
            (crate::storage::has_admin(&env), crate::storage::get_price_book(&env))
        });
        assert!(admin_is_set);
        assert_eq!(stored_price_book, Some(price_book_id));
    }

    #[test]
    fn double_initialization_is_rejected() {
        let (env, contract_id, admin, price_book_id) = setup();
        // The host only invokes a constructor once per real deployment;
        // env.as_contract lets this test call the guarded function again
        // directly, in the deployed contract's own storage context, to
        // prove the AlreadyInitialized guard actually fires.
        let result = env.as_contract(&contract_id, || {
            StatementRegistry::__constructor(env.clone(), admin.clone(), price_book_id.clone())
        });
        assert_eq!(result, Err(Error::AlreadyInitialized));
    }
}

mod anchor {
    use soroban_sdk::testutils::{Events as _, Ledger as _};
    use soroban_sdk::Event as _;

    use crate::event::AnchorEvent;
    use crate::types::Protocol;
    use crate::StatementRegistryClient;

    use super::*;

    #[test]
    fn happy_path_returns_incrementing_seq() {
        let (env, contract_id, _admin, price_book_id) = setup();
        env.mock_all_auths();
        let client = StatementRegistryClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let consumer = Address::generate(&env);
        let token = Address::generate(&env);
        let usage_root = BytesN::from_array(&env, &[42u8; 32]);

        let base = env.ledger().sequence();
        let version = publish_schedule(&env, &price_book_id, &operator, base);
        env.ledger().set_sequence_number(base + 100);

        let seq1 = client.anchor(
            &operator,
            &consumer,
            &base,
            &(base + 50),
            &usage_root,
            &10,
            &token,
            &1000i128,
            &900i128,
            &version,
            &Protocol::X402,
            &None,
        );
        assert_eq!(seq1, 1);

        let seq2 = client.anchor(
            &operator,
            &consumer,
            &(base + 50),
            &(base + 90),
            &usage_root,
            &10,
            &token,
            &1000i128,
            &900i128,
            &version,
            &Protocol::X402,
            &None,
        );
        assert_eq!(seq2, 2);
    }

    #[test]
    fn happy_path_emits_anchor_event() {
        let (env, contract_id, _admin, price_book_id) = setup();
        env.mock_all_auths();
        let client = StatementRegistryClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let consumer = Address::generate(&env);
        let token = Address::generate(&env);
        let usage_root = BytesN::from_array(&env, &[42u8; 32]);

        let base = env.ledger().sequence();
        let version = publish_schedule(&env, &price_book_id, &operator, base);
        env.ledger().set_sequence_number(base + 100);

        let seq = client.anchor(
            &operator,
            &consumer,
            &base,
            &(base + 50),
            &usage_root,
            &10,
            &token,
            &1000i128,
            &900i128,
            &version,
            &Protocol::X402,
            &None,
        );

        let expected = AnchorEvent {
            operator,
            consumer,
            seq,
            usage_root,
            amount_billed: 1000,
            amount_settled: 900,
            protocol: Protocol::X402,
        }
        .to_xdr(&env, &contract_id);
        assert_eq!(env.events().all(), std::vec![expected]);
    }

    // BadPeriod, EmptyStatement, and BadAmounts are all checked before the
    // price_book cross-contract call, so these tests pass an arbitrary
    // price_book_version (0) — it's never reached.

    #[test]
    fn bad_period_start_not_before_end_is_rejected() {
        let (env, contract_id, _admin, _price_book_id) = setup();
        env.mock_all_auths();
        let client = StatementRegistryClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let consumer = Address::generate(&env);
        let token = Address::generate(&env);
        let usage_root = BytesN::from_array(&env, &[42u8; 32]);
        let base = env.ledger().sequence();

        let result = client.try_anchor(
            &operator,
            &consumer,
            &base,
            &base, // period_start == period_end
            &usage_root,
            &10,
            &token,
            &1000i128,
            &900i128,
            &0,
            &Protocol::X402,
            &None,
        );
        assert_eq!(result, Err(Ok(Error::BadPeriod)));
    }

    #[test]
    fn bad_period_end_after_current_ledger_is_rejected() {
        let (env, contract_id, _admin, _price_book_id) = setup();
        env.mock_all_auths();
        let client = StatementRegistryClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let consumer = Address::generate(&env);
        let token = Address::generate(&env);
        let usage_root = BytesN::from_array(&env, &[42u8; 32]);
        let base = env.ledger().sequence();

        let result = client.try_anchor(
            &operator,
            &consumer,
            &base,
            &(base + 1), // period_end is in the future — the period hasn't finished
            &usage_root,
            &10,
            &token,
            &1000i128,
            &900i128,
            &0,
            &Protocol::X402,
            &None,
        );
        assert_eq!(result, Err(Ok(Error::BadPeriod)));
    }

    #[test]
    fn empty_statement_is_rejected() {
        let (env, contract_id, _admin, _price_book_id) = setup();
        env.mock_all_auths();
        let client = StatementRegistryClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let consumer = Address::generate(&env);
        let token = Address::generate(&env);
        let usage_root = BytesN::from_array(&env, &[42u8; 32]);
        let base = env.ledger().sequence();
        env.ledger().set_sequence_number(base + 100);

        let result = client.try_anchor(
            &operator,
            &consumer,
            &base,
            &(base + 1),
            &usage_root,
            &0, // request_count
            &token,
            &1000i128,
            &900i128,
            &0,
            &Protocol::X402,
            &None,
        );
        assert_eq!(result, Err(Ok(Error::EmptyStatement)));
    }

    #[test]
    fn bad_amounts_negative_billed_is_rejected() {
        let (env, contract_id, _admin, _price_book_id) = setup();
        env.mock_all_auths();
        let client = StatementRegistryClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let consumer = Address::generate(&env);
        let token = Address::generate(&env);
        let usage_root = BytesN::from_array(&env, &[42u8; 32]);
        let base = env.ledger().sequence();
        env.ledger().set_sequence_number(base + 100);

        let result = client.try_anchor(
            &operator,
            &consumer,
            &base,
            &(base + 1),
            &usage_root,
            &10,
            &token,
            &(-1i128),
            &0i128,
            &0,
            &Protocol::X402,
            &None,
        );
        assert_eq!(result, Err(Ok(Error::BadAmounts)));
    }

    #[test]
    fn bad_amounts_negative_settled_is_rejected() {
        let (env, contract_id, _admin, _price_book_id) = setup();
        env.mock_all_auths();
        let client = StatementRegistryClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let consumer = Address::generate(&env);
        let token = Address::generate(&env);
        let usage_root = BytesN::from_array(&env, &[42u8; 32]);
        let base = env.ledger().sequence();
        env.ledger().set_sequence_number(base + 100);

        let result = client.try_anchor(
            &operator,
            &consumer,
            &base,
            &(base + 1),
            &usage_root,
            &10,
            &token,
            &1000i128,
            &(-1i128),
            &0,
            &Protocol::X402,
            &None,
        );
        assert_eq!(result, Err(Ok(Error::BadAmounts)));
    }

    #[test]
    fn bad_amounts_settled_exceeds_billed_is_rejected() {
        let (env, contract_id, _admin, _price_book_id) = setup();
        env.mock_all_auths();
        let client = StatementRegistryClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let consumer = Address::generate(&env);
        let token = Address::generate(&env);
        let usage_root = BytesN::from_array(&env, &[42u8; 32]);
        let base = env.ledger().sequence();
        env.ledger().set_sequence_number(base + 100);

        let result = client.try_anchor(
            &operator,
            &consumer,
            &base,
            &(base + 1),
            &usage_root,
            &10,
            &token,
            &100i128,
            &101i128, // settled > billed
            &0,
            &Protocol::X402,
            &None,
        );
        assert_eq!(result, Err(Ok(Error::BadAmounts)));
    }

    // ChannelMismatch is also checked before the price_book call, so these
    // tests pass an arbitrary price_book_version (0) too.

    #[test]
    fn channel_missing_for_mpp_session_is_rejected() {
        let (env, contract_id, _admin, _price_book_id) = setup();
        env.mock_all_auths();
        let client = StatementRegistryClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let consumer = Address::generate(&env);
        let token = Address::generate(&env);
        let usage_root = BytesN::from_array(&env, &[42u8; 32]);
        let base = env.ledger().sequence();
        env.ledger().set_sequence_number(base + 100);

        let result = client.try_anchor(
            &operator,
            &consumer,
            &base,
            &(base + 50),
            &usage_root,
            &10,
            &token,
            &1000i128,
            &900i128,
            &0,
            &Protocol::MppSession,
            &None, // channel required for MppSession
        );
        assert_eq!(result, Err(Ok(Error::ChannelMismatch)));
    }

    #[test]
    fn channel_present_for_non_session_protocol_is_rejected() {
        let (env, contract_id, _admin, _price_book_id) = setup();
        env.mock_all_auths();
        let client = StatementRegistryClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let consumer = Address::generate(&env);
        let token = Address::generate(&env);
        let channel = Address::generate(&env);
        let usage_root = BytesN::from_array(&env, &[42u8; 32]);
        let base = env.ledger().sequence();
        env.ledger().set_sequence_number(base + 100);

        let result = client.try_anchor(
            &operator,
            &consumer,
            &base,
            &(base + 50),
            &usage_root,
            &10,
            &token,
            &1000i128,
            &900i128,
            &0,
            &Protocol::X402,
            &Some(channel), // channel must be absent outside MppSession
        );
        assert_eq!(result, Err(Ok(Error::ChannelMismatch)));
    }

    // The price_book check itself: anchored against a REAL deployed
    // price_book (never mocked — see setup()'s own doc comment), because
    // this is the check the whole contract exists for. PriceVersionUnknown
    // isn't named in its own build-sequence step, so it's covered here
    // alongside PriceVersionStale as the other half of the same
    // cross-contract validation block.

    #[test]
    fn price_version_stale_is_rejected() {
        let (env, contract_id, _admin, price_book_id) = setup();
        env.mock_all_auths();
        let client = StatementRegistryClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let consumer = Address::generate(&env);
        let token = Address::generate(&env);
        let usage_root = BytesN::from_array(&env, &[42u8; 32]);

        let base = env.ledger().sequence();
        // v1, effective immediately.
        let v1 = publish_schedule(&env, &price_book_id, &operator, base);
        // v2 supersedes v1 partway through the eventual statement period.
        let v2 = publish_schedule(&env, &price_book_id, &operator, base + 20);
        assert_eq!(v2, v1 + 1);
        env.ledger().set_sequence_number(base + 200);

        // period_end (base + 100) is after v2's effective_ledger
        // (base + 20), so version_at() returns v2 here — but the anchor
        // call claims v1, the price that was in force before the raise.
        // This is exactly the "anchor against a favourable old schedule"
        // attempt the check exists to stop.
        let result = client.try_anchor(
            &operator,
            &consumer,
            &base,
            &(base + 100),
            &usage_root,
            &10,
            &token,
            &1000i128,
            &900i128,
            &v1,
            &Protocol::X402,
            &None,
        );
        assert_eq!(result, Err(Ok(Error::PriceVersionStale)));
    }

    #[test]
    fn price_version_unknown_is_rejected() {
        let (env, contract_id, _admin, _price_book_id) = setup();
        env.mock_all_auths();
        let client = StatementRegistryClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let consumer = Address::generate(&env);
        let token = Address::generate(&env);
        let usage_root = BytesN::from_array(&env, &[42u8; 32]);
        let base = env.ledger().sequence();
        env.ledger().set_sequence_number(base + 100);

        // operator has never published anything on the real price_book —
        // version_at() itself errors, distinct from a version mismatch.
        let result = client.try_anchor(
            &operator,
            &consumer,
            &base,
            &(base + 50),
            &usage_root,
            &10,
            &token,
            &1000i128,
            &900i128,
            &1,
            &Protocol::X402,
            &None,
        );
        assert_eq!(result, Err(Ok(Error::PriceVersionUnknown)));
    }
}

mod merkle {
    use soroban_sdk::vec;

    use crate::merkle::fold;

    use super::*;

    #[test]
    fn single_leaf_with_empty_proof_returns_the_leaf_unchanged() {
        let env = Env::default();
        let leaf = BytesN::from_array(&env, &[7u8; 32]);

        let root = fold(&env, leaf.clone(), &vec![&env]);

        assert_eq!(root, leaf);
    }

    #[test]
    fn two_leaf_tree_matches_a_hand_computed_root() {
        let env = Env::default();
        let leaf0 = BytesN::from_array(&env, &[1u8; 32]);
        let leaf1 = BytesN::from_array(&env, &[2u8; 32]);
        let expected_root = hash_pair(&env, &leaf0, &leaf1);

        // Sorted-pair hashing means the root doesn't depend on which leaf
        // the proof starts from.
        assert_eq!(fold(&env, leaf0.clone(), &vec![&env, leaf1.clone()]), expected_root);
        assert_eq!(fold(&env, leaf1, &vec![&env, leaf0]), expected_root);
    }

    #[test]
    fn four_leaf_balanced_tree_matches_a_hand_computed_root() {
        let env = Env::default();
        let leaves: std::vec::Vec<BytesN<32>> =
            (0u8..4).map(|i| BytesN::from_array(&env, &[i + 1; 32])).collect();

        // Build the reference tree bottom-up: two internal nodes, then the
        // root over those.
        let node01 = hash_pair(&env, &leaves[0], &leaves[1]);
        let node23 = hash_pair(&env, &leaves[2], &leaves[3]);
        let expected_root = hash_pair(&env, &node01, &node23);

        let root = fold(&env, leaves[0].clone(), &vec![&env, leaves[1].clone(), node23.clone()]);
        assert_eq!(root, expected_root);

        // Every leaf's proof should fold to the same root.
        let root = fold(&env, leaves[2].clone(), &vec![&env, leaves[3].clone(), node01]);
        assert_eq!(root, expected_root);
    }
}

mod fixtures {
    use serde_json::Value;
    use soroban_sdk::vec;

    use crate::merkle::fold;

    use super::*;

    /// Parses a 64-hex-character digest into a BytesN<32>. Panics on
    /// malformed input — test-only, and a fixture that doesn't parse is a
    /// fixture bug worth failing loudly on.
    fn hex_to_bytesn(env: &Env, hex: &str) -> BytesN<32> {
        let mut bytes = [0u8; 32];
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap();
        }
        BytesN::from_array(env, &bytes)
    }

    /// Asserts every (leaf, proof) pair in the fixture folds to its root.
    fn assert_fixture_folds_to_root(json: &str) {
        let env = Env::default();
        let parsed: Value = serde_json::from_str(json).unwrap();
        let root = hex_to_bytesn(&env, parsed["root"].as_str().unwrap());

        for entry in parsed["proofs"].as_array().unwrap() {
            let leaf = hex_to_bytesn(&env, entry["leaf"].as_str().unwrap());
            let mut proof = vec![&env];
            for node in entry["proof"].as_array().unwrap() {
                proof.push_back(hex_to_bytesn(&env, node.as_str().unwrap()));
            }
            assert_eq!(fold(&env, leaf, &proof), root);
        }
    }

    #[test]
    fn single_leaf_fixture_folds_to_its_root() {
        assert_fixture_folds_to_root(include_str!("../../../fixtures/merkle/single-leaf.json"));
    }

    #[test]
    fn four_leaves_fixture_folds_to_its_root() {
        assert_fixture_folds_to_root(include_str!("../../../fixtures/merkle/four-leaves.json"));
    }

    #[test]
    fn seven_leaves_fixture_folds_to_its_root() {
        // The unbalanced tree: leaf index 6 is the odd one out at the first
        // level and gets a shorter proof than the rest — exactly the case
        // sorted-pair implementations tend to diverge on.
        assert_fixture_folds_to_root(include_str!("../../../fixtures/merkle/seven-leaves.json"));
    }

    #[test]
    fn corrupted_proof_node_does_not_reach_the_root() {
        let env = Env::default();
        let parsed: Value =
            serde_json::from_str(include_str!("../../../fixtures/merkle/four-leaves.json"))
                .unwrap();
        let root = hex_to_bytesn(&env, parsed["root"].as_str().unwrap());
        let entry = &parsed["proofs"][0];
        let leaf = hex_to_bytesn(&env, entry["leaf"].as_str().unwrap());

        let mut proof = vec![&env];
        for (i, node) in entry["proof"].as_array().unwrap().iter().enumerate() {
            let mut node_bytes = hex_to_bytesn(&env, node.as_str().unwrap()).to_array();
            if i == 0 {
                // Flip a bit in the first proof node — same shape as a
                // valid proof, wrong content.
                node_bytes[0] ^= 0xff;
            }
            proof.push_back(BytesN::from_array(&env, &node_bytes));
        }

        assert_ne!(fold(&env, leaf, &proof), root);
    }
}

mod price_book_import {
    use soroban_sdk::String;

    use super::*;

    /// Registers the *real* compiled price_book.wasm — never a mock — and
    /// drives it through the generated client, proving the contractimport!
    /// wiring actually works end to end: publish a schedule, then read it
    /// back via both get_version and version_at.
    #[test]
    fn real_wasm_registers_and_answers_queries() {
        let env = Env::default();
        env.mock_all_auths();

        let price_book_admin = Address::generate(&env);
        let price_book_id = env.register(crate::price_book::WASM, (price_book_admin,));
        let price_book_client = crate::price_book::Client::new(&env, &price_book_id);

        let operator = Address::generate(&env);
        let schedule_hash = BytesN::from_array(&env, &[9u8; 32]);
        let uri = String::from_str(&env, "https://example.com/schedule.json");
        let effective_ledger = env.ledger().sequence();

        let version = price_book_client.publish(&operator, &schedule_hash, &uri, &effective_ledger);
        assert_eq!(version, 1);

        let fetched = price_book_client.get_version(&operator, &version);
        assert_eq!(fetched.schedule_hash, schedule_hash);

        assert_eq!(price_book_client.version_at(&operator, &effective_ledger), version);
    }
}
