# Maintainer Cadence — `tallybook-contracts`

A standing brief for the coding agent. This is not a build sequence with an end; it is how
this repository is maintained week to week while it sits in the Stellar Wave Program.

Read this before any maintenance session. Everything in `CLAUDE.md` still applies — the
contract surface, coding standards, and git rules are unchanged.

---

## 1. The point of this document

The repository has to stay genuinely active. A repo with a month of silence reads as
abandoned, and Wave reviewers and prospective contributors both look at commit history.

But activity has to be real. Manufactured commits and padded issues are visible, and a
reviewer who notices padding discounts everything else in the repo. The bar for every
action taken under this brief:

> Would this action still be worth taking if nobody were watching the repo?

If no, do not take it.

---

## 2. Hard rules — never violate these

**Never close an issue a contributor could take.** Contributors earn Wave points by
solving issues. The open backlog is the incentive that attracts them. Closing your own
issues to generate activity destroys the thing the program rewards.

Specifically **never** touch:
- `feat(statement-registry): bucket consumer index by period` (high)
- `refactor(price-book): replace Timeline flat vector with a chunked structure` (high)
- Anything labelled `good first issue`
- Anything labelled `help wanted`
- Any issue in the merkle, property-testing, or documentation groups

**Never batch-create issues.** No "generate five issues" sessions. An issue exists because
a real problem was observed. If a session produces no findings, it produces no issues, and
that is a correct outcome to report.

**Never manufacture commits.** No reformatting for its own sake, no rewording comments, no
version bumps without cause, no whitespace churn. `git log` should read as work, not as
attendance.

**Never inflate complexity labels.** The current spread is 2 high, 11 medium, 20 low.
Complexity maps to points (100 / 150 / 200) and the maintainer assigns it. Keep the
distribution honest.

**Never change the contract surface** without explicit sign-off, as `CLAUDE.md` states.
That includes anything a contributor's in-flight work might depend on.

---

## 3. What the maintainer *is* allowed to close

A narrow list — work that is genuinely not contributor-suitable because it needs
credentials, repo admin rights, or judgement a newcomer cannot supply:

- CI workflow configuration and secrets
- Branch protection and repo settings
- Deploy script changes that require running against a funded account
- Anything requiring a mainnet or testnet key
- Reviewing and merging contributor PRs (this is the main job once contributors arrive)
- Security fixes with a disclosed advisory

If work outside that list needs doing urgently, do it — but say in the commit message why
it could not wait for a contributor.

---

## 4. Where real issues actually come from

These are the genuine sources. Check them on the cadence below; file only what they
surface.

### Upstream version movement
- `soroban-sdk` — currently pinned at `=27.0.6`. `28.0.0-rc.1` exists as a prerelease. When
  28.0.0 goes stable, that is a real issue: read the changelog, check whether the pinned
  MSRV floor moves, and file a scoped upgrade issue with the actual breaking changes named.
  Check with: `curl -s https://crates.io/api/v1/crates/soroban-sdk | python3 -c "import json,sys; d=json.load(sys.stdin); print(d['crate']['max_stable_version'])"`
- `stellar-cli` — currently 28.0.0. New releases may change flag names or fix the
  authorization-entry gap documented in `developers/invoking.md`. If a release fixes that
  gap, the docs page becomes wrong and that is an issue.
- Rust stable — the pin is 1.98.1. A new stable release is not automatically an issue; only
  file one if a dependency requires it.
- Protocol upgrades — a new Stellar protocol version may add host functions relevant to the
  contracts (as Protocol 25 added BN254 and Poseidon). Read the CAP, then decide.

### Documentation drift
Every one of these is a real bug when it happens:
- A function signature, error variant, or event in the contracts that no longer matches the
  README table or the docs site.
- A command in `guides/verifying-a-charge.md` or `developers/invoking.md` that no longer
  runs. **Re-run them.** They were verified live once; a CLI change can break them silently.
- An external link that has started 404ing. Fetch every link, do not eyeball them — this
  has already caught one broken Stellar docs URL.

### The deployment
- Testnet is periodically reset. If the deployed contracts stop resolving, the README, the
  docs site, the release notes, and the demo all reference dead addresses at once. Redeploy,
  update every reference in one commit, and note it in the release notes.
- Check the deployed contracts still respond with a read-only call. If they do not, that is
  the highest-priority maintenance task there is, because half the repo's evidence is links
  to them.

### CI
- Any red build. Fix it the same session.
- Action versions going stale or being deprecated.
- Dependabot or advisory alerts.

### The app repo
Once `tallybook` exists, building it will surface real requirements against the contracts —
a query shape the indexer needs, a leaf-format ambiguity the Go collector hits, an event
field that turns out to be missing. **These are the best issues this project will produce**,
because they come from a real consumer of the interface. File them here with an explicit
"Depends on" or "Surfaced by" cross-reference to the app repo issue.

---

## 5. The cadence

### Every session, before anything else
1. `git pull`
2. Check for open contributor PRs. Reviewing them takes priority over everything in this
   document.
3. Check CI is green on `main`.
4. Check for new issues opened by other people.

### Weekly (about 30 minutes)
1. Check the four upstream versions in §4. Report movement; file an issue only if something
   actually changed in a way that affects this repo.
2. Run one read-only call against each deployed contract. Confirm both still resolve.
3. `make fmt-check && make clippy && make test && make build` from a wiped `target/`.
4. `mdbook build docs` — confirm zero warnings.
5. Report: what was checked, what moved, what was filed. "Nothing to report" is a valid and
   frequent outcome.

### Fortnightly (about an hour)
1. Re-run every live command in `guides/verifying-a-charge.md` and
   `developers/invoking.md`. Anything that fails is an issue, and if the fix is
   documentation-only, fix it directly.
2. Fetch every external link in `docs/src` and the README. Fix 404s directly.
3. Banned-word grep across `docs/src` per the docs brief §1, in case an edit introduced one.
4. Verify the README's documented function lists, error tables, and event signatures still
   match the source exactly.

### Monthly
1. Review the issue backlog. Is anything now stale, obsolete, or superseded? Closing an
   obsolete issue is legitimate — closing a *valid* one is not. State the reason in the
   closing comment.
2. Re-check the complexity distribution. Has anything become easier or harder than labelled?
3. Check whether any issue has sat with no interest for a month. If so, it may be badly
   scoped rather than unpopular — rewrite the description, do not delete the issue.
4. Report the month's real activity: commits, PRs reviewed, issues filed, issues closed and
   why each one qualified under §3.

---

## 6. Issue quality bar

Any issue filed under this brief matches the existing 33 in structure. No exceptions:

- **Title** in commit style, lowercase: `type(scope): imperative description`
- **Summary** — what is wrong or missing, and where in the code. Name files and functions.
- **Why it matters** — one or two sentences. If this is hard to write, the issue is
  probably filler; discard it.
- **Acceptance Criteria** — checkboxes, specific enough that a contributor knows when they
  are done without asking.
- **Tech Stack** — one line.
- **Labels** — one complexity, one type, plus `good first issue` or `help wanted` where
  genuinely appropriate.

If a design decision is required that changes the contract surface, say so in the issue and
mark it as needing sign-off before implementation — do not decide it unilaterally.

---

## 7. What to do when there is nothing to do

Report that there is nothing to do. Do not invent work.

If the repository is genuinely stable and the backlog is healthy, the correct next action is
not in this repository at all — it is building the off-chain layer in `tallybook`. That work
produces real commits, real issues, and eventually turns three "Planned" rows in
`how-it-works.md` into "Built", which is worth more than any amount of maintenance activity
here.

---

## 8. Reporting format

End every maintenance session with:

```
Checked:      <what was inspected>
Moved:        <upstream changes found, or "none">
Filed:        <issues created, with why each is real, or "none">
Closed:       <issues closed, with which §3 category each falls under, or "none">
Fixed:        <commits made, or "none">
Needs you:    <anything requiring the maintainer's judgement or credentials>
```

A session that reports "none" across the board is a successful session. Silence in the log
is better than noise in the repo.
