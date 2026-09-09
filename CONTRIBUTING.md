# Contributing

This file is a seed — the full contributing guide lands per the build sequence in
`CLAUDE.md` §8. For now it carries one note that matters as soon as tests exist.

## Test snapshots

`cargo test` writes a ledger snapshot JSON per test under each contract's
`test_snapshots/`. These are committed, not ignored — they're a record of what each
test actually exercised, not disposable build output.

If a snapshot changes in a test unrelated to the change you're making, that is a
signal of an unintended side effect and should be investigated, not regenerated
away. Don't `git checkout` or re-run-and-recommit a snapshot just to make a diff go
away without understanding why it moved.
