# Contributing

## Documentation site

The docs site lives in `docs/` and builds with [mdBook](https://github.com/rust-lang/mdBook).
Install it with `cargo install mdbook` (or `brew install mdbook`), then run
`mdbook serve docs` for a live-reloading local preview, or `mdbook build docs` to build
the static site to `docs/book/`, which is gitignored — it is build output, not
something you commit.

## Build and test

See the README's Build section. In short: `make build` before `cargo test` or
`cargo clippy` on a clean checkout — `statement-registry` contractimports
`price-book`'s compiled wasm at compile time, so it cannot compile until that wasm
exists on disk.

## Coding standards

- `#![no_std]` in every contract crate.
- No `unwrap()`, `expect()`, `panic!()`, `unimplemented!()`, or `todo!()` outside
  `#[cfg(test)]` code. Where an invariant is genuinely impossible, still return an
  `Error` rather than panicking, and comment why it's unreachable.
- No floating point anywhere. Money is `i128` in the token's smallest unit; never a
  float or a decimal string.
- `cargo fmt` and `cargo clippy -- -D warnings` must pass before every commit — not
  just before pushing.
- Doc comments (`///`) on every public function: what it does, who may call it, which
  errors it returns.
- Storage access only through each contract's `storage.rs` helpers — no inline
  `env.storage()` calls in `lib.rs`.
- Comments explain *why*, not *what*.

## Testing conventions

- Each contract's `src/test.rs` has one `mod` per public function, with a `setup()`
  helper (and any other shared fixtures) at the top of the file, reused across `mod`s
  instead of repeating registration boilerplate in every test.
- **Every `Error` discriminant must be provoked by at least one test.** This is the
  coverage bar, not a suggestion.
- Every state-changing function needs a happy-path test and a negative auth test.
  Use `env.mock_all_auths()` for happy paths; for a negative auth test, use explicit
  `env.mock_auths(&[MockAuth { .. }])` naming the wrong address — never rely on the
  absence of `mock_all_auths()` alone, since an earlier call to it in the same `Env`
  puts the host in a persistent "auto-approve" recording mode that a later
  `mock_auths(&[...])` call correctly switches back to strict enforcement from
  (verified empirically; this is not documented behavior to take on faith).
- `statement-registry`'s tests register the **real** `price-book` wasm via
  `env.register(price_book::WASM, ..)` and drive it through the real generated
  client. Never mock `price-book` — the whole point of the cross-contract price
  checks is that they run against real `price-book` behavior.
- Assert emitted events with `SomeEvent { .. }.to_xdr(&env, &contract_id)` compared
  against `env.events().all()`, not by hand-building topic/data tuples.
- When a scratch/throwaway test is useful to sanity-check something before writing
  the real test (e.g. to determine an SDK type's exact shape, or to verify a runtime
  assumption), that's fine — but revert it before committing; don't leave debug
  scaffolding in a commit.

## Test snapshots

`cargo test` writes a ledger snapshot JSON per test under each contract's
`test_snapshots/`. These are committed, not ignored — they're a record of what each
test actually exercised, not disposable build output.

If a snapshot changes in a test unrelated to the change you're making, that is a
signal of an unintended side effect and should be investigated, not regenerated
away. Don't `git checkout` or re-run-and-recommit a snapshot just to make a diff go
away without understanding why it moved.

## Git workflow

- Stage named files only (`git add path/to/file`) — never `git add .` or `git add -A`.
- One commit per logical unit: one function, one type file, one test module.
- Push immediately after every commit — don't batch local history.
- Conventional commits: `type(scope): description`, lowercase, imperative, no
  trailing period. Types: `feat`, `fix`, `test`, `docs`, `chore`, `refactor`, `ci`.
  Scopes: `price-book`, `statement-registry`, `merkle`, `workspace`, `ci`, `fixtures`.
- Never force-push, never rewrite a pushed commit.
- Never commit a secret, a keypair, a `.env`, or a funded account's seed.

## Contract surface

Function names, parameters, return types, storage keys, error variants, and event
topics are fixed by the specification another repository is built against in
parallel. Don't change them without coordinating that change upstream first.
