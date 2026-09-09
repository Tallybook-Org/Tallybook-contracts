# Pricing

`price_book` never stores an actual price list. It stores a commitment to one: a
`sha256` hash and a URI, published by the operator, with a ledger from which it takes
effect. The full schedule — every endpoint, its unit, its per-unit price — lives
off-chain as canonical JSON. A 32-byte hash does the same evidentiary job as the
document for a fraction of the ledger cost, and the document's own content can be
independently hashed by anyone who fetches it to check that it matches.

An operator's schedule for the worked example on the next page might look like this:

```json
{"currency":"USDC","endpoints":[{"method":"GET","path":"/v1/forecast","price":"0.01","unit":"request"}],"operator":"acme-weather-api"}
```

That exact byte string hashes to
`63762798fe891366f1f0f4aa3f697f7e7bb20bb5041f7b656178e7e19caef47b` — the value
`publish()` would take as `schedule_hash`. The contract doesn't parse this JSON or
know its shape; it just stores the hash the operator gives it and the URI where the
document can be fetched.

## Versions move only forward

`publish()` rejects a new version's `effective_ledger` if it's before the current
ledger (`EffectiveInPast`), and rejects it if it doesn't strictly exceed the previous
version's `effective_ledger` (`EffectiveNotAfter`). A schedule can take effect
immediately — `effective_ledger` equal to the current ledger is allowed — but it can
never apply retroactively, and two versions can never share or reverse their order.
This is what makes "which price applied when" answerable at all: the timeline only
ever grows in one direction.

## `version_at`: which price applied at ledger N

`price_book` keeps a sorted `Vec<TimelineEntry>` per operator — just
`effective_ledger` and `version` pairs, ascending — separate from the full
`PriceBookVersion` records. `version_at(operator, ledger)` binary-searches this vector
for the entry with the highest `effective_ledger` that doesn't exceed `ledger`, and
returns its version. This is the function both `statement_registry.anchor()` and a
buyer checking their own bill call — it never scans every published version, so it
stays cheap as an operator's history grows toward the 256-entry cap (see
[Errors](../contracts/errors.md)).

## A billing period can't span a price change

`anchor()` calls `version_at()` twice — once for `period_start`, once for
`period_end` — and requires them to agree before comparing either to the statement's
claimed `price_book_version`. If they disagree, the statement is rejected with
`PeriodSpansPriceChange`, regardless of which version was claimed.

The reason is that no single version honestly describes such a period. If prices
changed on day 15 of a 30-day billing window, a statement covering the whole 30 days
under either version misrepresents the days on the other side of the change — the
version in force on day 1 wasn't in force on day 30, and vice versa. There's no
correct single number to put in `price_book_version` for that statement.

What an operator does instead is close the period at the price change: anchor one
statement running up to the ledger just before the new version's `effective_ledger`,
and start a new period from there. Two statements, each fully inside one version's
window, replace the one that would have straddled it.
