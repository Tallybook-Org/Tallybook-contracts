# Security Policy

## Audit status

**Neither contract in this repository has been audited.** `price-book` and
`statement-registry` are unaudited code. Treat them accordingly.

**The wider Tallybook system depends on
[`stellar-experimental/one-way-channel`](https://github.com/stellar/stellar-experimental)
for its MPP payment channel functionality, and that contract is also unaudited.**
This repo does not fork, vendor, or reimplement any part of it, but anything built on
top of Tallybook's MPP channel mode inherits that contract's unaudited status along
with this repo's own.

Do not deploy either contract in this repository, or any system built on top of them,
to hold or move real funds without an independent security audit first.

## Reporting a vulnerability

Report suspected vulnerabilities privately, not in a public GitHub issue: open a
[private security advisory](https://github.com/Tallybook-Org/tallybook-contracts/security/advisories/new)
on this repository. Include what you found, the affected function or contract, and
reproduction steps if you have them. Do not disclose the issue publicly until it has
been addressed.
