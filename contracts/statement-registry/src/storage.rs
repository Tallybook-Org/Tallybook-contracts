// Per CLAUDE.md's build sequence, storage helpers land in their own commit
// before the constructor and the functions that call them exist. Lifted
// once lib.rs has a caller for every helper below.
#![allow(dead_code)]

use soroban_sdk::{contracttype, vec, Address, Env, Vec};

use crate::types::{Dispute, Statement};

/// Ledgers in a day, used to derive the TTL constants below.
const DAY_IN_LEDGERS: u32 = 17_280;
/// Extend TTL once the remaining TTL drops below this many ledgers.
const BUMP_THRESHOLD: u32 = 30 * DAY_IN_LEDGERS;
/// ...and extend it out to this many ledgers of headroom. A year of
/// headroom on a one-year bump is deliberate: these are audit records, and
/// the cost of a rent bump is trivial next to an unreadable statement.
const BUMP_AMOUNT: u32 = 365 * DAY_IN_LEDGERS;

/// `list_statements()` for one (operator, consumer) pair is a Vec of
/// sequence numbers, capped rather than left to grow without limit.
/// `anchor()` errors with `IndexFull` once reached. Documented as a known
/// limitation in the README.
pub const CONSUMER_IDX_CAP: u32 = 500;

/// 32 nodes covers over 4 billion leaves; `verify_usage()` errors with
/// `ProofTooLong` above this rather than accepting an unbounded proof.
pub const MAX_PROOF_NODES: u32 = 32;

#[contracttype]
pub enum DataKey {
    /// instance: the address that owns rent for this contract. Holds no
    /// power over operator data.
    Admin,
    /// instance: the price_book contract this registry checks prices
    /// against. Immutable after construction — no setter exists.
    PriceBook,
    /// persistent: operator -> last assigned sequence number.
    Seq(Address),
    /// persistent: (operator, seq) -> the anchored Statement.
    Statement(Address, u64),
    /// persistent: (operator, consumer) -> Vec<u64> of sequence numbers,
    /// oldest first.
    ConsumerIdx(Address, Address),
    /// persistent: (operator, seq) -> the Dispute against that statement.
    Dispute(Address, u64),
}

/// Extends the instance's own TTL. Called from the constructor and from
/// every state-changing call, per the workspace-wide TTL policy.
pub fn extend_instance_ttl(env: &Env) {
    env.storage().instance().extend_ttl(BUMP_THRESHOLD, BUMP_AMOUNT);
}

/// Whether `Admin` has been set. Test-only: production code has no reason
/// to check this — per CAP-0058 the constructor runs exactly once and is
/// never callable again — but the constructor test still wants a way to
/// prove the write landed without a public getter for Admin.
#[cfg(test)]
pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

/// Sets `Admin`. Constructor-only; there is no setter after construction.
pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

/// The price_book contract address, if the contract has been constructed.
pub fn get_price_book(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::PriceBook)
}

/// Sets `PriceBook`. Constructor-only; there is no setter after
/// construction — a mutable price book address would let an operator swap
/// in a permissive registry and invalidate every historical statement.
pub fn set_price_book(env: &Env, price_book: &Address) {
    env.storage().instance().set(&DataKey::PriceBook, price_book);
}

/// The last sequence number assigned to `operator`, if any.
pub fn get_seq(env: &Env, operator: &Address) -> Option<u64> {
    env.storage().persistent().get(&DataKey::Seq(operator.clone()))
}

/// Sets and extends the TTL of `Seq(operator)`.
pub fn set_seq(env: &Env, operator: &Address, seq: u64) {
    let key = DataKey::Seq(operator.clone());
    env.storage().persistent().set(&key, &seq);
    env.storage().persistent().extend_ttl(&key, BUMP_THRESHOLD, BUMP_AMOUNT);
}

/// A single anchored statement, if it exists.
pub fn get_statement(env: &Env, operator: &Address, seq: u64) -> Option<Statement> {
    env.storage().persistent().get(&DataKey::Statement(operator.clone(), seq))
}

/// Sets and extends the TTL of `Statement(operator, seq)`.
pub fn set_statement(env: &Env, operator: &Address, seq: u64, statement: &Statement) {
    let key = DataKey::Statement(operator.clone(), seq);
    env.storage().persistent().set(&key, statement);
    env.storage().persistent().extend_ttl(&key, BUMP_THRESHOLD, BUMP_AMOUNT);
}

/// The sequence numbers anchored for (operator, consumer), oldest first, or
/// an empty vector if there are none.
pub fn get_consumer_idx(env: &Env, operator: &Address, consumer: &Address) -> Vec<u64> {
    env.storage()
        .persistent()
        .get(&DataKey::ConsumerIdx(operator.clone(), consumer.clone()))
        .unwrap_or(vec![env])
}

/// Sets and extends the TTL of `ConsumerIdx(operator, consumer)`.
pub fn set_consumer_idx(env: &Env, operator: &Address, consumer: &Address, idx: &Vec<u64>) {
    let key = DataKey::ConsumerIdx(operator.clone(), consumer.clone());
    env.storage().persistent().set(&key, idx);
    env.storage().persistent().extend_ttl(&key, BUMP_THRESHOLD, BUMP_AMOUNT);
}

/// The dispute against (operator, seq), if one has been opened.
pub fn get_dispute(env: &Env, operator: &Address, seq: u64) -> Option<Dispute> {
    env.storage().persistent().get(&DataKey::Dispute(operator.clone(), seq))
}

/// Sets and extends the TTL of `Dispute(operator, seq)`.
pub fn set_dispute(env: &Env, operator: &Address, seq: u64, dispute: &Dispute) {
    let key = DataKey::Dispute(operator.clone(), seq);
    env.storage().persistent().set(&key, dispute);
    env.storage().persistent().extend_ttl(&key, BUMP_THRESHOLD, BUMP_AMOUNT);
}
