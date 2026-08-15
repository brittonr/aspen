# Change: contract-export-envelope-standardization

## Why

Some Nickel-authored contract exports already include explicit schema metadata and stable identities, while plugin extension authoring and a few fixture surfaces are thinner. Without a consistent envelope, reviewers must infer which schema, version, source language, and identity a checked-in export represents before binding it into evidence.

## What

- Define a standard envelope shape for Nickel-authored contract and fixture exports: schema id, schema version, source language, stable identity, and payload.
- Apply the envelope to contract export surfaces that currently lack explicit metadata, preserving compatibility with reviewed Preserves evidence.
- Document that metadata identifies evidence shape only and grants no runtime authority.

## Impact

Evidence review becomes more uniform. Runtime gates can bind explicit metadata and content refs without treating authoring metadata as permission, policy, or source-gate authority.
