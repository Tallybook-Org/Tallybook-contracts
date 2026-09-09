#![cfg(test)]

// The crate is #![no_std]; tests need std for the standard test harness and
// for building throwaway data like an over-length URI below.
extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::{Error, PriceBook};

/// A distinct 32-byte value per `seed`, standing in for a real sha256
/// schedule hash — publish() never inspects its contents.
fn schedule_hash(env: &Env, seed: u8) -> soroban_sdk::BytesN<32> {
    soroban_sdk::BytesN::from_array(env, &[seed; 32])
}

/// Registers a fresh `PriceBook` with a generated admin. Auth is not
/// mocked here — individual tests opt into `env.mock_all_auths()` or
/// explicit `mock_auths` themselves, since which is correct differs per
/// test (happy path vs. negative auth path).
fn setup() -> (Env, Address, Address) {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(PriceBook, (admin.clone(),));
    (env, contract_id, admin)
}

mod constructor {
    use super::*;

    #[test]
    fn happy_path_sets_admin() {
        // There is no public getter for Admin — by design, it holds no
        // power and nothing reads it back — so reach into storage.rs
        // directly (crate-internal) to prove the write actually landed.
        let (env, contract_id, _admin) = setup();
        let admin_is_set = env.as_contract(&contract_id, || crate::storage::has_admin(&env));
        assert!(admin_is_set);
    }
}

mod publish {
    use soroban_sdk::{testutils::Events as _, Event as _, IntoVal, String};

    use crate::{event::PublishEvent, PriceBookClient};

    use super::*;

    #[test]
    fn happy_path_returns_incrementing_versions() {
        let (env, contract_id, _admin) = setup();
        env.mock_all_auths();
        let client = PriceBookClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let uri = String::from_str(&env, "https://example.com/schedule.json");
        let current_ledger = env.ledger().sequence();

        let v1 = client.publish(&operator, &schedule_hash(&env, 1), &uri, &current_ledger);
        assert_eq!(v1, 1);

        let v2 = client.publish(&operator, &schedule_hash(&env, 2), &uri, &(current_ledger + 1));
        assert_eq!(v2, 2);
    }

    #[test]
    fn happy_path_emits_publish_event() {
        let (env, contract_id, _admin) = setup();
        env.mock_all_auths();
        let client = PriceBookClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let hash = schedule_hash(&env, 7);
        let uri = String::from_str(&env, "https://example.com/schedule.json");
        let current_ledger = env.ledger().sequence();

        let version = client.publish(&operator, &hash, &uri, &current_ledger);

        let expected = PublishEvent {
            operator,
            version,
            schedule_hash: hash,
            effective_ledger: current_ledger,
        }
        .to_xdr(&env, &contract_id);
        assert_eq!(env.events().all(), std::vec![expected]);
    }

    #[test]
    fn uri_too_long_is_rejected() {
        let (env, contract_id, _admin) = setup();
        env.mock_all_auths();
        let client = PriceBookClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let long_uri = "a".repeat(201);
        let uri = String::from_str(&env, &long_uri);
        let current_ledger = env.ledger().sequence();

        let result = client.try_publish(&operator, &schedule_hash(&env, 1), &uri, &current_ledger);
        assert_eq!(result, Err(Ok(Error::UriTooLong)));
    }

    #[test]
    fn effective_in_past_is_rejected() {
        use soroban_sdk::testutils::Ledger as _;

        let (env, contract_id, _admin) = setup();
        env.mock_all_auths();
        env.ledger().set_sequence_number(1_000);
        let client = PriceBookClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let uri = String::from_str(&env, "https://example.com/schedule.json");

        let result = client.try_publish(&operator, &schedule_hash(&env, 1), &uri, &999);
        assert_eq!(result, Err(Ok(Error::EffectiveInPast)));
    }

    #[test]
    fn effective_not_after_previous_is_rejected() {
        let (env, contract_id, _admin) = setup();
        env.mock_all_auths();
        let client = PriceBookClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let uri = String::from_str(&env, "https://example.com/schedule.json");
        let current_ledger = env.ledger().sequence();

        client.publish(&operator, &schedule_hash(&env, 1), &uri, &current_ledger);

        // Not strictly after the first version's effective_ledger: equal is
        // rejected, same as anything lower would be.
        let result = client.try_publish(&operator, &schedule_hash(&env, 2), &uri, &current_ledger);
        assert_eq!(result, Err(Ok(Error::EffectiveNotAfter)));
    }

    #[test]
    fn timeline_full_is_rejected_once_cap_is_reached() {
        let (env, contract_id, _admin) = setup();
        env.mock_all_auths();
        let client = PriceBookClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let uri = String::from_str(&env, "https://example.com/schedule.json");
        let current_ledger = env.ledger().sequence();

        for i in 0..crate::storage::TIMELINE_CAP {
            client.publish(&operator, &schedule_hash(&env, 1), &uri, &(current_ledger + i));
        }

        let result = client.try_publish(
            &operator,
            &schedule_hash(&env, 1),
            &uri,
            &(current_ledger + crate::storage::TIMELINE_CAP),
        );
        assert_eq!(result, Err(Ok(Error::TimelineFull)));
    }

    #[test]
    fn unauthorized_caller_is_rejected() {
        use soroban_sdk::testutils::{MockAuth, MockAuthInvoke};

        let (env, contract_id, _admin) = setup();
        let client = PriceBookClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let attacker = Address::generate(&env);
        let hash = schedule_hash(&env, 1);
        let uri = String::from_str(&env, "https://example.com/schedule.json");
        let current_ledger = env.ledger().sequence();

        // Authorize attacker, not operator — publish() requires
        // operator.require_auth(), so this must fail even though *some*
        // valid auth entry is present.
        env.mock_auths(&[MockAuth {
            address: &attacker,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "publish",
                args: (operator.clone(), hash.clone(), uri.clone(), current_ledger).into_val(&env),
                sub_invokes: &[],
            },
        }]);

        let result = client.try_publish(&operator, &hash, &uri, &current_ledger);
        assert!(result.is_err());
    }
}

mod get_version {
    use soroban_sdk::String;

    use crate::PriceBookClient;

    use super::*;

    #[test]
    fn happy_path_returns_the_published_version() {
        let (env, contract_id, _admin) = setup();
        env.mock_all_auths();
        let client = PriceBookClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let hash = schedule_hash(&env, 1);
        let uri = String::from_str(&env, "https://example.com/schedule.json");
        let current_ledger = env.ledger().sequence();

        let version = client.publish(&operator, &hash, &uri, &current_ledger);
        let published = client.get_version(&operator, &version);

        assert_eq!(published.operator, operator);
        assert_eq!(published.version, version);
        assert_eq!(published.schedule_hash, hash);
        assert_eq!(published.uri, uri);
        assert_eq!(published.effective_ledger, current_ledger);
        assert_eq!(published.published_ledger, current_ledger);
    }

    #[test]
    fn not_found_for_unpublished_version() {
        let (env, contract_id, _admin) = setup();
        let client = PriceBookClient::new(&env, &contract_id);
        let operator = Address::generate(&env);

        let result = client.try_get_version(&operator, &1);
        assert_eq!(result, Err(Ok(Error::NotFound)));
    }
}

mod latest {
    use soroban_sdk::String;

    use crate::PriceBookClient;

    use super::*;

    #[test]
    fn happy_path_returns_the_highest_published_version() {
        let (env, contract_id, _admin) = setup();
        env.mock_all_auths();
        let client = PriceBookClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let uri = String::from_str(&env, "https://example.com/schedule.json");
        let current_ledger = env.ledger().sequence();

        client.publish(&operator, &schedule_hash(&env, 1), &uri, &current_ledger);
        client.publish(&operator, &schedule_hash(&env, 2), &uri, &(current_ledger + 1));

        assert_eq!(client.latest(&operator), 2);
    }

    #[test]
    fn not_found_for_an_operator_that_never_published() {
        let (env, contract_id, _admin) = setup();
        let client = PriceBookClient::new(&env, &contract_id);
        let operator = Address::generate(&env);

        let result = client.try_latest(&operator);
        assert_eq!(result, Err(Ok(Error::NotFound)));
    }
}

mod version_at {
    use soroban_sdk::String;

    use crate::PriceBookClient;

    use super::*;

    #[test]
    fn empty_timeline_is_not_found() {
        let (env, contract_id, _admin) = setup();
        let client = PriceBookClient::new(&env, &contract_id);
        let operator = Address::generate(&env);

        let result = client.try_version_at(&operator, &0);
        assert_eq!(result, Err(Ok(Error::NotFound)));
    }

    #[test]
    fn before_first_version_is_not_found() {
        let (env, contract_id, _admin) = setup();
        env.mock_all_auths();
        let client = PriceBookClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let uri = String::from_str(&env, "https://example.com/schedule.json");
        let current_ledger = env.ledger().sequence();
        let first_effective = current_ledger + 10;

        client.publish(&operator, &schedule_hash(&env, 1), &uri, &first_effective);

        let result = client.try_version_at(&operator, &(first_effective - 1));
        assert_eq!(result, Err(Ok(Error::NotFound)));
    }

    #[test]
    fn exactly_equal_ledger_returns_that_version() {
        let (env, contract_id, _admin) = setup();
        env.mock_all_auths();
        let client = PriceBookClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let uri = String::from_str(&env, "https://example.com/schedule.json");
        let current_ledger = env.ledger().sequence();

        let version = client.publish(&operator, &schedule_hash(&env, 1), &uri, &current_ledger);

        assert_eq!(client.version_at(&operator, &current_ledger), version);
    }

    #[test]
    fn after_last_version_returns_the_latest_version() {
        let (env, contract_id, _admin) = setup();
        env.mock_all_auths();
        let client = PriceBookClient::new(&env, &contract_id);
        let operator = Address::generate(&env);
        let uri = String::from_str(&env, "https://example.com/schedule.json");
        let current_ledger = env.ledger().sequence();

        client.publish(&operator, &schedule_hash(&env, 1), &uri, &current_ledger);
        let v2 = client.publish(&operator, &schedule_hash(&env, 2), &uri, &(current_ledger + 5));

        assert_eq!(client.version_at(&operator, &(current_ledger + 1_000)), v2);
    }
}
