//! `price-book`: an append-only, versioned record of what an operator
//! charges and from when. See the workspace README for the full interface.
#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env};

mod error;
mod event;
mod storage;
mod test;
mod types;

use error::Error;

#[contract]
pub struct PriceBook;

#[contractimpl]
impl PriceBook {
    /// Deploy-time only. Stores `admin` in instance storage and extends the
    /// instance TTL.
    ///
    /// `admin` holds no power over operator data — it cannot publish, edit,
    /// or remove a version. It exists only as a documented owner for future
    /// rent funding; there is no admin-gated function anywhere in this
    /// contract.
    ///
    /// Returns `AlreadyInitialized` if called a second time. The host only
    /// invokes a constructor once per deployment, so this guards against a
    /// direct second call rather than a real deployment scenario — cheap
    /// insurance against a construction path this contract does not expect.
    pub fn __constructor(env: Env, admin: Address) -> Result<(), Error> {
        if storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }
        storage::set_admin(&env, &admin);
        storage::extend_instance_ttl(&env);
        Ok(())
    }
}
