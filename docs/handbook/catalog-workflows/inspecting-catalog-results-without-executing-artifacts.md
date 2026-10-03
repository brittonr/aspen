# Inspecting catalog results without executing artifacts

Mode: How-to

## Goal and prerequisites

Inspect an existing local registry's discoverable metadata, resolve a subject using its full canonical reference, and preserve the distinction between query evidence and execution permission. These recipes are source-checked, not executed. They assume an already available `molten` executable and a registry you are authorized to inspect. They do not install dependencies, initialize storage, import artifacts, or activate a service.

Set `CATALOG_REGISTRY` to that registry's real directory. Set `CATALOG_REF` only after obtaining a full reference from authorized evidence or a catalog result. The guards below reject unset values; they are not authority checks. Do not substitute illustrative hashes. If output may contain restricted metadata, use an appropriately controlled terminal and do not copy it into shared issue reports.

The actual CLI prefix is `molten test catalog`, not a top-level `molten catalog`. [Main's alias](../../../src/main.rs) routes to the catalog shell; the [root command include](../../../src/main/root/command.rs) reaches the declaration containing `Top::Test` and `Test::Catalog`.

## 1. Choose summary inspection before a view

Start with listing when the question is “which artifact records are visible?” Do not request payloads simply to learn a reference. The following block is supported by the [root declaration](../../../src/main/root/parts/command/p000/body.rs), [catalog arguments](../../../src/cli/core/catalog/command.rs), and [list handler](../../../src/cli/core/catalog/ops.rs). Source-checked; not executed.

```sh
molten test catalog list --registry "${CATALOG_REGISTRY:?Set an authorized existing registry directory}"
```

This invokes catalog discovery, not the artifact's code. Nevertheless, the implementation may read payloads for classifications. “Without executing” is not “without reading.” The result includes summaries and receipt output; it is not a plain sequence of artifact references suitable for blindly feeding another command.

Omitting `--ledger` intentionally limits this first recipe to registry discovery. If the subject exists only in a ledger, add the separately authorized ledger root using the declared `--ledger` option. Do not interpret its absence from a registry-only result as global nonexistence.

## 2. Narrow using a meaningful question

If the task is to inspect receipt artifacts that represent passing decisions, use the two corresponding filters. The block follows the [search arguments](../../../src/cli/core/catalog/command.rs), [search handler](../../../src/cli/core/catalog/ops.rs), and [filter semantics](../../../src/catalog/parts/mod/p006/body.rs). Source-checked; not executed.

```sh
molten test catalog search \
  --registry "${CATALOG_REGISTRY:?Set an authorized existing registry directory}" \
  --kind receipt --receipt-decision pass
```

Filters intersect. This is not a request for every object that is either a receipt or passing. `--kind receipt` matches artifact-kind metadata; a ledger-only receipt can have a different kind, so this recipe is deliberately not a universal receipt search.

For example, the [checked-in search fixture](../../../src/catalog/parts/mod/tests/m000/p000/body.rs) installs a receipt artifact representing an `apply` decision and then locates it with kind, dependency, decision, and text filters. Its represented `pass` remains fixture data. In real use, inspect the subject, producer, and applicable admission evidence before drawing a stronger conclusion.

## 3. View a known registry artifact without requesting its payload

Copy the full reference of the intended artifact, not the hash of its summary or the catalog receipt. The block follows the [view declaration](../../../src/cli/core/catalog/command.rs), [view handler](../../../src/cli/core/catalog/ops.rs), and [registry view branch](../../../src/catalog/parts/mod/p000/body.rs). Source-checked; not executed.

```sh
molten test catalog view \
  "${CATALOG_REF:?Set the full artifact reference from authorized evidence}" \
  --registry "${CATALOG_REGISTRY:?Set an authorized existing registry directory}"
```

No `--payload` is requested. For a registry artifact, the view therefore uses a `none` payload slot. Redaction defaults on in the declaration. Do not infer the same payload-omission behavior for ledger-only objects: the ledger view branch renders the ledger value and marks payload inclusion true even when the caller did not request a payload. If your requirement is strict metadata-only ledger inspection, stop at list/search summaries instead of using this view recipe with a ledger fallback.

## 4. Decide whether to preserve evidence files

By default, the shell prints receipt text along with item output. If review requires a receipt file, the declared `--receipt-out` option is available. Choose a fresh, approved output path and preserve the exact request context alongside it. The [output helper](../../../src/cli/core/catalog/io.rs) creates parent directories and uses a file write that can replace an existing file; the option is not collision protection.

Do not treat redirected stdout as one canonical result object. The CLI emits individual items, receipt text or a receipt-written notice, and separate diagnostics. MCP calls have a distinct response envelope and optional `--out` file, but their `decision` must still be read rather than inferred from process success.

## Stop conditions

Stop if the full reference is uncertain, the root is unauthorized, visibility policy is unresolved, or the requested inspection would expose restricted metadata. Do not remove hidden references to make a query succeed. Do not switch to raw rendering as a diagnostic shortcut. Discovery, redaction, and evidence collection never grant execution, mutation, provenance trust, or release readiness; see the [technical explanation](../../technical/foundations/evidence-and-authority-separation.md).

## Sources

- [Handbook](../README.md)
- [Architecture](../../architecture.md)
- [Technical companion: evidence and authority separation](../../technical/foundations/evidence-and-authority-separation.md)
- [Root CLI declarations](../../../src/main/root/parts/command/p000/body.rs)
- [Catalog CLI declarations](../../../src/cli/core/catalog/command.rs)
- [Catalog CLI handlers](../../../src/cli/core/catalog/ops.rs)
- [Output and receipt file handling](../../../src/cli/core/catalog/io.rs)
- [Catalog search and view implementation](../../../src/catalog/parts/mod/p000/body.rs)
