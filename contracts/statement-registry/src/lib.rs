//! `statement-registry`: anchors a billing period's statement so a buyer
//! can verify one charge against it, check it against the price then in
//! force, and contest it publicly if it is wrong. See the workspace README
//! for the full interface.
#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env};

mod error;
mod event;
mod merkle;
mod price_book;
mod storage;
mod test;
mod types;

use error::Error;

#[contract]
pub struct StatementRegistry;

#[contractimpl]
impl StatementRegistry {
    /// Deploy-time only. Stores `admin` and `price_book` in instance
    /// storage and extends the instance TTL.
    ///
    /// `admin` holds no power over operator data. `price_book` is
    /// immutable after construction — there is no setter. A mutable price
    /// book address would let an operator swap in a permissive registry
    /// and invalidate every historical statement; if the price book must
    /// change, a new `statement_registry` is deployed.
    ///
    /// Errors: `AlreadyInitialized` if called a second time.
    pub fn __constructor(env: Env, admin: Address, price_book: Address) -> Result<(), Error> {
        if storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }
        storage::set_admin(&env, &admin);
        storage::set_price_book(&env, &price_book);
        storage::extend_instance_ttl(&env);
        Ok(())
    }
}
