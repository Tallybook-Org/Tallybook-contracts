// soroban-sdk 27.0.6 deprecates Events::publish(topics, data) in favour of
// the #[contractevent] struct macro. The macro's `data_format = "vec"` mode
// would reproduce this wire format, but it publishes via a struct-with-
// fields, not the "function per event" with an explicit topics/data tuple
// that CLAUDE.md §5/§6 specify, and topic/data shape here is a byte-for-
// byte interface another repo parses positionally — not something to
// restructure on a lint's say-so. Keeping the deprecated call, scoped and
// documented, is the safer choice.
#![allow(deprecated)]

use soroban_sdk::{Address, BytesN, Env, Symbol};

/// `topics: ("price_book", "publish")`
/// `data: (operator, version, schedule_hash, effective_ledger)`
///
/// Published only after the write it describes has landed. Field order is
/// part of the public interface — an off-chain indexer reads it
/// positionally — so it must never be reordered.
pub fn publish(
    env: &Env,
    operator: Address,
    version: u32,
    schedule_hash: BytesN<32>,
    effective_ledger: u32,
) {
    let topics = (Symbol::new(env, "price_book"), Symbol::new(env, "publish"));
    env.events().publish(topics, (operator, version, schedule_hash, effective_ledger));
}
