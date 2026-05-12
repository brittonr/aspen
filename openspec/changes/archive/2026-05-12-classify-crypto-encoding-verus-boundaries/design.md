## Context

The remaining crypto/encoding-heavy files include tuple encoding, commit hash, raft chain hash/verify, and secrets MAC specs. Verus cannot prove Blake3 collision resistance, HMAC key separation, or third-party tuple encoding order preservation from Aspen source code. Treating those markers like ordinary proof gaps creates noise and obscures the structural proofs that can be closed.

## Goals / Non-Goals

**Goals:**
- Minimize each trusted crypto/encoding boundary to the smallest axiom surface.
- Prove local shape facts such as fixed byte lengths, branch admission, and wrapper control flow.
- Add runtime tests or references for assumptions that remain external.

**Non-Goals:**
- Formal verification of Blake3, HMAC, postcard, tuple encoding libraries, or cryptographic collision resistance.
- Replacing production crypto/encoding dependencies.

## Decisions

### 1. Classify before attempting proof closure

**Choice:** First label every residual marker as provable-local, external-library shape, or crypto-security axiom.

**Rationale:** This avoids wasting effort on impossible cryptographic proofs while still exposing easy shape closures.

### 2. Keep residual axioms local and named

**Choice:** Residual markers should remain close to the modeled function with a comment naming the assumption and evidence, not buried in a generic helper.

**Rationale:** Auditors need to see exactly what is trusted.

## Risks / Trade-offs

**Over-classification** → Require a proof attempt or clear reason before labeling a marker as an axiom.

**Runtime tests mistaken for proof** → Tests are evidence for library integration, not formal cryptographic proof; comments must say so.
