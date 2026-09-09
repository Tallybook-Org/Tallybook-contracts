use soroban_sdk::contracterror;

/// Discriminants are explicit and never renumbered once committed — an
/// indexer in another repo matches on these numbers.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotFound = 2,
    /// `effective_ledger` is before the current ledger.
    EffectiveInPast = 3,
    /// `effective_ledger` does not strictly exceed the previous version's.
    EffectiveNotAfter = 4,
    /// `Timeline` has reached `TIMELINE_CAP` entries.
    TimelineFull = 5,
    /// `uri` exceeds 200 bytes.
    UriTooLong = 6,
}
