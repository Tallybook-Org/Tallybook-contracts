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
    // alongside PriceVersionStale and PeriodSpansPriceChange as the other
    // halves of the same cross-contract validation block.

    #[test]
    fn period_straddling_price_change_is_rejected() {
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

        // period_start (base) is under v1; period_end (base + 100) is
        // after v2's effective_ledger (base + 20), so it's under v2. No
        // single price_book_version honestly covers this period —
        // rejected regardless of which version is claimed.
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
        assert_eq!(result, Err(Ok(Error::PeriodSpansPriceChange)));
    }

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
        let v1 = publish_schedule(&env, &price_book_id, &operator, base);
        let v2 = publish_schedule(&env, &price_book_id, &operator, base + 20);
        env.ledger().set_sequence_number(base + 200);

        // Both period_start and period_end (base + 50, base + 100) are
        // after v2's effective_ledger, so the period sits entirely inside
        // v2's window — no spanning. The claim of v1 is simply wrong.
        let result = client.try_anchor(
            &operator,
            &consumer,
            &(base + 50),
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
        // Sanity: v2 is what version_at would actually return here.
        assert_eq!(v2, v1 + 1);
    }

    #[test]
    fn claimed_version_newer_than_in_force_is_rejected() {
        // Inverse of price_version_stale_is_rejected: only one version
        // exists, the period sits entirely inside its window (no
        // spanning), but the claim names a version that hasn't been
        // published — newer than what's actually in force, not older.
        let (env, contract_id, _admin, price_book_id) = setup();
        env.mock_all_auths();
        let client = StatementRegistryClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let consumer = Address::generate(&env);
        let token = Address::generate(&env);
        let usage_root = BytesN::from_array(&env, &[42u8; 32]);

        let base = env.ledger().sequence();
        let v1 = publish_schedule(&env, &price_book_id, &operator, base);
        env.ledger().set_sequence_number(base + 200);

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
            &(v1 + 1), // claims a version that was never published
            &Protocol::X402,
            &None,
        );
        assert_eq!(result, Err(Ok(Error::PriceVersionStale)));
    }

    #[test]
    fn honest_period_fully_inside_one_version_window_succeeds() {
        // Multiple versions exist over the contract's lifetime, but the
        // chosen period sits entirely before the second one takes effect —
        // the mere existence of a later version elsewhere in time must not
        // false-positive as PeriodSpansPriceChange.
        let (env, contract_id, _admin, price_book_id) = setup();
        env.mock_all_auths();
        let client = StatementRegistryClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let consumer = Address::generate(&env);
        let token = Address::generate(&env);
        let usage_root = BytesN::from_array(&env, &[42u8; 32]);

        let base = env.ledger().sequence();
        let v1 = publish_schedule(&env, &price_book_id, &operator, base);
        publish_schedule(&env, &price_book_id, &operator, base + 50);
        env.ledger().set_sequence_number(base + 200);

        let seq = client.anchor(
            &operator,
            &consumer,
            &(base + 10),
            &(base + 40), // fully before v2's effective_ledger (base + 50)
            &usage_root,
            &10,
            &token,
            &1000i128,
            &900i128,
            &v1,
            &Protocol::X402,
            &None,
        );
        assert_eq!(seq, 1);
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

    #[test]
    fn unauthorized_caller_is_rejected() {
        use soroban_sdk::testutils::{MockAuth, MockAuthInvoke};
        use soroban_sdk::IntoVal;

        let (env, contract_id, _admin, _price_book_id) = setup();
        let operator = Address::generate(&env);
        let attacker = Address::generate(&env);
        let consumer = Address::generate(&env);
        let token = Address::generate(&env);
        let usage_root = BytesN::from_array(&env, &[42u8; 32]);
        let base = env.ledger().sequence();
        env.ledger().set_sequence_number(base + 100);

        let client = StatementRegistryClient::new(&env, &contract_id);

        // Authorize attacker, not operator — anchor() requires
        // operator.require_auth(), so this must fail even though *some*
        // valid auth entry is present.
        env.mock_auths(&[MockAuth {
            address: &attacker,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "anchor",
                args: (
                    operator.clone(),
                    consumer.clone(),
                    base,
                    base + 50,
                    usage_root.clone(),
                    10u64,
                    token.clone(),
                    1000i128,
                    900i128,
                    1u32,
                    Protocol::X402,
                    Option::<Address>::None,
                )
                    .into_val(&env),
                sub_invokes: &[],
            },
        }]);

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
        assert!(result.is_err());
    }

    #[test]
    fn index_full_is_rejected_once_cap_is_reached() {
        let (env, contract_id, _admin, price_book_id) = setup();
        env.mock_all_auths();
        let client = StatementRegistryClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let consumer = Address::generate(&env);
        let token = Address::generate(&env);
        let usage_root = BytesN::from_array(&env, &[42u8; 32]);

        let base = env.ledger().sequence();
        let version = publish_schedule(&env, &price_book_id, &operator, base);
        env.ledger().set_sequence_number(base + crate::storage::CONSUMER_IDX_CAP + 100);

        for i in 0..crate::storage::CONSUMER_IDX_CAP {
            client.anchor(
                &operator,
                &consumer,
                &(base + i),
                &(base + i + 1),
                &usage_root,
                &10,
                &token,
                &1000i128,
                &900i128,
                &version,
                &Protocol::X402,
                &None,
            );
        }

        let cap = crate::storage::CONSUMER_IDX_CAP;
        let result = client.try_anchor(
            &operator,
            &consumer,
            &(base + cap),
            &(base + cap + 1),
            &usage_root,
            &10,
            &token,
            &1000i128,
            &900i128,
            &version,
            &Protocol::X402,
            &None,
        );
        assert_eq!(result, Err(Ok(Error::IndexFull)));
    }
}

mod verify_usage {
    use serde_json::Value;
    use soroban_sdk::testutils::Ledger as _;
    use soroban_sdk::vec;

    use crate::types::Protocol;
    use crate::StatementRegistryClient;

    use super::*;

    fn hex_to_bytesn(env: &Env, hex: &str) -> BytesN<32> {
        let mut bytes = [0u8; 32];
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap();
        }
        BytesN::from_array(env, &bytes)
    }

    /// Anchors a minimal, otherwise-valid statement with `usage_root` and
    /// returns (Env, contract_id, operator, seq) for verify_usage calls
    /// against it.
    fn anchor_with_usage_root(usage_root: &BytesN<32>) -> (Env, Address, Address, u64) {
        let env = usage_root.env().clone();
        let admin = Address::generate(&env);
        let price_book_admin = Address::generate(&env);
        let price_book_id = env.register(crate::price_book::WASM, (price_book_admin,));
        let contract_id = env.register(StatementRegistry, (admin, price_book_id.clone()));
        env.mock_all_auths();
        let client = StatementRegistryClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let consumer = Address::generate(&env);
        let token = Address::generate(&env);

        let base = env.ledger().sequence();
        let version = publish_schedule(&env, &price_book_id, &operator, base);
        env.ledger().set_sequence_number(base + 100);

        let seq = client.anchor(
            &operator,
            &consumer,
            &base,
            &(base + 50),
            usage_root,
            &10,
            &token,
            &1000i128,
            &900i128,
            &version,
            &Protocol::X402,
            &None,
        );
        (env, contract_id, operator, seq)
    }

    fn assert_fixture_verifies(json: &str) {
        // hex_to_bytesn needs an Env before anchor_with_usage_root's own
        // setup exists, so build one just to parse the root — BytesN
        // values built against it are then handed into
        // anchor_with_usage_root, which reuses that same Env throughout.
        let env = Env::default();
        let parsed: Value = serde_json::from_str(json).unwrap();
        let usage_root = hex_to_bytesn(&env, parsed["root"].as_str().unwrap());

        let (env, contract_id, operator, seq) = anchor_with_usage_root(&usage_root);
        let client = StatementRegistryClient::new(&env, &contract_id);

        for entry in parsed["proofs"].as_array().unwrap() {
            let leaf = hex_to_bytesn(&env, entry["leaf"].as_str().unwrap());
            let mut proof = vec![&env];
            for node in entry["proof"].as_array().unwrap() {
                proof.push_back(hex_to_bytesn(&env, node.as_str().unwrap()));
            }
            assert!(client.verify_usage(&operator, &seq, &leaf, &proof));
        }
    }

    #[test]
    fn single_leaf_fixture_verifies() {
        assert_fixture_verifies(include_str!("../../../fixtures/merkle/single-leaf.json"));
    }

    #[test]
    fn four_leaves_fixture_verifies() {
        assert_fixture_verifies(include_str!("../../../fixtures/merkle/four-leaves.json"));
    }

    #[test]
    fn seven_leaves_fixture_verifies() {
        assert_fixture_verifies(include_str!("../../../fixtures/merkle/seven-leaves.json"));
    }

    #[test]
    fn corrupted_proof_returns_false_not_error() {
        let env = Env::default();
        let parsed: Value =
            serde_json::from_str(include_str!("../../../fixtures/merkle/four-leaves.json"))
                .unwrap();
        let usage_root = hex_to_bytesn(&env, parsed["root"].as_str().unwrap());

        let (env, contract_id, operator, seq) = anchor_with_usage_root(&usage_root);
        let client = StatementRegistryClient::new(&env, &contract_id);

        let entry = &parsed["proofs"][0];
        let leaf = hex_to_bytesn(&env, entry["leaf"].as_str().unwrap());
        let mut proof = vec![&env];
        for (i, node) in entry["proof"].as_array().unwrap().iter().enumerate() {
            let mut node_bytes = hex_to_bytesn(&env, node.as_str().unwrap()).to_array();
            if i == 0 {
                node_bytes[0] ^= 0xff;
            }
            proof.push_back(BytesN::from_array(&env, &node_bytes));
        }

        assert!(!client.verify_usage(&operator, &seq, &leaf, &proof));
    }

    #[test]
    fn not_found_for_missing_statement() {
        let (env, contract_id, _admin, _price_book_id) = setup();
        let client = StatementRegistryClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let leaf = BytesN::from_array(&env, &[1u8; 32]);

        let result = client.try_verify_usage(&operator, &1, &leaf, &vec![&env]);
        assert_eq!(result, Err(Ok(Error::NotFound)));
    }

    #[test]
    fn proof_too_long_is_rejected() {
        let (env, contract_id, _admin, _price_book_id) = setup();
        let client = StatementRegistryClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let leaf = BytesN::from_array(&env, &[1u8; 32]);

        let mut proof = vec![&env];
        for i in 0..(crate::storage::MAX_PROOF_NODES + 1) {
            proof.push_back(BytesN::from_array(&env, &[i as u8; 32]));
        }

        // No statement needs to exist: ProofTooLong is checked before the
        // storage lookup.
        let result = client.try_verify_usage(&operator, &1, &leaf, &proof);
        assert_eq!(result, Err(Ok(Error::ProofTooLong)));
    }
}

mod open_dispute {
    use soroban_sdk::testutils::{Events as _, Ledger as _};
    use soroban_sdk::Event as _;

    use crate::event::DisputeEvent;
    use crate::types::{Protocol, Status};
    use crate::StatementRegistryClient;

    use super::*;

    /// Anchors a minimal, otherwise-valid statement and returns
    /// (Env, contract_id, operator, consumer, seq).
    fn anchor_statement() -> (Env, Address, Address, Address, u64) {
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
        (env, contract_id, operator, consumer, seq)
    }

    #[test]
    fn happy_path_sets_status_disputed_and_writes_dispute() {
        let (env, contract_id, operator, consumer, seq) = anchor_statement();
        let client = StatementRegistryClient::new(&env, &contract_id);
        let reason_hash = BytesN::from_array(&env, &[7u8; 32]);

        client.open_dispute(&operator, &seq, &consumer, &reason_hash);

        // No public getters yet (they land in a later commit) — reach into
        // storage.rs directly to confirm the transition.
        let (status, dispute) = env.as_contract(&contract_id, || {
            (
                crate::storage::get_statement(&env, &operator, seq).unwrap().status,
                crate::storage::get_dispute(&env, &operator, seq).unwrap(),
            )
        });
        assert_eq!(status, Status::Disputed);
        assert_eq!(dispute.consumer, consumer);
        assert_eq!(dispute.reason_hash, reason_hash);
        assert_eq!(dispute.amount_credited, 0);
        assert_eq!(dispute.resolution_hash, None);
        assert_eq!(dispute.resolved_ledger, None);
    }

    #[test]
    fn happy_path_emits_dispute_event() {
        let (env, contract_id, operator, consumer, seq) = anchor_statement();
        let client = StatementRegistryClient::new(&env, &contract_id);
        let reason_hash = BytesN::from_array(&env, &[7u8; 32]);

        client.open_dispute(&operator, &seq, &consumer, &reason_hash);

        let expected =
            DisputeEvent { operator, consumer, seq, reason_hash }.to_xdr(&env, &contract_id);
        assert_eq!(env.events().all(), std::vec![expected]);
    }

    #[test]
    fn not_found_for_missing_statement() {
        let (env, contract_id, _admin, _price_book_id) = setup();
        env.mock_all_auths();
        let client = StatementRegistryClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let consumer = Address::generate(&env);
        let reason_hash = BytesN::from_array(&env, &[7u8; 32]);

        let result = client.try_open_dispute(&operator, &1, &consumer, &reason_hash);
        assert_eq!(result, Err(Ok(Error::NotFound)));
    }

    #[test]
    fn not_found_for_wrong_consumer_does_not_leak_existence() {
        let (env, contract_id, operator, _real_consumer, seq) = anchor_statement();
        let client = StatementRegistryClient::new(&env, &contract_id);
        let impostor = Address::generate(&env);
        let reason_hash = BytesN::from_array(&env, &[7u8; 32]);

        // A statement genuinely exists at (operator, seq), but not for
        // this consumer — must come back identical to a missing statement.
        let result = client.try_open_dispute(&operator, &seq, &impostor, &reason_hash);
        assert_eq!(result, Err(Ok(Error::NotFound)));
    }

    #[test]
    fn not_anchored_when_already_disputed() {
        let (env, contract_id, operator, consumer, seq) = anchor_statement();
        let client = StatementRegistryClient::new(&env, &contract_id);
        let reason_hash = BytesN::from_array(&env, &[7u8; 32]);

        client.open_dispute(&operator, &seq, &consumer, &reason_hash);

        let result = client.try_open_dispute(&operator, &seq, &consumer, &reason_hash);
        assert_eq!(result, Err(Ok(Error::NotAnchored)));
    }

    #[test]
    fn unauthorized_caller_is_rejected() {
        use soroban_sdk::testutils::{MockAuth, MockAuthInvoke};
        use soroban_sdk::IntoVal;

        let (env, contract_id, operator, consumer, seq) = anchor_statement();
        let client = StatementRegistryClient::new(&env, &contract_id);
        let attacker = Address::generate(&env);
        let reason_hash = BytesN::from_array(&env, &[7u8; 32]);

        // Authorize attacker, not consumer — open_dispute() requires
        // consumer.require_auth(). Confirmed separately that mock_auths
        // here correctly switches auth enforcement back to strict, even
        // though anchor_statement() used mock_all_auths for the setup call.
        env.mock_auths(&[MockAuth {
            address: &attacker,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "open_dispute",
                args: (operator.clone(), seq, consumer.clone(), reason_hash.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);

        let result = client.try_open_dispute(&operator, &seq, &consumer, &reason_hash);
        assert!(result.is_err());
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
