use soroban_sdk::{contractevent, Address, BytesN};

/// `topics: ("price_book", "publish")`
/// `data: (operator, version, schedule_hash, effective_ledger)`, in field
/// declaration order (`data_format = "vec"` — a plain positional list, not
/// the macro's default per-field map).
///
/// Publish only after the write it describes has landed, via
/// `PublishEvent { .. }.publish(&env)`. Field order is part of the public
/// interface — an off-chain indexer reads it positionally — so it must
/// never be reordered.
#[contractevent(topics = ["price_book", "publish"], data_format = "vec")]
#[derive(Clone, Debug, PartialEq)]
pub struct PublishEvent {
    pub operator: Address,
    pub version: u32,
    pub schedule_hash: BytesN<32>,
    pub effective_ledger: u32,
}
