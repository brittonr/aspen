# Authority-boundary review verification — 2026-10-15

## Outcome and ownership

The reviewed outcome is **add no new authority format**. Current Molten consumers
need scoped admission, not an admitted Preserves payload-rewriting language.
`docs/caveat-attenuation-decision.md` records the consumer inventory, exact
Basalt/UCAN cohort, compact and final UCAN profile boundaries, future guard
rails, and why no implementation follow-up is required. The incorporation review
links to that decision. Runtime-spine and authority maintainers own this decision
and repeat the source review when consumers or the pinned cohort change.

This package changes prose and lifecycle requirements only. It adds no runtime
type, evaluator, token format, dependency, or integration. Its document/source
review uses the repository's prose-only exemption; no runtime tests are claimed.

## Source and semantic review

- Reviewed the nine inventory surfaces against their cited implementation paths:
  plugin grants; runtime authority/capability admission; effect-handle derivation;
  remote/control/bootstrap tickets; vat graph labels; retained filesystem store
  authority; scoped artifact views; peer projection admission; and world-branch
  authority mapping. Labels and supplied observations are not execution evidence.
- The locked Basalt source is
  `89675cd4f585f837323c049e4a25f7b94c903038`, including its vendored UCAN packages.
  Source review covers both compact token/caveat APIs and the final UCAN 1.0
  bounded IPLD argument policies and Invocation authorizer. No sibling HEAD was
  substituted for this cohort.
- Independent agentic review found an omitted final-profile inventory. The note
  was corrected and re-reviewed against `Policy::parse`/`satisfied`,
  `compose_chain`/`check_policies`, profile admission, and Basalt's final-profile
  replay gate. Amendment review reported no remaining material finding.
- The add-nothing outcome is explicitly permitted by this package's design.
  Future rewrite guard rails do not claim a filter or final-profile integration
  currently exists. The rule against an unreviewed second authority format is
  retained in the specification delta.

Agentic review is advisory. Its output is not runtime, security, readiness, or
publication authority.

## Executed checks

All commands run from this change's dedicated Molten worktree. The native
provider is selected with `nix run path:/home/brittonr/git/OnixResearch/cairn#cairn`;
its explicit policy is
`/home/brittonr/git/OnixResearch/cairn/cairn-policy/generated/cairn-policy.json`.
These are local verification locators, not product dependencies or release
identities.

- Changed-document link/structure check: three documents checked, 14 local link
  targets, no broken targets, missing titles, or unbalanced code fences. The two
  authority documents are included; the third is independently edited delivery
  documentation and does not expand this package's implementation scope.
- Native proposal, design, and tasks gates: `valid: true`, `verdict: PASS`.
- Documented repository validation with the explicit provider policy and
  `--strict`: `valid: true`, no findings or issues; pre-closeout receipt
  `48a9d2f0d682efa146b9f89cb136b89fe299110df6594dd05c95d9d29b63ae03`
  observed 52 active changes. Structural validity is not behavioral correctness.
- Native sync preflight: unblocked, with one ADDED requirement for
  `molten.runtime_spine.caveat_authority_boundary`; no other accepted requirement
  is replaced or removed.

Final gate, strict-validation, sync, and archive-preflight observations are retained
as native command receipts alongside this note. The archive-execution receipt is
stored separately at
`evidence/cairn-completion-2026-10-15/review-caveat-attenuation-boundary-archive.json`,
so the archived input tree is not edited after the native move.

No Octet, runtime, live-cluster, security, whole-system, or release-readiness
result is claimed for this prose-only package. Other active changes retain
their own acceptance blockers.
