# `unsafe` Block Audit

Contract: `prediction-oracle-verifier`

## Method

```bash
grep -rn "unsafe" src/
```

## Result (based on `lib.rs`, `storage.rs`, `types.rs`, `errors.rs` reviewed)

No `unsafe` blocks were found in the source reviewed for this issue.

| File | Line | Block | Justification |
|------|------|-------|----------------|
| —    | —    | none found | n/a |

## Process going forward

If any `unsafe` block is introduced in a future PR, it must be documented
here **before merge**, following this template:

```
| src/foo.rs | 42 | `unsafe { ptr::read(...) }` | <why it's necessary, why
it's sound, what invariant the caller must uphold, and what would happen if
that invariant were violated> |
```

CI should fail (or at minimum warn loudly) if a PR introduces a new
`unsafe` block that isn't reflected in this file — recommend adding a
simple grep-diff check to the security workflow:

```yaml
- name: Check for undocumented unsafe blocks
  run: |
    UNSAFE_COUNT=$(grep -rn "unsafe" src/ | wc -l)
    DOC_COUNT=$(grep -c "^| src/" docs/security/UNSAFE_BLOCKS.md || true)
    if [ "$UNSAFE_COUNT" -gt "$DOC_COUNT" ]; then
      echo "::error::Found $UNSAFE_COUNT unsafe usages in src/ but only $DOC_COUNT documented in UNSAFE_BLOCKS.md"
      exit 1
    fi
```

(This check is included as a commented-out optional step in
`.github/workflows/security-audit.yml` — enable once the counting logic has
been tuned to your actual file layout, since the naive grep above will also
match the word "unsafe" inside comments/strings.)