#!/usr/bin/env bash
# Deploys price_book, then statement_registry (pointed at price_book's
# address), to Stellar testnet. Builds both contracts first.
#
# Required environment:
#   STELLAR_ACCOUNT  Signing/source account: an identity name from the
#                     `stellar keys` store (recommended — see `stellar keys
#                     add`/`stellar keys generate`), or a secret key / seed
#                     phrase you export into the environment yourself.
#                     NEVER pass a secret key as a script argument or
#                     hardcode one here — this script only ever reads it
#                     from the environment or the keys store.
#   ADMIN_ADDRESS     Public key (G...) to pass as `admin` to both
#                     constructors. Holds no power over operator data in
#                     either contract; it only exists as a documented rent
#                     payer. Can be the same address as STELLAR_ACCOUNT's,
#                     but is deliberately a separate variable since the
#                     signer and the admin need not be the same account.
#
# Usage:
#   STELLAR_ACCOUNT=alice ADMIN_ADDRESS=GABC... ./scripts/deploy-testnet.sh
set -euo pipefail

: "${STELLAR_ACCOUNT:?Set STELLAR_ACCOUNT to a stellar keys identity (see \`stellar keys add\`), a secret key, or a seed phrase, via the environment. Never pass it as a command-line argument.}"
: "${ADMIN_ADDRESS:?Set ADMIN_ADDRESS to the public key (G...) to use as admin for both contracts.}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

NETWORK="testnet"
PRICE_BOOK_WASM="target/wasm32v1-none/release/price_book.wasm"
STATEMENT_REGISTRY_WASM="target/wasm32v1-none/release/statement_registry.wasm"

sha256_of() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    sha256sum "$1" | awk '{print $1}'
  fi
}

echo "==> Building price-book" >&2
stellar contract build --package price-book

echo "==> Building statement-registry" >&2
stellar contract build --package statement-registry

echo "==> Deploying price_book to $NETWORK" >&2
PRICE_BOOK_ID=$(stellar contract deploy \
  --wasm "$PRICE_BOOK_WASM" \
  --source-account "$STELLAR_ACCOUNT" \
  --network "$NETWORK" \
  -- --admin "$ADMIN_ADDRESS")

echo "==> Deploying statement_registry to $NETWORK (price_book = $PRICE_BOOK_ID)" >&2
STATEMENT_REGISTRY_ID=$(stellar contract deploy \
  --wasm "$STATEMENT_REGISTRY_WASM" \
  --source-account "$STELLAR_ACCOUNT" \
  --network "$NETWORK" \
  -- --admin "$ADMIN_ADDRESS" --price_book "$PRICE_BOOK_ID")

PRICE_BOOK_WASM_HASH=$(sha256_of "$PRICE_BOOK_WASM")
STATEMENT_REGISTRY_WASM_HASH=$(sha256_of "$STATEMENT_REGISTRY_WASM")

echo
echo "network:                      $NETWORK"
echo "price_book address:           $PRICE_BOOK_ID"
echo "price_book wasm hash:         $PRICE_BOOK_WASM_HASH"
echo "statement_registry address:   $STATEMENT_REGISTRY_ID"
echo "statement_registry wasm hash: $STATEMENT_REGISTRY_WASM_HASH"
