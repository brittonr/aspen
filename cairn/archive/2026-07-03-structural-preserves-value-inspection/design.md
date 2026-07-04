# Design: structural Preserves value inspection

## Scope

This change replaces semantic decisions based on `to_text(...).contains(...)` with pure structural traversal over `IOValue`. It covers marker detection, ambient/mobile-code token detection, and ref-retention checks that currently rely on rendered text.

## Proof checklist

- **Proof claim**: security and cleanup gates inspect Preserves structure rather than diagnostic rendering.
- **Out of scope**: removing text rendering for CLI/operator readback.
- **Trusted assumptions**: supported `IOValue` traversal exposes record labels, symbols, strings, byte strings, and children consistently.
- **Positive evidence**: structural markers and refs are found in nested records, sequences, sets, and dictionaries.
- **Negative evidence**: unrelated string payloads that merely contain rendered marker text do not trigger structure-only checks unless the check explicitly opts into string scanning.
- **Canonical refs**: inspected value ref, matched path, matched structural token, and check name.
- **Regeneration command**: `cargo test service job upgrade preserves_rail`.

## Functional core

Add a pure bounded visitor that walks `IOValue` and invokes explicit predicate functions. Each caller declares whether it inspects record labels, symbols, strings, byte strings, or refs. Shells pass parsed values and map matches into existing diagnostics.

## Non-goals

- No policy decision based solely on pretty-printed Preserves text.
- No broad schema migration in this slice; schema-backed validation is handled by a separate change.
