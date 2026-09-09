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

Contracts are built exclusively with `stellar contract build`, targeting `wasm32v1-none` —
never with `cargo build`. The full contract interfaces, error variants, events, and known
limitations are documented later in this README once the contracts land.
