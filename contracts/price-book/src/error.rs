use soroban_sdk::contracterror;

/// Discriminants are explicit and never renumbered once committed — an
/// indexer in another repo matches on these numbers.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    // 1 is deliberately unused. It was AlreadyInitialized, guarding
    // __constructor against a second call — but per CAP-0058, a contract's
    // constructor is only ever invoked once at creation and is never
    // callable again, so that error was unreachable on-chain. Removed
    // rather than renumbered, per the no-renumbering rule above.
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
