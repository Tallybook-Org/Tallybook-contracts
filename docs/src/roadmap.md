# Roadmap

## What exists

Both contracts — `price_book` and `statement_registry` — are complete, tested, and
deployed to testnet. See [Contracts](contracts/overview.md) for the addresses and
[How it works](how-it-works.md) for exactly which of the five settlement steps they
cover (anchoring and verification) versus which are still planned.

## Planned: the off-chain layer

Nothing below this line exists in this repository, or anywhere else yet. Each is a
sentence on what it will do, not a commitment to a date.

- **Collector.** Meters requests against an operator's published schedule and builds
  the merkle tree — leaves, proof generation, the works — that `anchor()` commits to.
- **Settler.** Holds signed payment-channel commitments as they arrive in MPP session
  mode, and submits the latest one to the channel contract before the funder's refund
  window closes — the step that actually closes the gap described in
  [The problem](problem.md).
- **Indexer.** Independently sums real SAC transfer and channel events per period and
  flags any mismatch against a statement's `amount_settled` claim — the check this
  contract deliberately doesn't perform on-chain.
- **Dashboard.** Reads all of the above plus the two contracts directly, so an
  operator or a buyer can see a period's statement, its dispute status, and its
  reconciliation state without running any commands themselves.

## Planned: consumer index bucketing

**Status: settled design, not yet implemented.** `ConsumerIdx(operator, consumer)`
currently stores every sequence number ever anchored for a pair in one persistent
`Vec<u64>` — every `anchor()` call for that pair reads, appends to, and rewrites the
whole thing, and `IndexFull` (`CONSUMER_IDX_CAP` = 500) permanently blocks a pair
once its lifetime history fills it.

The settled replacement, recorded in full in `CLAUDE.md` §6:

- The storage key becomes `ConsumerIdx(operator, consumer, bucket)`, where
  `bucket = period_end / BUCKET_LEDGERS`.
- `BUCKET_LEDGERS = 518_400` — 30 × `DAY_IN_LEDGERS` (17,280), the same unit
  `BUMP_THRESHOLD` already uses elsewhere in this contract. Each bucket spans roughly
  30 days.
- `list_statements` changes signature to
  `list_statements(env, operator, consumer, start_bucket: u32, limit: u32) -> Result<Vec<u64>, Error>`,
  returning ascending sequence numbers from `start_bucket` forward, `limit` clamped
  to 100.
- `IndexFull` stays at discriminant 9 but is redefined: it means a single bucket
  exceeded `BUCKET_CAP = 500`, not that the pair's lifetime history is full — lifetime
  capacity per pair becomes unbounded, since a pair just accumulates more buckets
  over time instead of hitting a hard ceiling.

This is a public API change (the `list_statements` signature), not only a storage
change, which is why it's specified here and in `CLAUDE.md` before any code changes
land — see the tracked issue for current implementation status.

## Deliberately out of scope

- A facilitator. Tallybook doesn't settle payments — see
  [The problem](problem.md) for what OpenZeppelin already does here.
- A router. Tallybook doesn't execute or route requests — RouteDock already does.
- A payment channel contract. Tallybook consumes
  `stellar-experimental/one-way-channel`; it doesn't fork, wrap, or reimplement it.
- A protocol fee. Tallybook takes no cut of anything, and there's no fee logic
  anywhere in either contract.
