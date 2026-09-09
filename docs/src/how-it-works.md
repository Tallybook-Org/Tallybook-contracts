# How it works

The full Tallybook system has five steps. Two of them run inside this repository today;
the other three are off-chain components planned but not yet built. Don't read this
page as describing a system that runs end to end right now — the sweep step in
particular, the one that protects revenue from a channel refund, is not running.

| Step | What happens | Status | Where |
|---|---|---|---|
| 1. Meter | An off-chain collector counts requests and prices them against the operator's published schedule. | Planned | Off-chain collector, not in this repository |
| 2. Custody commitments | For MPP session mode, an off-chain custodian holds the signed channel commitments as they arrive. | Planned | Off-chain custodian, not in this repository |
| 3. Sweep before the refund window | The custodian submits the latest commitment to the channel contract's `settle` before the funder's refund window closes. | Planned | Off-chain sweeper calling `stellar-experimental/one-way-channel`, not in this repository |
| 4. Anchor statements | When a billing period closes, the collector merkle-roots the period's usage records and calls `anchor` on `statement_registry`, tied to the exact `price_book` version in force. | Built | `statement_registry.anchor()` |
| 5. Verify | A buyer checks one line item against the anchored root with `verify_usage`. A wrong charge goes to `open_dispute`. | Built | `statement_registry.verify_usage()`, `open_dispute()`, `resolve_dispute()` |

Steps 4 and 5 are the two contracts in this repository, and both are deployed to
testnet with a full test suite behind them — see [Contracts](contracts/overview.md).
Steps 1 through 3 are off-chain, unwritten, and out of scope for this repository; they
are tracked in the [roadmap](roadmap.md). Until step 3 exists and runs reliably, the
refund risk described in [The problem](problem.md) is not mitigated by anything this
project ships — an operator using MPP session mode today needs their own process for
sweeping commitments before a refund window closes.
