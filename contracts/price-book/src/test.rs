#![cfg(test)]

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
        // directly (crate-internal) to prove the write actually landed,
        // rather than only inferring it from the double-init guard below.
        let (env, contract_id, _admin) = setup();
        let admin_is_set = env.as_contract(&contract_id, || crate::storage::has_admin(&env));
        assert!(admin_is_set);
    }

    #[test]
    fn double_initialization_is_rejected() {
        let (env, contract_id, admin) = setup();
        // The host only invokes a constructor once per real deployment;
        // env.as_contract lets this test call the guarded function again
        // directly, in the deployed contract's own storage context, to
        // prove the AlreadyInitialized guard actually fires.
        let result =
            env.as_contract(&contract_id, || PriceBook::__constructor(env.clone(), admin.clone()));
        assert_eq!(result, Err(Error::AlreadyInitialized));
    }
}

mod publish {
    use soroban_sdk::{testutils::Events as _, vec, IntoVal, String, Symbol};

    use crate::PriceBookClient;

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

        assert_eq!(
            env.events().all(),
            vec![
                &env,
                (
                    contract_id.clone(),
                    (Symbol::new(&env, "price_book"), Symbol::new(&env, "publish")).into_val(&env),
                    (operator, version, hash, current_ledger).into_val(&env),
                ),
            ]
        );
    }
}
