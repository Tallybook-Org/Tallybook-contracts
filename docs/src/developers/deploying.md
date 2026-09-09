# Deploying

Two scripts, `scripts/deploy-testnet.sh` and `scripts/deploy-mainnet.sh`. Both do the
same thing in the same order: build both contracts, deploy `price_book`, then deploy
`statement_registry` with `price_book`'s freshly deployed address passed into its
constructor. Neither script ever accepts a secret key as a command-line argument —
both read signing material only from the environment or the `stellar keys` store.

## Environment variables

| Variable | Required by | Meaning |
|---|---|---|
| `STELLAR_ACCOUNT` | both | Signing/source account: a `stellar keys` identity name, a secret key, or a seed phrase, read from the environment. Never pass this as a script argument. |
| `ADMIN_ADDRESS` | both | The `G...` public key passed as `admin` to both constructors. Holds no power over operator data in either contract — see [Contracts](../contracts/overview.md) — it exists only as a documented rent payer. Can equal `STELLAR_ACCOUNT`'s address, but is a separate variable since the signer and the admin need not be the same account. |
| `STELLAR_RPC_URL` | mainnet only | Mainnet has no default public RPC endpoint; set this to your own provider's URL. |
| `CONFIRM=yes` | mainnet only, optional | Skips the interactive "type yes to continue" prompt, for scripted or CI use. Omit it to be asked to confirm by hand before anything is signed and submitted. |

## Constructor argument ordering

`price_book` deploys first, with no dependency on anything else. Its constructor
takes only `admin`. `statement_registry` deploys second, and its constructor takes
`admin` **and** `price_book` — the address `price_book`'s own deploy just returned.
That address is fixed permanently at construction; there's no setter, so getting the
order right at deploy time matters — there's no fixing it afterward short of
deploying a new `statement_registry` instance.

## Testnet

```
STELLAR_ACCOUNT=my-operator ADMIN_ADDRESS=GABC... ./scripts/deploy-testnet.sh
```

Testnet has a default public RPC baked into the CLI's network config, so no
`STELLAR_RPC_URL` is needed. Prints both addresses and both wasm hashes at the end.

## Mainnet

```
STELLAR_ACCOUNT=my-operator ADMIN_ADDRESS=GABC... \
  STELLAR_RPC_URL=https://your-rpc-provider.example \
  ./scripts/deploy-mainnet.sh
```

**Neither contract in this repository has been audited.** The mainnet script prints
that warning and asks you to type `yes` before signing or sending anything, unless
`CONFIRM=yes` is set. Read
[SECURITY.md](https://github.com/Tallybook-Org/tallybook-contracts/blob/main/SECURITY.md)
before running this against a network that moves real funds.

## Security rules these scripts follow

- **Never a secret key on the command line.** Both scripts read `STELLAR_ACCOUNT`
  from the environment only — `stellar contract deploy --source-account
  "$STELLAR_ACCOUNT"` — never as a literal argument you'd type into a shell history
  or a CI log.
- **Never a key in a commit.** `.gitignore` covers `.env*`, `.stellar/`, and every
  build artifact; nothing this repo generates or reads includes a secret.
