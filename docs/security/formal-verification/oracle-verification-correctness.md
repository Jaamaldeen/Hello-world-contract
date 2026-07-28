# Formal Verification Spec — Oracle Verification Correctness

Contract: `PredictionVerifier`
Maps to acceptance criterion: *"Formal verification specs written for
`betting` (payout correctness)"* — adapted here to this contract's actual
core function, which is **verifying oracle-submitted outcomes correctly**
rather than paying out funds directly.

<!-- ASSUMPTION: the properties below are derived from storage.rs, types.rs,
     and the numbered comments in the pasted lib.rs (1. check oracle
     authorized → ... → 4. determine outcome → 5. emit event). The actual
     resolve_prediction body was not available in a compilable form in the
     material reviewed. Confirm each property against the real function
     before treating this spec as verified. -->

## 1. State

```
admin              : Address                       (VerifierKey::Admin)
authorized_oracles  : Set<Address>                   (VerifierKey::AuthorizedOracles)
resolutions[id]     : Option<Resolution>             (VerifierKey::Resolution(id))
prediction[id]      : { target_price, deadline, ... } (external / shared)
```

## 2. Invariants

**INV-1 (Resolution immutability).**
For every `prediction_id`, once `resolutions[prediction_id]` is `Some(_)`,
no subsequent contract call may change its value.

```
∀ id, ∀ calls c1 before c2:
  resolutions[id] == Some(r) after c1
  ⟹ resolutions[id] == Some(r) after c2   (same r, unchanged)
```

*Verification approach:* `save_resolution` must be reachable only from a
code path that first asserts `get_resolution(env, id).is_none()`, panicking
with `AlreadyResolved` otherwise. Test: call `resolve_prediction` twice with
valid but different `actual_price` values for the same `prediction_id`;
second call must revert and the stored `Resolution` must equal the first
call's result exactly (not partially overwritten).

**INV-2 (Result/price consistency).**
The stored `ResolutionResult` must always be the correct function of the
stored `actual_price` and the prediction's `target_price`.

```
∀ id where resolutions[id] == Some(r):
  r.result == Correct   ⟺  r.actual_price >= prediction[id].target_price
  r.result == Incorrect ⟺  r.actual_price <  prediction[id].target_price
```

*Verification approach:* property-based test generating random
`(target_price, actual_price)` pairs across the `u128` range (including
`0`, `u128::MAX`, and `target_price == actual_price` boundary), asserting
the stored `result` matches the arithmetic comparison every time. This also
covers the overflow-boundary case from the audit checklist §2.

**INV-3 (Oracle authorization at time of resolution).**
A `Resolution` can only be created by a call where the submitting address
was, at the moment of the call, a member of `authorized_oracles`, and that
address underwent `require_auth()`.

```
∀ id where resolutions[id] == Some(r):
  r.oracle ∈ authorized_oracles_at_call_time
  ∧ r.oracle.require_auth() was satisfied for this invocation
```

*Verification approach:* negative test — remove an oracle via
`remove_authorized_oracle`, then attempt `resolve_prediction` from that now
non-whitelisted address; must panic with `Unauthorized` (or equivalent) and
must not write a `Resolution`. Also test that the `oracle` field stored in
`Resolution` matches the authenticated caller, not an arbitrary
caller-supplied address (i.e. no spoofing another oracle's identity).

**INV-4 (No premature resolution).** <!-- ASSUMPTION: verify this check
     exists in the real resolve_prediction body -->
A prediction cannot be resolved before its `deadline` has passed.

```
∀ id where resolutions[id] == Some(r):
  r.resolution_timestamp >= prediction[id].deadline
```

*Verification approach:* test calling `resolve_prediction` with
`env.ledger().timestamp() < prediction.deadline`; must revert. If this
check does **not** exist in the current implementation, this is a
front-running / early-resolution risk (see Audit Checklist §4) and should
be either added or explicitly documented as an accepted risk before
mainnet.

**INV-5 (Event/state consistency).**
Every successful resolution emits exactly one `PredictionResolved`-style
event whose payload (`correct`, `actual_price`, `oracle`) matches the
persisted `Resolution` record.

*Verification approach:* integration test asserting emitted event data
equals `get_resolution(env, id)` immediately after the call, for both the
`Correct` and `Incorrect` branches.

## 3. Out of scope for this spec

- Any payout/fund-transfer logic. If a separate settlement contract reads
  `Resolution` to disburse funds, that contract needs its **own** formal
  spec covering fund-conservation (total paid out == total staked, no
  double-payout) — this document only covers the verifier's correctness,
  not money movement.
- Reputation-weighted voting (`create_idea`/`vote`/`VoteRecord` functions
  seen in the pasted `lib.rs`) — this appears to belong to a different
  contract entirely and is out of scope here. Flag for a separate audit
  issue if it's part of the same crate.

## 4. Suggested tooling

- Property tests via `proptest` or `soroban-sdk`'s `testutils` for
  boundary fuzzing (INV-2, INV-4).
- Consider a lightweight TLA+ or Alloy model for INV-1/INV-3 if the
  resolution flow grows more complex (multi-oracle consensus, disputes).