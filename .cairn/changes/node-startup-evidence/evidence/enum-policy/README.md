# Enum-policy review and marker-admission defect

The last full gate remains run 14: **30 node-host errors, zero warnings**.
This review neither changes that count nor authorizes startup. No full gate or VM was rerun.
No Molten production source, policy, feature flag, marker, or match arm changed.

## Four sites, three domains

| Site at product `ab3efbe97` | Domain | Required decision |
|---|---|---|
| `local_store/mod.rs:31` | `LocalStoreKind` | Map each local-store kind to its fixed directory name |
| `node/state/authority.rs:40` | `NodeStateNamespaceKind` | Map each namespace to its fixed relative directory |
| `node/state/filesystem.rs:157` | `NodeStateFileObservation` | Deny missing/non-regular files; read only an acquired regular-file handle |
| `node/state/namespace.rs:105` | `NodeStateFileObservation` | Distinguish absence, non-regular denial, and regular-file mode observation |

`docs/node-state-filesystem-authority.md` requires fixed capability-derived namespaces, regular-file-only reads, and no-follow observations.
These are deliberate domain decisions. Adding a variant requires reviewing its directory or observation behavior rather than inheriting a fallback.
The observation enum is not automatically an FSM state/event enum; do not apply FSM markers by analogy alone.

## What Octet actually specifies

At Octet `6b581a980e7c114304bf20743ad4ed705d97a113`, `cairn/specs/ocp-enum-match-lints/spec.md` requires:

- `r[octet.ocp_enum_match.fragile_exhaustive]`: diagnose unmarked exhaustive same-crate matches, default level `allow`.
- `r[octet.ocp_enum_match.fragile_exhaustive.sealed]`: accept a deliberately closed enum carrying the configured marker.
- `r[octet.ocp_enum_match.sealed_marker.config]`: honor the configured attribute name, not a coincidental name elsewhere.
- FSM profile requirements separately require explicit handling and reject wildcard arms in transition cores.

The four Molten findings match the current unmarked-enum policy. They are not the same kind of implementation false positive as the const or vector-growth defects.
A domain-sealing declaration needs a reviewed contract; it must not be a marker added merely to reduce the count.
Do not add catch-alls, blanket allows, or marker-looking documentation as a workaround.
The test-only registered tool attributes below are not an approved production feature/tool configuration change.

## Reproduced marker-admission defect

`src/safety/fragile_exhaustive_enum_match.rs::enum_has_marker` formats HIR attributes as debug text and searches for the configured marker or its final path segment.
This admits arbitrary doc text containing `sealed_enum` as if it were a sealing attribute.
The FSM owner also contains marker-recognition code; audit that owner for the same issue before sharing any repaired helper. This review does not establish its runtime behavior.

Task 10412 ran six probes using the retained March-21 compiler, repaired driver, and r5aw library:

| Probe | Exit | Measured outcome |
|---|---:|---|
| Reduced unmarked models, rustc | 0 | Four exhaustive functions compile |
| Same models, Dylint | 101 | Exactly four ordinary fragile-match diagnostics |
| Add one variant to each domain, rustc | 1 | Exactly four E0004 errors |
| Genuine sealing attributes, Dylint | 0 | No diagnostics |
| Genuine markers plus added variants, rustc | 1 | Exactly four E0004 errors remain |
| Documentation text only, Dylint | 0 | **Incorrectly suppresses all four diagnostics** |

The fixture reduces the namespace sets and replaces acquired-file behavior with scalar results. It proves compiler/marker behavior, not full filesystem semantics or actual domain admission.
Only `unknown_lints` and `fragile_exhaustive_enum_match` are denied in the direct-driver probes. The canonical full-gate flags remain unchanged.
Input identities matched before and after; no ICE markers appeared.
No compiler, driver, library, CLI, runtime, Mantle, Darkhttpd, or Stage0 build ran. The compiler produced fixture metadata only.

## Exact replay

From this Molten worktree:

```sh
sh .cairn/changes/node-startup-evidence/evidence/enum-policy/verify.sh \
  /home/brittonr/.local/state/onix/molten-node-vm/enum-review/run-1 \
  /nix/store/r5bzbvda2ydnz09c6vxvqhxsmh37nhpw-dylint-driver-5.0.0/bin/dylint-driver \
  /nix/store/r5aw3lz2sa6h2cg9lphy881w0w1ra2id-octet-0.1.0/lib/liboctet.so \
  /nix/store/1yvh3d6y3fj3xk2dgwczrp1dj5svd92c-rust-default-1.96.0-nightly-2026-03-21
```

That output exists; use a fresh absolute output for another replay.
The helper deliberately reproduces the defect. Its doc-only zero-diagnostic expectation must fail after a correct owner repair.
Logs and member digests are retained under `~/.local/state/onix/molten-node-vm/enum-review/` and in `private-digests.txt`.

## Next boundary

Repair exact attribute-path admission at the Octet owner before relying on sealing declarations.
Controls must reject doc strings, same-tail wrong namespaces, and unrelated attribute payloads while accepting the configured marker.
Then review explicit closed-domain declarations for the three Molten enums against their authority contracts, preserving compiler rejection of unhandled variants.
No startup-policy promotion, May-26 build/binding, approved cohort, normal-node VM, lifecycle closeout, or replay is established.
