# Statement lifecycle

A `Statement` moves through exactly three states, in one direction, with no state
after the last one:

```text
                anchor()              open_dispute()             resolve_dispute()
              operator only            consumer only         operator AND consumer
                    │                       │                          │
                    ▼                       ▼                          ▼
              ┌───────────┐           ┌───────────┐             ┌───────────┐
   (none) ───▶│ Anchored  │──────────▶│ Disputed  │────────────▶│ Resolved  │
              └───────────┘           └───────────┘             └───────────┘
```

This repo evaluated `mdbook-mermaid` for this diagram and used plain text instead —
it renders identically everywhere the page does, with no preprocessor and no extra
build step.

## Transitions

- **`(none) → Anchored`**, via `anchor()`. Requires `operator.require_auth()`. This is
  also where the statement's data is fixed: `usage_root`, `amount_billed`,
  `price_book_version`, and everything else on `Statement` is set once here and never
  changed again, even after a dispute resolves.
- **`Anchored → Disputed`**, via `open_dispute()`. Requires the statement's own
  `consumer.require_auth()` — no one else can open a dispute against a statement, and a
  caller who is not that consumer gets `NotFound`, the same error as a statement that
  doesn't exist, so an unrelated party can't even learn whether one does. Fails with
  `NotAnchored` if the statement isn't currently `Anchored`.
- **`Disputed → Resolved`**, via `resolve_dispute()`. Requires **both** parties' auth,
  `operator` first, then the statement's `consumer`. Fails with `NotDisputed` if the
  statement isn't currently `Disputed`. `amount_credited` cannot be negative or exceed
  `amount_billed` (`CreditTooLarge` otherwise).

## What's impossible

- **`Anchored → Resolved` directly.** There is no way to resolve a statement that was
  never disputed; `resolve_dispute()` only accepts a `Disputed` statement.
- **`Disputed → Anchored`.** A dispute cannot be withdrawn or closed by only the
  consumer. It stays `Disputed` until both parties call `resolve_dispute()`, however
  long that takes.
- **`Resolved → anything`.** `Resolved` is terminal. There's no function that reopens
  a resolved statement.
- **Anyone resolving alone.** `resolve_dispute()` requires both signatures in the same
  call — there's no operator-only or consumer-only path to `Resolved`.

There is deliberately no arbiter, no admin override, and no timeout that
auto-resolves in the operator's favor. If the two sides never agree, the statement
stays publicly `Disputed` forever. That public mark, visible to anyone reading
`get_statement`, is the entire enforcement mechanism — Tallybook doesn't adjudicate
who's right, it makes sure a live disagreement can't be hidden.
