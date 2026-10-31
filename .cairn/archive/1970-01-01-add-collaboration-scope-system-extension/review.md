# Implementation review: collaboration-scope system extension

## Disposition

The collaboration-scope extension, its pure core, shell, projections, simulation and
multiprocess fixtures meet the change's tasks under the declared test inputs. This is an
implementation review, not independent security approval, production readiness or release
authority. Executed commands and results are in `evidence/2026-10-31-verification.md`; the
reconstructed baseline, dependency identities and QM reference record are in
`evidence/2026-10-31-baseline-and-dependencies.md`.

## Audit of the inherited implementation

Every task was audited against source and tests before closure; each task line names the
implementing symbols and tests. The audit found these gaps and closed them in source:

- Manifest admission (`admit_extension`) had no caller and no test. Positive and negative
  tests now admit the exact manifest through ordinary tier and port admission, confirm that no
  fabric membership or placement port is bound, and deny a foreign extension identity or state
  schema, zero or oversized byte bounds, an oversized queue bound, a missing `recover`
  callback and a missing logical-time port.
- `Installation::from_host` had no test. A real admitted `Dispatcher` and an exactly opened
  consistency group now show that only a running host with its exact active group installs;
  a non-running host, a group for another generation and a merely declared group are stale.
- Operator status and the Lattice, Animus and Tile consumer views had no assertions. Tests now
  show status carries scope and state references, epochs, counts and non-claims but no member
  subjects, resources, policy references or policy bodies, and that consumer views carry an
  audience digest and a revalidation marker but no subjects, decisions or policy references.
- Snapshot decoding had no leak test. Appended payloads, a replaced non-bearer marker and raw
  identities in state or audience positions are now rejected.
- Task 5.4 named properties that existed only as one deterministic sequence. Two Hegel
  properties now check epoch monotonicity, deterministic and non-mutating previews, replay and
  conflict outcomes, bounded growth, allowed-authority honesty and one current owner scope.
  Both fail against a core mutated to advance two epochs or rebind a bound resource.
- The audience negative cases now include a wrong scope and a wrong resource.

No production code changed; the gaps were missing tests and evidence. Task 1.1's record was
reconstructed after implementation from the last mainline commit without collaboration source,
and says so.

## Reviewed invariants

- Collaboration membership never reads or writes fabric node membership; the extension binds
  no membership or placement port, and `src/fabric_membership` is unchanged since `d7e435e71`.
- Every transition previews a new state and leaves its input unchanged; denied requests commit
  nothing, identical replays keep the original result and conflicting replays are denied.
- Effective audience is the intersection of current membership, scope policy, resource policy
  and the organization floor; missing, duplicate or denied facts deny the whole request.
- `resource/share` and moves need their own exact abilities; read or use never implies them.
- Snapshots are evidence for one request; consumers must revalidate currentness before a
  protected effect.

## Non-claims

- No identity truth, global revocation freshness, Basalt or UCAN correctness, transport
  confidentiality, external-effect safety, prior-disclosure erasure, exactly-once effects or
  release eligibility is claimed.
- The deterministic simulation replays recorded committed-log delivery; it is not live
  consensus or disk evidence. The three-process scenario runs on one host.
- Installed native executable acceptance through the source-gated retained public host is
  `route-installed-native-effects` work and is not evidenced here.
