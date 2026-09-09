use soroban_sdk::{contracttype, Address, BytesN, String};

/// One published price schedule for an operator. The schedule itself
/// (endpoints, units, per-unit prices) lives off-chain as canonical JSON;
/// only its hash and location are anchored here.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PriceBookVersion {
    pub operator: Address,
    pub version: u32,
    /// sha256 of the canonical JSON schedule.
    pub schedule_hash: BytesN<32>,
    /// `https://` or `ipfs://` location of the schedule document, max 200 bytes.
    pub uri: String,
    /// Ledger from which this schedule applies.
    pub effective_ledger: u32,
    /// Ledger this version was published at. Set by the contract, never by
    /// the caller.
    pub published_ledger: u32,
}

/// One entry in an operator's sorted timeline of effective ledgers, used to
/// binary-search "which version applied at ledger N" without an unbounded
/// scan over every published `PriceBookVersion`.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct TimelineEntry {
    pub effective_ledger: u32,
    pub version: u32,
}
