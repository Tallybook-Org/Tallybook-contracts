// Per CLAUDE.md's build sequence, event publishers land in their own commit
// before anchor()/open_dispute()/resolve_dispute() exist to construct and
// publish them. #[contractevent]'s generated code only reads fields (via
// &self) to serialize an instance, it never constructs one, so clippy's
// never-constructed check fires on all three structs until a real call
// site exists. Lifted once each event has a caller.
#![allow(dead_code)]

use soroban_sdk::{contractevent, Address, BytesN};

use crate::types::Protocol;

// All three events use #[contractevent] with data_format = "vec" (a plain
// positional list in field declaration order), matching price-book's
// migration off the deprecated Events::publish(topics, data) — same wire
// shape, current API, no #[allow(deprecated)] needed from the start here.

/// `topics: ("statement", "anchor")`
/// `data: (operator, consumer, seq, usage_root, amount_billed,
/// amount_settled, protocol)`
///
/// Publish only after the write it describes has landed, via
/// `AnchorEvent { .. }.publish(&env)`. Field order is part of the public
/// interface — an off-chain indexer reads it positionally.
#[contractevent(topics = ["statement", "anchor"], data_format = "vec")]
#[derive(Clone, Debug, PartialEq)]
pub struct AnchorEvent {
    pub operator: Address,
    pub consumer: Address,
    pub seq: u64,
    pub usage_root: BytesN<32>,
    pub amount_billed: i128,
    pub amount_settled: i128,
    pub protocol: Protocol,
}

/// `topics: ("statement", "dispute")`
/// `data: (operator, consumer, seq, reason_hash)`
#[contractevent(topics = ["statement", "dispute"], data_format = "vec")]
#[derive(Clone, Debug, PartialEq)]
pub struct DisputeEvent {
    pub operator: Address,
    pub consumer: Address,
    pub seq: u64,
    pub reason_hash: BytesN<32>,
}

/// `topics: ("statement", "resolve")`
/// `data: (operator, consumer, seq, resolution_hash, amount_credited)`
#[contractevent(topics = ["statement", "resolve"], data_format = "vec")]
#[derive(Clone, Debug, PartialEq)]
pub struct ResolveEvent {
    pub operator: Address,
    pub consumer: Address,
    pub seq: u64,
    pub resolution_hash: BytesN<32>,
    pub amount_credited: i128,
}
