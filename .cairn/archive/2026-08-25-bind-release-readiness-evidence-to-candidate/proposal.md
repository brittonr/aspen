# Bind release-readiness evidence to one candidate

## Why

The release-candidate gate requires evidence references, but those references do not identify the candidate source that each artifact evaluated. A passing receipt can therefore combine valid artifacts from different source revisions.

## What Changes

- Represent every release-readiness artifact as an artifact reference paired with its candidate source reference.
- Require every source reference to equal the release candidate's reviewed source reference.
- Deny missing, malformed, or mismatched evidence bindings.
- Record the bindings in a versioned canonical release-candidate receipt.
- Expose explicit candidate bindings through the operator command and document their bounded meaning.

## Impact

This change affects the pure release-readiness core, the production soak CLI, its canonical receipt schema, tests, operator documentation, and lifecycle specifications. It proves declared candidate binding only. It does not prove the truth of an external artifact or grant release authority.
