## ADDED Requirements

### Requirement: Molten-owned source-scope rows are cleared before scope narrowing
r[molten.octet_burndown.source_scope_actionables] Molten MUST remediate or explicitly fail closed on Molten-owned source-scope findings before source-gate evidence narrows or removes source-scope caveats, and MUST keep external/remapped classifications deterministic and reviewable.

#### Scenario: Molten-owned row is actionable
- GIVEN source-scope classification reports a no-disabled finding as Molten-owned source
- WHEN remediation evidence is prepared
- THEN the finding is either fixed by a behavior-preserving change or remains an active blocked caveat
- AND source-gate evidence does not treat that row as external or clean.

#### Scenario: External row is classification-only
- GIVEN a no-disabled finding is classified as generated, remapped, registry, or rustlib source
- WHEN source-scope evidence is reported
- THEN the classification receipt explains the basis for that classification
- AND the classification does not grant authority, policy, provenance, release trust, or permission to hide unknown source findings.
