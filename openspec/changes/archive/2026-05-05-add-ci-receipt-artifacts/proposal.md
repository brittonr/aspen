# Add CI receipt artifact evidence

## Why

Native CI run receipts already give operators a schema-versioned run, stage, and job summary by run ID. Dogfood receipts can point to the CI run ID, but a native CI receipt still leaves build artifacts behind a second discovery step: operators must inspect job IDs, then issue separate artifact queries per job. That weakens the receipt as a durable evidence object for Aspen's self-hosting claim.

Including bounded artifact metadata in the native CI run receipt makes the CI receipt a better operator handoff: it ties run, job, artifact names, blob hashes, sizes, and content types together without scraping logs or exposing download tickets.

## What Changes

- Extend CI run receipt jobs with artifact metadata entries collected from the existing `_ci:artifacts:{job_id}:*` KV records.
- Keep artifact collection bounded and deterministic.
- Fail the receipt request explicitly if artifact metadata scan fails instead of returning partial/fabricated evidence.
- Update CLI human output to summarize artifact count while JSON output carries the full artifact metadata.

## Impact

- Public/operator-visible CI receipt schema gains artifact metadata in job entries.
- No secret material or blob tickets are embedded in receipts; follow-up downloads still use explicit artifact retrieval.
- Existing CI status, logs, output, and artifact commands keep their behavior.
