# Cluster harness fixture file admission

## Prerequisite provenance and ownership

The clean canonical base at `81baf80cb667a22277be22c140d34829e5c7da7f`
has a 512-node parser but no byte cap or bounded cluster-CLI reader.
`aspen-octet-quality-owner-20260929` supplies a public parser
`MAX_CLUSTER_MANIFEST_BYTES = 1_048_576`, a pre-line-scan byte guard,
the exact-bound/+1 parser test and cluster CLI
`File::open(...).take(limit + 1).read_to_string(...)`.
Port only those source/test snippets verbatim to the split clean owner.
The detached composition already contains them, so its parser/CLI
and accepted specs remain untouched. The new clean-based owner is
**NOT source-isolated** and MUST wait for the quality owner in
adoption/dependency order. Evidence records the quality owner HEAD
and full parser/CLI file hashes plus split-port correspondence.

## Fixture boundary

Keep `validate_execution_input` first. Move the pure `plan_run`
ahead of `prepare_output_roots` in both code layouts; it opens and
reads only up to `MAX_CLUSTER_MANIFEST_BYTES + 1` bytes into a
String, then delegates to the existing byte/512-node parser before
any state or output root creation/removal. A valid 1-MiB fixture is
fully read and remains eligible; one extra byte returns the parser's
current count error. Materialization and child startup stay after
root preparation, with no redundant fixture read or second cap.
An actual `harness-run --force` regression checks prior state and
output sentinel files survive +1 denial, no index/child state is
staged, and an exact-bound fixture spawns a real child successfully.

## Independent ticket-path walker audit

`collect_ticket_paths` in split `p004` / composed `runner.rs`
checks `MAX_TICKET_FILES = 1,024` against matched tickets only,
then eagerly collects every entry in each `read_dir` into a Vec
and pushes every directory/non-ticket path into `pending`. The
4,096 `MAX_RUN_ARTIFACTS` applies to indexed run-directory output,
not arbitrary child state-root entries; 512 nodes and 1,024 tickets
also cannot bound those entries. The exact missing owner-selected
budget is a traversal bound over every discovered filesystem entry
(directories, non-ticket files, tickets and symlinks) across node
state roots, including per-directory enumeration and pending queue.
Do not extend an unrelated artifact index cap or modify this walker.

## Verification scope

Reproduce the staged-output violation before editing the harness
producer. Verify parser prerequisite provenance and source hashes;
exercise focused exact/+1 parser and actual harness-run integration,
owner/composition fmt, Clippy, nonincremental library Octet and strict
Cairn. Record residual static warnings and the adoption dependency.
