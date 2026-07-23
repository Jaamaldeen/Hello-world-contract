# Security Audit Checklist — `prediction-oracle-verifier`

Status: **Draft — pre-mainnet review**
Contract: `PredictionVerifier` (Soroban)
Reviewer: _______________  Date: _______________

Each item must be checked off with a note on how it was verified (code
reference, test name, or tool output). Unchecked items block mainnet
deployment.

---

## 1. Reentrancy

Soroban's host-enforced call model removes classic EVM-style reentrancy, but
cross-contract calls (e.g. to a reputation/prediction contract) can still
create state-consistency bugs if storage writes happen *after* an external
call.

- [ ] All external/cross-contract calls (`ReputationContractClient`, any
      future `PredictionContractClient`) happen **after** this contract's own
      state has been finalized for the operation, not before.
- [ ] No function reads state, calls out to another contract, then writes
      state based on stale pre-call assumptions (check-effects-interaction
      ordering).
- [ ] `resolve_prediction` writes the `Resolution` record and marks the
      prediction resolved atomically within a single invocation — no
      intermediate state where a second call could observe "in progress".
- [ ] Verify with a test that a resolution can only be written once, and
      that a re-entrant/duplicate call to `resolve_prediction` on an
      already-resolved prediction reverts (`AlreadyResolved`) rather than
      overwriting.

## 2. Overflow / Underflow

- [ ] Confirm `overflow-checks = true` is set in `[profile.release]` in
      `Cargo.toml` (confirmed present) — this makes overflow a panic/abort in
      release, not silent wraparound. Keep it enabled; do not weaken this for
      a "production" profile.
- [ ] Audit every arithmetic operation on `u128`/`i128` price and id fields
      (`actual_price`, `target_price`, `prediction_id`, `next_idea_id`-style
      counters) for overflow potential, especially anything derived from
      oracle-submitted input (untrusted).
- [ ] Confirm oracle-submitted `actual_price` is bounds-checked (e.g. against
      a sane max) before being compared/stored — an oracle (even an
      authorized one that's compromised) should not be able to submit a
      value that overflows downstream consumers.
- [ ] Fuzz or property-test the `Correct`/`Incorrect` comparison
      (`actual_price >= target_price`) at `u128::MAX` and `0` boundaries.

## 3. Access Control

- [ ] `require_admin` is called on **every** admin-only entrypoint
      (`set_authorized_oracle`, `remove_authorized_oracle`, and any future
      admin function such as changing the admin itself).
- [ ] `require_admin` both checks `admin == caller` **and** calls
      `caller.require_auth()` — confirmed present in `storage.rs`. Do not
      allow either check to be skipped independently.
- [ ] Admin is set exactly once at initialization; there is no
      unauthenticated `set_admin` entrypoint reachable after init. If admin
      rotation is a requirement, it must itself be gated by the *current*
      admin's `require_auth()`.
- [ ] `resolve_prediction` checks `is_oracle_authorized(&env, &oracle)` (or
      equivalent) **before** trusting the submitted price, and that the
      calling address is the same address that underwent `require_auth()` —
      i.e. no oracle-address parameter can be spoofed by a caller who is not
      that oracle.
- [ ] Confirm there's no function that lets a *non-admin* directly write to
      `VerifierKey::AuthorizedOracles` or `VerifierKey::Admin` via a generic
      setter.
- [ ] Negative tests exist for: non-admin calling admin functions, revoked
      oracle attempting to resolve, and unauthenticated caller attempting
      `require_auth()`-gated calls.

## 4. Front-Running

- [ ] Because oracle submissions and resolutions are on-chain and
      observable in the mempool/simulation phase, confirm there's no
      economically exploitable window where a party can act on a
      not-yet-committed `resolve_prediction` call (e.g. front-run a
      close-to-threshold resolution if there's any dependent action, like a
      payout contract reading `Resolution` immediately after).
- [ ] If a downstream payout/settlement contract consumes `Resolution`,
      confirm there's no way to front-run the *initial* oracle whitelist
      call or a resolution submission to lock in a stale price.
- [ ] Consider (and document a decision on) whether `resolve_prediction`
      should require the prediction's `deadline` to have passed
      (`env.ledger().timestamp() >= deadline`) so oracles can't resolve
      early based on favorable interim prices before the true settlement
      time — confirm this check exists in the real `resolve_prediction`
      body (not visible in the code shared for this review; **verify
      directly in source**).

## 5. Oracle Manipulation

- [ ] Only addresses in `VerifierKey::AuthorizedOracles` can call
      `resolve_prediction` — confirmed enforced via `is_oracle_authorized`
      per `storage.rs`.
- [ ] There is no single point of failure where **one** compromised oracle
      key can resolve arbitrarily large numbers of predictions with no rate
      limit, price-deviation check, or multi-oracle confirmation. Document
      the trust model explicitly: is this single-oracle-per-resolution by
      design, or should it require N-of-M agreement? If single-oracle by
      design, this must be an explicit, documented risk-acceptance, not an
      oversight.
- [ ] `add_oracle` / `remove_oracle` changes take effect immediately with no
      timelock — confirm this is acceptable for the threat model, or add a
      timelock/event-monitoring requirement.
- [ ] Confirm oracle price submissions can't reference an already-resolved
      prediction (`AlreadyResolved` guard) to prevent a compromised or
      buggy oracle from overwriting a legitimate resolution.
- [ ] Confirm events are emitted on every oracle add/remove and every
      resolution (`OracleAdd`, `OracleRem`, resolution event) so off-chain
      monitoring can detect anomalous oracle behavior (e.g. a new oracle
      immediately resolving many predictions).

---

## Sign-off

| Category            | Checked by | Date | Notes |
|----------------------|-----------|------|-------|
| Reentrancy            |           |      |       |
| Overflow/Underflow    |           |      |       |
| Access Control        |           |      |       |
| Front-Running         |           |      |       |
| Oracle Manipulation   |           |      |       |

Mainnet deployment is blocked until every row above is checked off and
linked to either a passing test, a `cargo audit`/`cargo clippy` clean run, or
an explicit documented risk-acceptance.