# Change: production-profile-schema-metadata

## Why

The exported production profile currently has no explicit schema identity or source-language marker. Receipts can bind the profile ref, but reviewers cannot distinguish profile schema versions or detect a JSON file that has the same field shape but did not come from the reviewed Nickel contract boundary.

## What

- Add root metadata to exported production profiles, including schema id, schema version, source language, and profile identity.
- Bind the metadata into deployment-profile/startup evidence and documentation.
- Keep metadata evidence-only: it identifies the profile contract and source boundary but does not grant authority, source-gate trust, or adapter readiness.

## Impact

Profile receipts become easier to audit and migrate. Future schema changes can be versioned instead of inferred from field presence.
