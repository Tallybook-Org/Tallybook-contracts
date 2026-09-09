#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::{Error, PriceBook};

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
