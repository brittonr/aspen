# Design: Pin the fabric boundary compatibility fixtures

## Context

The fabric port and adapter migration landed in `3de348149` ("Keep fabric policy independent from mechanisms"). Its
parent `3de348149^` (`59aa8d153548bca4242f76bceb3245b0a2374c39`) is the last pre-migration tree. Between that commit
and the base of this change, the four `canonical.rs` projection files changed only through
import-qualification refactors (`1ab24c3fe`). The core input types the fixtures use have the same fields at both
commits.

## Decisions

### Decision: Generate the fixtures at the pre-migration commit

**Choice:** Run a throwaway generator (`evidence/premigration-generator.rs`) in a detached worktree at `3de348149^`.
The generator writes each projection's canonical Preserves bytes and a `refs.tsv` manifest (name, ref, byte length).
The fixtures are copied unchanged into `tests/fixtures/fabric-boundary/`.

**Rationale:** The scenario asks for "an accepted pre-migration fabric transition and receipt fixture". Generating at
the pre-migration commit meets that wording literally, so no scenario reword is needed.

### Decision: One shared set of explicit-input files

**Choice:** `tests/fabricboundarycompat/inputs.rs` (membership and time inputs), `ports.rs` (transport and
durability inputs), and `cases.rs` (the projection sequence and the `OrFail` test-error helper) hold every explicit
input. The generator includes the same files through `#[path]`; `evidence/fixture-inputs.b3` records their BLAKE3, which is
identical at both commits. The inputs use only APIs present at both commits. The one exception is the empty
transport state, which the caller passes in: `TransportState::default()` at the pre-migration commit and
`TransportState::new()` at head. Both produce an empty state with zero counters. Evidence refs are BLAKE3 over
fixed labels, not `DefaultHasher`, so the inputs do not depend on the standard library's hasher.

**Rationale:** With one input source, "equal explicit inputs" is structural rather than a claim. If the inputs drift,
the fixture comparison fails.

### Decision: Cover eight projections across the four fabric families

**Choice:**
- Membership: the source profile, the view, and a `Reserve` role-assignment transition (`canonical_assignment_transition`).
- Time: the admitted deterministic-simulation profile.
- Transport: the profile, and a `Register` transition.
- Durability: the profile, and an `append_log` transition.

**Rationale:** Each family's profile covers the canonical Preserves values, and each transition covers the
transition refs and the receipt-bearing evidence records. The live and simulation shells project through these
same functions, so pinning the pure projections covers both.

### Decision: Paired positive and negative tests

**Choice:**
- Positive: live bytes and refs equal the fixture bytes and the `refs.tsv` refs, and a strict decode of each fixture
  under its pinned ref equals the live value.
- Negative: for each of the eight cases, a one-field input mutation produces a different ref.
- Negative: a flipped last byte and a truncated fixture both fail `strict_canonical_decode_with_ref`.

**Rationale:** The negatives show that the pins are sensitive: a stale or constant pin would pass the positive test
but fail the mutation test.

### Decision: Add no Octet findings

**Choice:** The test files follow the repository's Octet profile without allows:
- Tests return `TestResult` and label failures through a local `OrFail` helper instead of `expect` or `unwrap`.
- Concrete types are written at their owner paths; the only import is the `OrFail` trait.
- The mutation checks are split into per-family tests, so every function stays within 70 lines.
- The inputs are split across three files, so every file stays within 300 lines.
- The test binary is named `fabricboundarycompat`, so the file name has no underscore.

**Rationale:** New code must not grow the Octet findings that the burn-down series is removing. Pinned Octet runs
(root and `-p molten --lib`) must show a zero per-lint delta against the base.

## No-spec classification

Accepted requirement text does not change; this change implements it as written. Semantic review inputs: the
proposal, this design, the acceptance criteria, the tasks, the generator and inputs, the fixture manifest, and the test
file.

## Failure behavior

- If any canonical projection changes value or ref for equal inputs, the positive test fails. A change that is meant
  to alter them must land as a separate versioned change that regenerates the fixtures.
- If an input stops producing a distinct ref when it changes, the mutation test fails.
- If a fixture file is corrupted or truncated, the tamper test fails.

## Risks / Trade-offs

- The fixtures pin eight projections, not every fabric projection. Other projections still rely on their own tests.
- A future intended change to one of these projections must regenerate the fixtures and record approval. The
  fixtures deliberately resist silent edits.
