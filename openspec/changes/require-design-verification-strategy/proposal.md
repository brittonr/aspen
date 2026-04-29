## Why

Review metrics repeatedly flag designs that describe implementation choices without mapping requirements to checks. That causes later task plans to miss negative paths and makes design-stage approval weaker than implementation needs.

## What Changes

- Require a `## Verification Strategy` section in design artifacts for non-trivial changes.
- Require the section to map requirement or scenario IDs to concrete build, test, lint, gate, or proof rails.
- Gate designs that add or modify specs but omit verification coverage.

## Capabilities

### New Capabilities
- `openspec-governance.design-verification-strategy`: Designs carry requirement-to-rail verification mapping.

## Impact

- **Files**: OpenSpec design template/instructions, design gate rules, tests/fixtures for missing strategy and missing negative-path mapping.
- **APIs**: None.
- **Testing**: Gate fixture for passing strategy and failing missing strategy.

## Verification

Run design-gate fixtures and a real design gate against a sample change with mapped positive and negative checks.
