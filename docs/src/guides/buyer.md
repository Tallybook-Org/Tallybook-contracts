# Buying from an operator

This guide is for whoever's paying an operator — you, or an agent acting for you —
and wants to check what got billed. It assumes you know HTTP, not Stellar.

## Reading a statement

Every billing period an operator anchors becomes one `Statement`, fetchable with
`get_statement(operator, seq)`. The fields that matter to you as a buyer:

- `period_start` / `period_end` — the ledgers this statement covers.
- `usage_root` — the merkle root over every usage record for the period. This is
  what you check individual charges against.
- `request_count` — how many requests the operator says this period covers.
- `amount_billed` — what you're being asked to pay, in the token's smallest unit.
- `amount_settled` — the operator's own claim about what already moved on-chain for
  this period. This one is unverified by the contract itself; it's not proof of
  anything by itself, only a claim to cross-check against real transfers if you want
  to go that far.
- `price_book_version` — which price schedule version this period was billed under.
  You can fetch that version directly from `price_book.get_version` and check its
  `schedule_hash` against the schedule document yourself.

You don't need any Stellar identity of your own just to read a statement — every
getter on `statement_registry` is open to anyone, no auth required.

## Checking one charge

To check that one specific request is actually included in what you were billed,
you need three things: the leaf for that request (built the same way the operator's
collector builds it — see [Merkle verification](../contracts/merkle.md) for the exact
format), the merkle proof for it, and the statement's `seq`. Then call
`verify_usage(operator, seq, leaf, proof)`. It returns `true` if that request is
genuinely part of what the root commits to, `false` if it isn't — a `false` isn't an
error, it's the answer. [Verifying a charge](verifying-a-charge.md) walks through this
against a real statement, with real commands.

## Disputing a charge

If a charge looks wrong, call `open_dispute(operator, seq, consumer, reason_hash)`,
signed by you (the address the statement lists as `consumer`). `reason_hash` is the
`sha256` of whatever complaint document you're standing behind — the contract doesn't
interpret it, just records it. This marks the statement `Disputed`, publicly, for
anyone to see.

**Opening a dispute does not, by itself, get you anything back.** There's no arbiter
and no automatic outcome. Resolving it requires the operator's agreement:
`resolve_dispute` needs both your signature and the operator's, together, before
anything changes. If the operator never agrees, the statement just stays `Disputed`
indefinitely — which is itself the point. A statement anyone can see is permanently
and publicly marked disputed is real pressure, even without a court or an arbiter
attached to it, but it isn't a refund mechanism. If you need your money back and the
operator won't agree to credit you through `resolve_dispute`, that has to happen
outside this contract entirely.
