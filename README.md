# Tallybook-contracts
Tallybook is settlement bookkeeping for services that charge per HTTP request - APIs paid by AI agents and other machines over Stellar, using the x402 protocol, MPP charge mode, or MPP payment channels.

## Build

This repo contains two contracts: `price-book` has no dependencies; `statement-registry`
imports `price-book`'s compiled wasm at compile time (via `contractimport!`), so
`price-book` must be built first.

```sh
make build   # builds price-book, then statement-registry, via `stellar contract build`
make test    # cargo test --workspace
make fmt     # cargo fmt --all
make clippy  # cargo clippy --workspace --all-targets -- -D warnings
```

**On a clean checkout, run `make build` (or at least
`stellar contract build --package price-book`) before `cargo test` or `cargo clippy`.**
`statement-registry/src/price_book.rs` contractimports
`target/wasm32v1-none/release/price_book.wasm` unconditionally, so `statement-registry`
cannot even compile — for a test run, a clippy pass, or an IDE's background check — until
that file exists on disk. This is a real compile-time dependency, not just a build-order
nicety; `make build` and CI both encode it, but a bare `cargo test` on a fresh clone will
fail with a file-not-found error from the `contractimport!` macro until you build price-book
at least once.

Contracts are built exclusively with `stellar contract build`, targeting `wasm32v1-none` —
never with `cargo build`. The full contract interfaces, error variants, events, and known
limitations are documented later in this README once the contracts land.
