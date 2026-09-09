# Worked example

A full month for one operator, `acme-weather-api`, and one consumer paying by MPP
session mode, with every number carried through to the end. Addresses below
(`GEXAMPLE...`, `CEXAMPLE...`) are illustrative, not real testnet identities —
[Verifying a charge](../guides/verifying-a-charge.md) runs the same shapes of call
against the live deployment.

## The schedule

`acme-weather-api` publishes one price: $0.01 per request to `GET /v1/forecast`. The
canonical JSON, and its hash, are the same ones shown in [Pricing](pricing.md):

```json
{"currency":"USDC","endpoints":[{"method":"GET","path":"/v1/forecast","price":"0.01","unit":"request"}],"operator":"acme-weather-api"}
```

```
sha256 = 63762798fe891366f1f0f4aa3f697f7e7bb20bb5041f7b656178e7e19caef47b
```

`publish(operator, schedule_hash, uri, effective_ledger)` is called with that hash,
`effective_ledger` set to the current ledger (effective immediately), and returns
version `1`. This is the only version published all month — nothing in this example
crosses a price change.

USDC on Stellar, like other SAC tokens minted from a classic Stellar asset, uses 7
decimal places, so `i128` amounts throughout are in units of 0.0000001 USDC ("stroops"):
$0.01 is `100_000`.

## The channel

The consumer opens a `one-way-channel` instance and calls `top_up` for $1,500.00 —
`15_000_000_000` stroops — as a buffer well above what a month at this rate is
expected to cost.

## Usage accrues

Over the month, the consumer's agent makes requests against `/v1/forecast`. Each one
is metered off-chain and priced against schedule version 1. The off-chain custodian
tracks the cumulative signed commitment as it grows; nothing is on-chain yet.

## Mid-month sweep

By ledger `1_259_200` — 15 days into the period, which started at ledger
`1_000_000` — the agent has made 52,340 requests. The custodian has a valid signed
commitment for that cumulative total and calls `settle` on the channel, well before
any refund window is at risk:

```
52,340 requests × 100,000 stroops = 5,234,000,000 stroops = $523.40
```

`settle` withdraws $523.40 to the operator without closing the channel. $976.60 of
the $1,500.00 deposit remains in the contract.

## Period close

By ledger `1_518_400` — the end of the 30-day period (`30 × 17,280` ledgers after
`period_start`, the same `DAY_IN_LEDGERS` unit `price_book`'s TTL constants use) —
the agent has made 100,000 requests in total for the month:

```
100,000 requests × 100,000 stroops = 10,000,000,000 stroops = $1,000.00
```

The custodian calls `close` with the final cumulative commitment of $1,000.00. Since
$523.40 of that was already withdrawn via `settle`, `close` transfers the remaining
$476.60 to the operator and returns the rest of the deposit to the consumer:

```
$1,000.00 (final commitment) − $523.40 (already settled) = $476.60 (transferred at close)
$1,500.00 (deposit)         − $1,000.00 (final commitment) = $500.00 (returned to consumer)
```

The operator received $523.40 + $476.60 = $1,000.00 across the two calls — exactly
the amount billed for the period, and not one stroop more.

## The merkle root

The collector builds a merkle tree over the period's usage records and anchors its
root. For two of those records:

```
record_a ≈ request-id 0000000000000001, GET /v1/forecast, amount=100000, ledger=1000042, price_v=1, units=1
record_b ≈ request-id 0000000000000002, GET /v1/forecast, amount=100000, ledger=1000043, price_v=1, units=1

leaf_a = sha256(sha256(record_a)) = 4c3c3a5491ccd43935a8af732742f4f3a24783078a8b8c86c52c6429791b21a8
leaf_b = sha256(sha256(record_b)) = 25cc509cf93233110d24bcf8486a8133fe5c31b6a5494a5d1e0510fc5981050b

usage_root = sha256(leaf_b || leaf_a)   -- leaf_b < leaf_a byte-wise (0x25... < 0x4c...)
           = a1ddcc4b46c00189f0a2afd84067d7f40fa40ca8479929e08895e82d13f1dbea
```

(The full record layout — the actual XDR bytes hashed into a leaf — is specified
exactly in [Merkle verification](../contracts/merkle.md); the values above stand in
for two of the 100,000 real leaves this tree would actually have.)

## Anchoring the statement

The operator calls:

```
anchor(
  operator: acme-weather-api,
  consumer: GEXAMPLE...,
  period_start: 1_000_000,
  period_end: 1_518_400,
  usage_root: a1ddcc4b...1dbea,
  request_count: 100_000,
  token: <USDC SAC address>,
  amount_billed: 10_000_000_000,
  amount_settled: 10_000_000_000,
  price_book_version: 1,
  protocol: MppSession,
  channel: Some(CEXAMPLE...),
)
```

`amount_billed` and `amount_settled` are equal here because, in this example, the full
$1,000.00 was in fact settled by month end — `amount_settled` is still just the
operator's claim; the contract doesn't check it against the channel's real transfer
events. `version_at(1_000_000)` and `version_at(1_518_400)` both return `1`, so the
period doesn't span a price change, and `1` matches the claimed
`price_book_version`. `anchor()` succeeds and returns sequence number `1` for this
`(operator, consumer)` pair.

## One buyer verification

The consumer takes `record_a` off their own logs, recomputes `leaf_a`, and calls:

```
verify_usage(operator: acme-weather-api, seq: 1, leaf: leaf_a, proof: [leaf_b])
```

`fold()` hashes `sha256(leaf_b || leaf_a)` — the same order shown above, since
`leaf_b < leaf_a` byte-wise — and gets `a1ddcc4b...1dbea`, which matches the statement's
`usage_root`. `verify_usage` returns `true`. If the consumer instead submitted an
`amount` field one stroop different from what was actually billed, `leaf_a` would
hash to something else, the root wouldn't match, and `verify_usage` would return
`false` — not an error, since the proof itself is well-formed.

## The cost of anchoring

None of the calls in this walkthrough were run against a live network — the numbers
above are illustrative. Two numbers here aren't: real `publish()` and `anchor()`
calls against the live testnet deployment, each a real transaction with a measured
fee.

```
publish():
  tx hash:     be5cc70b41bc89b139011c0b69945596856709ca35ca6074191126ca70024021
  ledger:      4,587,953 (testnet)
  fee_charged: 1,963,213 stroops = 0.1963213 XLM
  source:      https://horizon-testnet.stellar.org/transactions/be5cc70b41bc89b139011c0b69945596856709ca35ca6074191126ca70024021

anchor():
  tx hash:     54da8ffd78e0061c7f1d2ace7139ca7628ec3e7f70807b8493ec87b57be72d48
  ledger:      4,592,019 (testnet)
  fee_charged: 2,726,864 stroops = 0.2726864 XLM
  source:      https://horizon-testnet.stellar.org/transactions/54da8ffd78e0061c7f1d2ace7139ca7628ec3e7f70807b8493ec87b57be72d48
```

(That `anchor()` call is the one walked through end to end in
[Verifying a charge](../guides/verifying-a-charge.md) — a much smaller statement
than this page's, five requests rather than 100,000, since it exists to demonstrate
the interface rather than to bill anyone. Its fee is real regardless of how small the
statement behind it is; a persistent write costs what it costs independent of the
number this particular call happened to be anchoring.)

At the XLM/USD spot price quoted by CoinGecko's public API at the time of writing
(`$0.184197`), those fees are:

```
publish(): 0.1963213 XLM × $0.184197/XLM ≈ $0.0362
anchor():  0.2726864 XLM × $0.184197/XLM ≈ $0.0502
```

`anchor()` costs more than `publish()` — unsurprising, since a `Statement` is a
larger record than a `PriceBookVersion` and `anchor()` writes to one more index
(`ConsumerIdx`) than `publish()` does. Using the real, larger `anchor()` figure:
$0.0502 against the $1,000.00 this operator billed for the month is about **0.005%
of revenue — one part in roughly 19,900**. That's for one `anchor()` call covering
the whole period, however many requests it represents. `anchor()` here represents
100,000 requests; at this same per-invocation cost, writing once per request instead
of once per period would run **100,000 × $0.0502 ≈ $5,020** — more than five times
the entire month's billed revenue, before either contract does anything with the
money. That comparison, not the exact cent figure, is why one `anchor()` call per
period is viable where a per-request on-chain write is not.
