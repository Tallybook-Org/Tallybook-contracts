# Settlement shapes

Tallybook has to account for three different settlement mechanisms, and each one
produces a different shape of on-chain evidence. This page describes the mechanism
itself; [Statement lifecycle](statement-lifecycle.md) covers what
`statement_registry` does with the result.

## x402

The client signs a Soroban authorization entry — not a full transaction — and hands it
to a facilitator, which verifies the signature and submits the actual settlement
transaction. Any SEP-41 token works; USDC is the default. The exchange runs over three
headers:

- `PAYMENT-REQUIRED` — the server's response to an unpaid request, describing what it
  wants.
- `PAYMENT-SIGNATURE` — the client's signed authorization entry, sent with the retried
  request.
- `PAYMENT-RESPONSE` — confirmation that settlement happened.

The on-chain evidence is one settlement transaction per request, submitted by the
facilitator rather than the client. Nothing in that transaction records which HTTP
request it paid for or which price applied — that link exists only in the facilitator's
and the server's own logs, which is exactly what a `statement_registry.anchor()` call
supplies afterward.

## MPP charge mode

Each request settles individually as a direct Soroban SAC transfer. There is no
facilitator in the path; the client's own transaction moves the token.

The on-chain evidence here is stronger than x402's — a real transfer, from the payer,
for a specific amount — but it still doesn't say what the request was or whether
`amount_billed` used the right price. The transfer and the statement are two separate
records that have to be reconciled off-chain.

## MPP session mode

The funder deposits once into a `stellar-experimental/one-way-channel` contract instance
(`top_up`), then signs cumulative commitments off-chain as usage accrues — each
commitment supersedes the last with a strictly higher total. The server verifies a
commitment by simulating the channel's `prepare_commitment` before accepting it, then
keeps the highest cumulative amount it has a valid signature for. Submitting a
commitment on-chain (`settle`) withdraws against it without closing the channel;
`close` (or `close_start` then `close` after the funder's waiting period, if the funder
initiates it) transfers the final committed amount and returns whatever remains of the
deposit to the funder.

Session mode produces the thinnest on-chain trail of the three during the billing
period itself: nothing is on-chain until someone calls `settle` or `close`. Revenue
accrues as signatures in a database until then, which is the mechanism behind the
refund risk in [The problem](problem.md).

## What a unified ledger has to reconcile

`statement_registry` doesn't distinguish these three at the settlement level — it
records `protocol` on the `Statement` (`X402`, `MppCharge`, or `MppSession`) and, for
session mode, the `channel` contract address, but `anchor()` never checks a settlement
transaction against the statement. What it checks is the price: that `period_start` and
`period_end` fall inside a single `price_book` version's window, and that
`price_book_version` matches it. Whether the money shown in `amount_settled` actually
moved is left to an off-chain indexer summing the real transfer and channel events for
the period — `amount_settled` is documented on the `Statement` type as the operator's
own unverified claim, not something this contract checks.
