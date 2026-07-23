# Formal Verification Spec — Access Control Invariant

Contract: `PredictionVerifier`
Maps to acceptance criterion: *"Formal verification specs written for
`player-nft` (ownership invariant)"* — adapted here since this contract has
no NFT/token ownership; the equivalent trust-sensitive invariant is **who
may mutate admin/oracle state**.

## 1. State

```
admin              : Address                (VerifierKey::Admin)
authorized_oracles  : Set<Address>            (VerifierKey::AuthorizedOracles)
```

## 2. Invariants

**INV-1 (Single admin, authenticated).**
At any point after initialization, exactly one address is the admin, and
every state-mutating call gated by `require_admin` succeeds if and only if
the caller equals `admin` **and** that caller authenticated via
`require_auth()` for this invocation.

```
∀ call to an admin-gated function f, ∀ caller c:
  f(c) succeeds ⟺ (c == admin) ∧ auth(c) verified for this call
```

*Verification approach:*
- Positive test: admin address calls `set_authorized_oracle` /
  `remove_authorized_oracle`; succeeds.
- Negative test: any non-admin address (including a former admin, if
  rotation exists, and including an authorized oracle — oracle status must
  **not** imply admin status) calling either function; must panic with
  `Unauthorized`.
- Negative test: a call constructed with `caller == admin` as a plain
  parameter but without a valid `require_auth()` signature/context (e.g. in
  a test harness that doesn't call `.mock_auths()` for that identity) must
  fail — this specifically guards against a bug where `require_admin` only
  checks address equality and forgets to call `require_auth()`.

**INV-2 (Admin identity cannot be forged via parameter).**
The `caller: Address` parameter passed into admin functions cannot be used
to impersonate the admin without a matching authentication proof — i.e. no
function trusts the `caller` field's *equality* to `admin` without also
requiring *authentication* from that same address for that invocation.

*Verification approach:* code review confirms `require_admin` always pairs
the `admin == *caller` check with `caller.require_auth()` (both present in
current `storage.rs`) — regression test should fail the build/CI if either
line is ever removed independently. Add a unit test asserting both branches
of `require_admin` are exercised (admin mismatch panics with
`Unauthorized`; missing auth panics regardless of address match).

**INV-3 (Oracle whitelist mutation is closed under admin gating).**
`authorized_oracles` changes **only** as a result of `add_oracle` /
`remove_oracle`, and those are only reachable from `set_authorized_oracle`
/ `remove_authorized_oracle`, both of which call `require_admin` before any
state write.

```
∀ state transitions where authorized_oracles changes:
  the transition was preceded by a successful require_admin(caller) check
```

*Verification approach:* static check — grep/clippy lint (or manual review
checklist item) confirming no other public function in `storage.rs` or
`lib.rs` calls `add_oracle`/`remove_oracle`/`.set(&VerifierKey::
AuthorizedOracles, ...)` directly. Add this as a standing rule: any new
function touching `VerifierKey::AuthorizedOracles` or `VerifierKey::Admin`
must go through `require_admin` first, enforced via code review checklist
until a clippy custom lint or macro wrapper can enforce it mechanically.

**INV-4 (No admin bypass via cross-contract call).**
If any future function accepts a contract address and delegates a
privileged decision to it (as `ReputationContractClient` does for vote
weighting elsewhere in the crate), that external call must never be able to
directly set `VerifierKey::Admin` or `VerifierKey::AuthorizedOracles` —
those keys are written **only** from within `PredictionVerifier`'s own
contract functions, never on behalf of an external contract.

*Verification approach:* code review — confirm no `contractclient` trait in
this crate exposes a method that Soroban would let an external contract
invoke to write these storage keys. This is largely enforced automatically
by Soroban's storage model (each contract has its own storage namespace),
but should be explicitly noted here as a reviewed assumption, not an
oversight.

## 3. Open question to resolve before mainnet

- Is admin rotation (`set_admin` to a new address, post-init) a required
  feature? If yes, it needs its own invariant (old admin must
  `require_auth()` to hand off, and the new admin should not be settable to
  the zero/self-referential address). If no, confirm there is genuinely no
  reachable `set_admin` after initialization — an unused/dead admin-setter
  function left in the code is itself a finding.

## 4. Suggested tooling

- Unit tests using `soroban-sdk::testutils::Address::generate` for multiple
  distinct identities (admin, non-admin, oracle, non-oracle) to exhaustively
  cover the cross-product of "is admin" × "has valid auth".
- `cargo clippy -D warnings` in CI (see accompanying workflow) to catch
  unused/dead admin-setter code paths before they ship.