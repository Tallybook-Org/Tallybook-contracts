# Contributing

Full build instructions, coding standards, and testing conventions live in
[CONTRIBUTING.md](https://github.com/Tallybook-Org/tallybook-contracts/blob/main/CONTRIBUTING.md)
at the repository root — this page doesn't duplicate it, only points at it and at the
rules specific to contributing against a protected branch.

## Where to start

Issues labeled
[`good first issue`](https://github.com/Tallybook-Org/tallybook-contracts/labels/good%20first%20issue)
are scoped to be a reasonable entry point. Issues labeled
[`help wanted`](https://github.com/Tallybook-Org/tallybook-contracts/labels/help%20wanted)
are ones a maintainer would specifically welcome outside help on, independent of how
large the change is.

## Commit convention

`type(scope): description` — lowercase, imperative, no trailing period. Types:
`feat`, `fix`, `test`, `docs`, `chore`, `refactor`, `ci`. Scopes:
`price-book`, `statement-registry`, `merkle`, `workspace`, `ci`, `fixtures`, `book`.
One commit per logical unit — one function, one type file, one page — not one commit
for a whole feature.

## Branch protection on `main`

`main` requires a pull request before merging — direct pushes aren't accepted from
outside the repository's own automation. Status checks must pass and be up to date
with the base branch before merging; the required check is `ci` (the workflow that
runs `make fmt-check`, `make build`, `make clippy`, and `make test`). Required
approving review count is `0` — review is welcome but not a merge gate at this
project's current size. Force-pushes and branch deletion are both disabled on `main`.
