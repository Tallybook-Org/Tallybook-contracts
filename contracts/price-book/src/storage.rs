// Per CLAUDE.md's build sequence, storage helpers land in their own commit
// before the constructor and publish() exist to call them. Lifted once
// lib.rs has a caller for every helper below.
#![allow(dead_code)]

use soroban_sdk::{contracttype, vec, Address, Env, Vec};

use crate::types::{PriceBookVersion, TimelineEntry};

/// Ledgers in a day, used to derive the TTL constants below.
const DAY_IN_LEDGERS: u32 = 17_280;
/// Extend TTL once the remaining TTL drops below this many ledgers.
const BUMP_THRESHOLD: u32 = 30 * DAY_IN_LEDGERS;
/// ...and extend it out to this many ledgers of headroom. A year of
/// headroom on a one-year bump is deliberate: these are audit records, and
/// the cost of a rent bump is trivial next to an unreadable statement.
const BUMP_AMOUNT: u32 = 365 * DAY_IN_LEDGERS;

/// `version_at()` binary-searches `Timeline(operator)` rather than scanning
/// every published `Version` key, so the vector is capped instead of left to
/// grow without limit. `publish()` errors with `TimelineFull` once reached.
/// Documented as a known limitation in the README.
pub const TIMELINE_CAP: u32 = 256;

#[contracttype]
pub enum DataKey {
    /// instance: the address that owns rent for this contract. Holds no
    /// power over operator data — see the constructor's doc comment.
    Admin,
    /// persistent: operator -> latest published version number.
    Latest(Address),
    /// persistent: (operator, version) -> the published PriceBookVersion.
    Version(Address, u32),
    /// persistent: operator -> Vec<TimelineEntry>, ascending by effective_ledger.
    Timeline(Address),
}

/// Extends the instance's own TTL. Called from the constructor and from
/// every state-changing call, per the workspace-wide TTL policy.
pub fn extend_instance_ttl(env: &Env) {
    env.storage().instance().extend_ttl(BUMP_THRESHOLD, BUMP_AMOUNT);
}

/// Whether `Admin` has already been set, i.e. the contract has already been
/// constructed.
pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

/// Sets `Admin`. Constructor-only; there is no setter after construction.
pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

/// The latest version number an operator has published, if any.
pub fn get_latest(env: &Env, operator: &Address) -> Option<u32> {
    env.storage().persistent().get(&DataKey::Latest(operator.clone()))
}

/// Sets and extends the TTL of `Latest(operator)`.
pub fn set_latest(env: &Env, operator: &Address, version: u32) {
    let key = DataKey::Latest(operator.clone());
    env.storage().persistent().set(&key, &version);
    env.storage().persistent().extend_ttl(&key, BUMP_THRESHOLD, BUMP_AMOUNT);
}

/// A single published version, if it exists.
pub fn get_version(env: &Env, operator: &Address, version: u32) -> Option<PriceBookVersion> {
    env.storage().persistent().get(&DataKey::Version(operator.clone(), version))
}

/// Sets and extends the TTL of `Version(operator, version)`.
pub fn set_version(env: &Env, operator: &Address, version: u32, pbv: &PriceBookVersion) {
    let key = DataKey::Version(operator.clone(), version);
    env.storage().persistent().set(&key, pbv);
    env.storage().persistent().extend_ttl(&key, BUMP_THRESHOLD, BUMP_AMOUNT);
}

/// An operator's timeline, or an empty vector if they have never published.
pub fn get_timeline(env: &Env, operator: &Address) -> Vec<TimelineEntry> {
    env.storage().persistent().get(&DataKey::Timeline(operator.clone())).unwrap_or(vec![env])
}

/// Sets and extends the TTL of `Timeline(operator)`.
pub fn set_timeline(env: &Env, operator: &Address, timeline: &Vec<TimelineEntry>) {
    let key = DataKey::Timeline(operator.clone());
    env.storage().persistent().set(&key, timeline);
    env.storage().persistent().extend_ttl(&key, BUMP_THRESHOLD, BUMP_AMOUNT);
}
