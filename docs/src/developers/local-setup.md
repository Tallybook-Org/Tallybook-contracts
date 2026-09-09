# Local setup

## Clone

```
git clone https://github.com/Tallybook-Org/tallybook-contracts.git
cd tallybook-contracts
```

## Toolchain

Install Rust via [rustup](https://rustup.rs). This repo pins its toolchain in
`rust-toolchain.toml` — `rustup` reads that file automatically and installs the exact
channel it names (currently `1.98.1`) the first time you run a cargo command in this
directory, along with the `wasm32v1-none` target the file also declares. You don't
need to run `rustup target add` by hand for a normal checkout.

`wasm32v1-none` is the only wasm target the Soroban runtime accepts —
`wasm32-unknown-unknown` isn't supported here, and isn't even available past Rust
1.82 for contracts built this way. If you ever drive the toolchain by hand outside
this repo's own `rust-toolchain.toml`, make sure you're targeting `wasm32v1-none`.

Install the [Stellar CLI](https://developers.stellar.org/docs/tools/stellar-cli),
`28.0.0` or newer: `cargo install --locked stellar-cli`.

## The build-order requirement

**Build `price-book` before running `cargo test` or `cargo clippy` on a clean
checkout.** `contracts/statement-registry/src/price_book.rs` uses
`soroban_sdk::contractimport!` to pull in `price-book`'s compiled wasm at compile
time — that file has to exist on disk before `statement-registry` will even compile,
let alone test. `make build` builds `price-book` first, specifically to encode this
ordering:

```
make build
```

Skipping straight to `cargo test` on a fresh clone fails with a file-not-found error
pointing at a `.wasm` path that doesn't exist yet — that's this dependency, not a
broken checkout.

## Running tests

```
make test
```

runs `cargo test` across the workspace. `make fmt` / `make clippy` run formatting and
lints.

## `test_snapshots/`

Each contract's tests write a ledger snapshot JSON per test under its own
`test_snapshots/` directory. These are committed, not gitignored — they're a record
of what each test actually exercised on the simulated ledger, not disposable build
output. If a snapshot changes in a test unrelated to whatever you're working on,
that's a signal worth investigating, not something to regenerate away without
understanding why it moved.
