# Verifying a charge

A full walkthrough against the live testnet deployment, checking one specific
request against a real anchored statement. Every command below was run for real
against `CB2IEP4SQ2GC5747HFHNMXEYWEULC5Z5TTTLET2QA4CAA5SYCWAXFKAW` (`price_book`) and
`CB75TTWGP3TLKEDGA2WOLEVCNKLUX6X5KS47GMWGBHUAVES7J55LY25M` (`statement_registry`)
before this page was committed, and the output shown is the real output — not
retyped from source.

Statement `seq: 1` for operator
`GDOLCHAOYP63BEHGAUJJS5IVQNUXLPBWCHO2HZRBTZRU52XBMW2TRJLM` was anchored specifically
to write this page: one usage record, so the merkle tree is the simplest possible
case — a single leaf, where the leaf and the root are the same value and the proof is
empty. [Merkle verification](../contracts/merkle.md) covers the multi-leaf case.

## 1. Get the statement

```
stellar contract invoke \
  --id CB75TTWGP3TLKEDGA2WOLEVCNKLUX6X5KS47GMWGBHUAVES7J55LY25M \
  --network testnet \
  --source tb-deployer \
  -- get_statement \
  --operator GDOLCHAOYP63BEHGAUJJS5IVQNUXLPBWCHO2HZRBTZRU52XBMW2TRJLM \
  --seq 1
```

```json
{"amount_billed":"500000","amount_settled":"500000","anchored_ledger":4592019,"channel":null,"consumer":"GAJ46LDZSSYAB4YY6VMPM763P652ZROT7TBG3YYE3BZBAUZDXYDDK6OY","operator":"GDOLCHAOYP63BEHGAUJJS5IVQNUXLPBWCHO2HZRBTZRU52XBMW2TRJLM","period_end":4592000,"period_start":4590000,"price_book_version":1,"protocol":"X402","request_count":5,"status":"Anchored","token":"CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC","usage_root":"bb48f8056bf24646e0e9eb0758ca0c3769ece76845023c4d320438182d9ee3fa"}
```

Any caller can run this — `get_statement` takes no auth. Note
`price_book_version: 1` and `usage_root`; both matter for the next two steps.

## 2. Get the price book version that applied

Confirm what `price_book` itself says was in force at the end of this period —
that's what `anchor()` already checked before this statement was allowed to exist,
but you don't have to take the statement's word for `price_book_version` either:

```
stellar contract invoke \
  --id CB2IEP4SQ2GC5747HFHNMXEYWEULC5Z5TTTLET2QA4CAA5SYCWAXFKAW \
  --network testnet \
  --source tb-deployer \
  -- version_at \
  --operator GDOLCHAOYP63BEHGAUJJS5IVQNUXLPBWCHO2HZRBTZRU52XBMW2TRJLM \
  --ledger 4592000
```

```
1
```

Matches `price_book_version: 1` from the statement.

## 3. Build the leaf

The leaf format is fixed and documented in full in
[Merkle verification](../contracts/merkle.md) — an XDR-encoded record, double-hashed.
For this statement, the leaf (built the same way a real off-chain collector would)
is:

```
bb48f8056bf24646e0e9eb0758ca0c3769ece76845023c4d320438182d9ee3fa
```

Since this statement covers a single usage record, this leaf **is** the
`usage_root` — there's nothing to fold it up through, so the proof is empty.

## 4. Call `verify_usage`

```
stellar contract invoke \
  --id CB75TTWGP3TLKEDGA2WOLEVCNKLUX6X5KS47GMWGBHUAVES7J55LY25M \
  --network testnet \
  --source tb-deployer \
  -- verify_usage \
  --operator GDOLCHAOYP63BEHGAUJJS5IVQNUXLPBWCHO2HZRBTZRU52XBMW2TRJLM \
  --seq 1 \
  --leaf bb48f8056bf24646e0e9eb0758ca0c3769ece76845023c4d320438182d9ee3fa \
  --proof '[]'
```

```
true
```

## 5. Read the result

`true` means this leaf really is what `usage_root` commits to. `verify_usage` also
takes no auth — anyone can run exactly this command against exactly this statement
and get the same answer, without asking the operator to confirm anything.

For comparison, the same call with a leaf that doesn't match returns `false`, not an
error — this was run for real too:

```
stellar contract invoke \
  --id CB75TTWGP3TLKEDGA2WOLEVCNKLUX6X5KS47GMWGBHUAVES7J55LY25M \
  --network testnet \
  --source tb-deployer \
  -- verify_usage \
  --operator GDOLCHAOYP63BEHGAUJJS5IVQNUXLPBWCHO2HZRBTZRU52XBMW2TRJLM \
  --seq 1 \
  --leaf 0000000000000000000000000000000000000000000000000000000000000000 \
  --proof '[]'
```

```
false
```
