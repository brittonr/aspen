## Context

The forge web UI renders HTML server-side with maud templates. No JavaScript exists in the frontend yet. The `AppState` has methods for fetching trees and blobs via RPC. Tree walking is already implemented for diffs (`diff_trees_recursive`).

## Goals / Non-Goals

**Goals:**

- Filter the repo list by name instantly (no page reload)
- Search file contents within a repo at HEAD and show matching lines with context
- Keep search bounded (max files walked, max blob size, max results)

**Non-Goals:**

- Cross-repo search (search one repo at a time)
- Regex search (substring match is sufficient)
- Search indexing or caching (brute-force walk-and-grep per query)
- Search history or saved queries

## Decisions

**Repo filtering is client-side JavaScript.**

The repo list is already fully rendered. A small inline `<script>` hides non-matching cards as the user types. No server round-trip, instant feedback. This is the only JS in the entire frontend — kept minimal (< 20 lines).

**Code search is server-side tree walk + blob grep.**

The search handler resolves HEAD, recursively walks the tree, fetches each text blob, and checks for substring matches. Results are collected with surrounding context lines (2 lines above/below). This is a brute-force approach with no index — acceptable for repos under ~10K files.

**Resource bounds:**

- Max 500 files walked per search
- Max 256KB per blob (skip larger files)
- Max 50 result matches returned
- Search query minimum 2 characters

**Search results link to the file viewer with the matching line highlighted.**

The blob view page already has line numbers. Search results link to `/{repo}/blob/{ref}/{path}#L{line}` and the template adds `id="L{n}"` attributes to line number cells.

## Risks / Trade-offs

**[Slow on large repos]** → The 500-file and 50-result caps prevent runaway searches. For repos larger than that, code search will miss files in deep subtrees. A future improvement would be a background indexing job, but that's out of scope.

**[No result ranking]** → Results come back in tree-walk order (alphabetical by path), not relevance. Good enough for substring search.
