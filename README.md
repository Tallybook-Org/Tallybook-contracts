# Tallybook-contracts
Tallybook is settlement bookkeeping for services that charge per HTTP request - APIs paid by AI agents and other machines over Stellar, using the x402 protocol, MPP charge mode, or MPP payment channels.

## Build

This repo contains two contracts: `price-book` has no dependencies; `statement-registry`
imports `price-book`'s compiled wasm at compile time (via `contractimport!`), so
`price-book` must be built first.

```sh
make build   # builds price-book, then statement-registry, via `stellar contract build`
make test    # cargo test --workspace
make fmt     # cargo fmt --all
make clippy  # cargo clippy --workspace --all-targets -- -D warnings
```

**On a clean checkout, run `make build` (or at least
`stellar contract build --package price-book`) before `cargo test` or `cargo clippy`.**
`statement-registry/src/price_book.rs` contractimports
`target/wasm32v1-none/release/price_book.wasm` unconditionally, so `statement-registry`
cannot even compile — for a test run, a clippy pass, or an IDE's background check — until
that file exists on disk. This is a real compile-time dependency, not just a build-order
nicety; `make build` and CI both encode it, but a bare `cargo test` on a fresh clone will
fail with a file-not-found error from the `contractimport!` macro until you build price-book
at least once.

Contracts are built exclusively with `stellar contract build`, targeting `wasm32v1-none` —
never with `cargo build`.

## `price-book`

An append-only, versioned record of what an operator charges and from when. Without it, the
amounts in a statement are unfalsifiable — a buyer cannot distinguish an honest bill from a
retroactively raised price. The full price schedule (endpoints, units, per-unit prices) lives
off-chain as canonical JSON; only its `sha256` hash and its location go on-chain.

### Functions

- **`__constructor(admin: Address)`** — deploy-time only. Stores `admin` in instance storage.
  `admin` holds no power over operator data — it cannot publish, edit, or remove a version. It
  exists only as a documented owner for future rent funding; there is no admin-gated function
  anywhere in this contract.

- **`publish(operator: Address, schedule_hash: BytesN<32>, uri: String, effective_ledger: u32) -> Result<u32, Error>`**
  — callable only by `operator`. Publishes a new price schedule version. Schedules must move
  strictly forward in time: `effective_ledger` may equal the current ledger (effective
  immediately) but not precede it, and must strictly exceed the previous version's
  `effective_ledger` if one exists. `uri` must be at most 200 bytes. Returns the new version
  number (the first published version is `1`).
  Errors: `UriTooLong`, `EffectiveInPast`, `EffectiveNotAfter`, `TimelineFull`.

- **`get_version(operator: Address, version: u32) -> Result<PriceBookVersion, Error>`** — no
  auth, read-only. Errors: `NotFound`.

- **`latest(operator: Address) -> Result<u32, Error>`** — no auth, read-only. The latest
  version number `operator` has published. Errors: `NotFound` if `operator` has never
  published.

- **`version_at(operator: Address, ledger: u32) -> Result<u32, Error>`** — no auth,
  read-only. Returns which version of `operator`'s schedule was in force at `ledger`: the
  version with the highest `effective_ledger` not exceeding `ledger`. This is the function
  `statement-registry` calls, and the function a buyer calls to check which prices applied on
  the day they were billed. Binary searches the operator's timeline rather than scanning every
  published version. Errors: `NotFound` if the timeline is empty or every entry's
  `effective_ledger` is after `ledger`.

### Errors

| Discriminant | Variant | Meaning |
|---|---|---|
| 1 | *(unused)* | Was `AlreadyInitialized`. Per [CAP-0058](https://stellar.org/protocol/cap-58), a contract's constructor is invoked exactly once, at creation, and is never callable again — the guard this occupied was unreachable on-chain, and the discriminant is left unused rather than renumbered. |
| 2 | `NotFound` | No such version, or operator has never published. |
| 3 | `EffectiveInPast` | `effective_ledger` is before the current ledger. |
| 4 | `EffectiveNotAfter` | `effective_ledger` does not strictly exceed the previous version's. |
| 5 | `TimelineFull` | `TIMELINE_CAP` (256) reached — see Known limitations. |
| 6 | `UriTooLong` | `uri` exceeds 200 bytes. |

### Events

`publish` — topics `("price_book", "publish")`, data `(operator: Address, version: u32, schedule_hash: BytesN<32>, effective_ledger: u32)`.

### Known limitations

`Timeline(operator)` — the sorted vector `version_at()` binary-searches — is capped at
**256 entries** (`TIMELINE_CAP`). `publish()` returns `TimelineFull` once an operator's
256th version would be appended. This is a deliberate, documented limit, not a hidden one:
an operator who needs more than 256 price changes over the life of a deployment needs a new
`price-book` deployment (or a design change to this contract), not an unbounded vector.

## `statement-registry`

Anchors a billing period's statement so a buyer can verify one charge against it, check it
against the price then in force, and contest it publicly if it is wrong. Reads `price-book`
via a cross-contract call; the `price-book` address is set at construction and is immutable.

### Functions

- **`__constructor(admin: Address, price_book: Address)`** — deploy-time only. Stores both in
  instance storage. `admin` holds no power over operator data. `price_book` is immutable
  after construction — there is no setter. A mutable price book address would let an operator
  swap in a permissive registry and invalidate every historical statement; if the price book
  must change, a new `statement-registry` is deployed.

- **`anchor(operator, consumer, period_start: u32, period_end: u32, usage_root: BytesN<32>, request_count: u64, token: Address, amount_billed: i128, amount_settled: i128, price_book_version: u32, protocol: Protocol, channel: Option<Address>) -> Result<u64, Error>`**
  — callable only by `operator`. Validates, in order: the period is non-empty and already
  finished (`period_start < period_end <= current ledger`); `request_count` is non-zero;
  amounts are non-negative and `amount_settled <= amount_billed`; `channel` is present iff
  `protocol == MppSession`; and — the most important checks in this contract — that the price
  book agrees on **one** version for the whole period. This last check is two live calls to
  `price-book`, not one: `version_at(period_start)` and `version_at(period_end)` must agree
  with each other before either is compared against `price_book_version`. If the schedule
  changed partway through the period, the two calls disagree and the period is rejected
  outright with `PeriodSpansPriceChange` — no single version honestly covers a statement
  whose window straddles a price change, no matter what the caller claims. Only once the
  period is confirmed to sit inside one version's window is that version compared against
  `price_book_version`; a mismatch there is `PriceVersionStale`. Returns the new sequence
  number for `operator` (the first anchor is `1`).
  Errors: `BadPeriod`, `EmptyStatement`, `BadAmounts`, `ChannelMismatch`,
  `PriceVersionUnknown`, `PeriodSpansPriceChange`, `PriceVersionStale`, `IndexFull`.

- **`verify_usage(operator, seq: u64, leaf: BytesN<32>, proof: Vec<BytesN<32>>) -> Result<bool, Error>`**
  — no auth; anyone can verify a charge against a statement, and that is the whole point.
  Folds `leaf` up through `proof` via sorted-pair merkle hashing and compares the result
  against the statement's `usage_root`. Returns `Ok(false)` — not an error — for a
  valid-shaped proof that does not reach the root; errors are reserved for malformed input.
  Errors: `ProofTooLong` if `proof` has more than `MAX_PROOF_NODES` (32) entries, `NotFound`
  if no such statement exists.

- **`open_dispute(operator, seq: u64, consumer: Address, reason_hash: BytesN<32>) -> Result<(), Error>`**
  — callable only by the statement's `consumer`. `consumer` must match the statement's own
  `consumer` field — a mismatch returns `NotFound`, the same error as a missing statement, so
  a party unrelated to a statement cannot learn whether it exists. Sets the statement's status
  to `Disputed` and records the dispute.
  Errors: `NotFound`, `NotAnchored` if the statement is not currently `Anchored`.

- **`resolve_dispute(operator, seq: u64, resolution_hash: BytesN<32>, amount_credited: i128) -> Result<(), Error>`**
  — requires **both** parties' auth, `operator` first, then the statement's `consumer`. There
  is deliberately no arbiter, no admin override, and no timeout that auto-resolves in the
  operator's favour — if the two sides never agree, the statement stays publicly `Disputed`
  forever, and that public mark is the entire enforcement mechanism.
  Errors: `NotFound`, `NotDisputed` if the statement is not currently `Disputed`,
  `CreditTooLarge` if `amount_credited` is negative or exceeds the statement's
  `amount_billed`.

- **`get_statement(operator, seq: u64) -> Result<Statement, Error>`** — no auth, read-only.
  Errors: `NotFound`.

- **`list_statements(operator, consumer) -> Result<Vec<u64>, Error>`** — no auth, read-only.
  Sequence numbers anchored for the pair, oldest first, newest last. An empty result means no
  statements exist for the pair; that is not an error.

- **`get_dispute(operator, seq: u64) -> Result<Dispute, Error>`** — no auth, read-only.
  Errors: `NotFound` if no dispute has been opened.

- **`extend_statement_ttl(operator, seq: u64, ledgers: u32) -> Result<(), Error>`** — **no
  auth, intentionally.** Anyone may pay rent to keep an audit record alive — an auditor
  checking a three-year-old statement is not a party to it and was never going to be able to
  authenticate as one; requiring auth here would just make old records unrecoverable once
  their original parties are unreachable. Extends the TTL of the statement, and its dispute if
  one exists, out to `ledgers` from now, clamped to a sane maximum. Do not "fix" this function
  by adding auth. Errors: `NotFound` if no such statement exists.

### Errors

| Discriminant | Variant | Meaning |
|---|---|---|
| 1 | *(unused)* | Was `AlreadyInitialized`, for the same reason and with the same fix as `price-book`'s — see above. |
| 2 | `NotFound` | No such statement/dispute, or (for `open_dispute`) a real statement but the wrong consumer. |
| 3 | `BadPeriod` | `period_start >= period_end`, or `period_end > current ledger`. |
| 4 | `BadAmounts` | A negative amount, or `amount_settled > amount_billed`. |
| 5 | `EmptyStatement` | `request_count == 0`. |
| 6 | `PriceVersionUnknown` | `price-book` has no such version for this operator (or the cross-contract call itself failed). |
| 7 | `PriceVersionStale` | `version_at(period_start)` and `version_at(period_end)` agree with each other, but not with the claimed `price_book_version`. |
| 8 | `ChannelMismatch` | `channel` set without `MppSession`, or absent with it. |
| 9 | `IndexFull` | `CONSUMER_IDX_CAP` (500) reached — see Known limitations. |
| 10 | `NotAnchored` | A dispute was opened on a statement that is not `Anchored`. |
| 11 | `NotDisputed` | `resolve_dispute` called on a statement that is not `Disputed`. |
| 12 | `CreditTooLarge` | `amount_credited` is negative or exceeds `amount_billed`. |
| 13 | `ProofTooLong` | A merkle proof longer than `MAX_PROOF_NODES` (32). |
| 14 | `PeriodSpansPriceChange` | `version_at(period_start) != version_at(period_end)`: the schedule changed partway through the period, so no single `price_book_version` honestly covers the whole thing. Distinct from `PriceVersionStale`, which is about the claimed version not matching what's in force — this is about the period itself straddling a change, independent of what was claimed. |

### Events

- `anchor` — topics `("statement", "anchor")`, data `(operator: Address, consumer: Address, seq: u64, usage_root: BytesN<32>, amount_billed: i128, amount_settled: i128, protocol: Protocol)`.
- `dispute` — topics `("statement", "dispute")`, data `(operator: Address, consumer: Address, seq: u64, reason_hash: BytesN<32>)`.
- `resolve` — topics `("statement", "resolve")`, data `(operator: Address, consumer: Address, seq: u64, resolution_hash: BytesN<32>, amount_credited: i128)`.

### Known limitations

`ConsumerIdx(operator, consumer)` — the vector `list_statements()` returns — is capped at
**500 entries** (`CONSUMER_IDX_CAP`). `anchor()` returns `IndexFull` once a given
(operator, consumer) pair's 500th statement would be appended. A merkle proof passed to
`verify_usage()` is capped at **32 nodes** (`MAX_PROOF_NODES`), which covers over 4 billion
leaves; longer proofs are rejected with `ProofTooLong` rather than accepted unbounded. Both
are deliberate, documented limits.

### `amount_settled`

`amount_settled` is the operator's unverifiable claim. The contract cannot check it and
must not pretend to. What makes it meaningful is an off-chain indexer independently summing
SAC transfer and channel events for the period and flagging mismatches.
