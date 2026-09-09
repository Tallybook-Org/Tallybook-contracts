use soroban_sdk::{contracttype, Address, BytesN};

/// Which payment mechanism a statement's charges were collected under.
#[contracttype]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Protocol {
    X402,
    MppCharge,
    MppSession,
}

/// A statement's lifecycle. There is no state after `Resolved` and no way
/// back from `Disputed` except through `resolve_dispute` — no arbiter, no
/// admin override, no timeout that auto-resolves in the operator's favour.
#[contracttype]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Anchored,
    Disputed,
    Resolved,
}

/// One billing period, anchored so a buyer can verify a single charge
/// against `usage_root`, check `price_book_version` against the price book,
/// and contest the whole statement if it's wrong.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Statement {
    pub operator: Address,
    /// The payer account (`G…`) for x402/MppCharge, or the channel
    /// contract address (`C…`) for MppSession.
    pub consumer: Address,
    /// Ledger, inclusive.
    pub period_start: u32,
    /// Ledger, inclusive.
    pub period_end: u32,
    /// sha256 merkle root over usage records.
    pub usage_root: BytesN<32>,
    pub request_count: u64,
    /// The SEP-41 asset this bill is denominated in — a record of which
    /// token, nothing more. This contract never holds, moves, or touches
    /// tokens.
    pub token: Address,
    /// Token's smallest unit.
    pub amount_billed: i128,
    /// The operator's unverifiable CLAIM of what actually landed on-chain
    /// for this period. The contract cannot check it and does not pretend
    /// to — what makes it meaningful is an off-chain indexer independently
    /// summing SAC transfer and channel events for the period and flagging
    /// mismatches.
    pub amount_settled: i128,
    pub price_book_version: u32,
    pub protocol: Protocol,
    /// `Some(..)` iff `protocol == MppSession`. In session mode this
    /// duplicates `consumer` by design — it makes the relationship
    /// explicit for indexers instead of inferred.
    pub channel: Option<Address>,
    /// Set by the contract, never by the caller.
    pub anchored_ledger: u32,
    /// Set by the contract, never by the caller.
    pub status: Status,
}

/// A public contest against a `Statement`. Its existence, not its outcome,
/// is the enforcement mechanism: an unresolved dispute leaves the statement
/// publicly `Disputed` forever if the two sides never agree.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Dispute {
    pub consumer: Address,
    /// sha256 of the off-chain complaint document.
    pub reason_hash: BytesN<32>,
    pub opened_ledger: u32,
    /// `None` until resolved.
    pub resolution_hash: Option<BytesN<32>>,
    /// `0` until resolved.
    pub amount_credited: i128,
    /// `None` until resolved.
    pub resolved_ledger: Option<u32>,
}
