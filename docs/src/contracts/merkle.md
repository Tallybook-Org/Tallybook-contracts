# Merkle verification

`statement_registry.verify_usage()` checks one usage record against an anchored
`usage_root` by folding a leaf up through a proof. This page is precise enough to
implement a compatible tree builder in another language — that's its job, since the
off-chain collector that builds these trees does exactly that, in Go.

## Leaf format

The leaf itself is **not computed by the contract**. `verify_usage` receives a leaf
and only folds it; the format below is fixed by the off-chain collector, documented
here so the two sides can't drift apart:

```
record_bytes = XDR(ScVal::Map{           // keys sorted alphabetically
    Symbol("amount"):   I128(charged_amount),
    Symbol("consumer"): Address(consumer),
    Symbol("endpoint"): BytesN<32>(sha256(method || " " || path_template)),
    Symbol("ledger"):   U32(settlement_or_observation_ledger),
    Symbol("price_v"):  U32(price_book_version),
    Symbol("reqid"):    BytesN<32>(request_id),
    Symbol("units"):    U64(unit_count),
})
leaf = sha256(sha256(record_bytes))
```

`ScVal::Map` keys are sorted alphabetically — `amount`, `consumer`, `endpoint`,
`ledger`, `price_v`, `reqid`, `units` — which matches the convention
`stellar-experimental/one-way-channel` uses for its own commitments; that consistency
is deliberate, not incidental. `endpoint` is a hash rather than the raw method and
path so leaves stay a fixed size regardless of route length. The double `sha256`
prevents a leaf from being passed off as an internal node: an attacker who wanted to
claim some leaf was actually two children of the tree merged together would need a
first-preimage of a single `sha256`, which double-hashing the leaf specifically rules
out as a valid internal-node value (internal nodes are single-hashed pairs; a
double-hashed leaf can't coincide with one without a genuine collision).

## Sorted-pair folding

```rust
pub fn fold(env: &Env, leaf: BytesN<32>, proof: &Vec<BytesN<32>>) -> BytesN<32> {
    let mut node = leaf;
    for sibling in proof.iter() {
        let (lo, hi) = if node <= sibling { (node, sibling) } else { (sibling, node) };
        let mut concatenated = Bytes::from(lo);
        concatenated.append(&Bytes::from(hi));
        node = env.crypto().sha256(&concatenated).to_bytes();
    }
    node
}
```

At each step, the current node and the next proof entry are ordered by byte
comparison before concatenating — `sha256(min(a, b) || max(a, b))` — rather than
concatenated in a fixed left/right order. This is what makes a leaf index
unnecessary: a verifier doesn't need to know whether its leaf was originally the left
or right child at each level, because the fold puts the smaller value first
regardless. There's no leaf-index parameter anywhere in this contract's interface, so
an off-by-one in that index — a whole class of bug in index-based merkle proofs — is
structurally impossible here.

## Worked verification, from a committed fixture

[`fixtures/merkle/four-leaves.json`](https://github.com/Tallybook-Org/tallybook-contracts/blob/main/fixtures/merkle/four-leaves.json)
commits four 32-byte leaves and their root:

```
leaf[0] = 0101010101010101010101010101010101010101010101010101010101010101
leaf[1] = 0202020202020202020202020202020202020202020202020202020202020202
leaf[2] = 0303030303030303030303030303030303030303030303030303030303030303
leaf[3] = 0404040404040404040404040404040404040404040404040404040404040404
root    = 1d0cafe12ca55e5e8d0903a1847cffae908539b86fee1c52418b2cd453479e7c
```

The tree is balanced: two pairs folded once each, then those two results folded
together.

```
n01 = sha256(leaf[0] || leaf[1])   -- leaf[0] < leaf[1], no swap
    = f818afd37a6dc3bc92fb44731011277006db4efa6e9023cd7468c02335d22a4d
n23 = sha256(leaf[2] || leaf[3])   -- leaf[2] < leaf[3], no swap
    = 505a9c6ac70bdffa46248e2025483f9fe997a0e31ed25559e448b73b7e02b9bd
root = sha256(n01 || n23)          -- n01 < n23, no swap
     = 1d0cafe12ca55e5e8d0903a1847cffae908539b86fee1c52418b2cd453479e7c
```

The fixture's committed proof for `leaf[0]` is `[leaf[1], n23]` — which checks out:
folding `leaf[0]` with `leaf[1]` first reproduces `n01`, and folding that with `n23`
reproduces `root`, exactly as computed above. All four leaves' committed proofs were
independently re-derived and matched against the fixture before this page was
committed.

**Negative case:** flip one bit in `leaf[1]` before folding `leaf[0]`'s proof against
it, and the intermediate result no longer equals `n01`, so the final fold doesn't
equal `root`. `verify_usage` returns `Ok(false)` for this — a corrupted or wrong
sibling is a well-formed 32-byte value, not malformed input, so it's not an error.

## Fixtures

Three fixtures are committed under `fixtures/merkle/`: `single-leaf.json` (a
degenerate one-node tree, empty proof), `four-leaves.json` (above), and
`seven-leaves.json` — an unbalanced tree, deliberately included because unbalanced
trees are where sorted-pair implementations most often diverge from each other.
