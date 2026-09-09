use soroban_sdk::contracterror;

/// Discriminants are explicit and never renumbered once committed — an
/// indexer in another repo matches on these numbers.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotFound = 2,
    /// `period_start >= period_end`, or `period_end > current ledger`.
    BadPeriod = 3,
    /// A negative amount, or `amount_settled > amount_billed`.
    BadAmounts = 4,
    /// `request_count == 0`.
    EmptyStatement = 5,
    /// `price_book` has no such version for this operator.
    PriceVersionUnknown = 6,
    /// That version was not in force at `period_end`.
    PriceVersionStale = 7,
    /// `channel` set without `MppSession`, or absent with it.
    ChannelMismatch = 8,
    /// `CONSUMER_IDX_CAP` reached.
    IndexFull = 9,
    /// A dispute was opened on a statement that is not `Anchored`.
    NotAnchored = 10,
    /// `resolve_dispute` called on a statement that is not `Disputed`.
    NotDisputed = 11,
    /// `amount_credited > amount_billed`.
    CreditTooLarge = 12,
    /// A merkle proof longer than `MAX_PROOF_NODES`.
    ProofTooLong = 13,
}
