## 1. Repo list filtering (client-side)

- [x] 1.1 Add a text input with id `repo-filter` above the repo list in `repo_list` template
- [x] 1.2 Add inline `<script>` that listens to input events, hides `.card` elements whose repo name doesn't match, shows a "No matching repositories" message when all are hidden
- [x] 1.3 Add patchbay test assertion that the repo list page contains the filter input element

## 2. Code search — data layer

- [x] 2.1 Add `AppState::search_code` method that takes repo_id, ref_name, query string. Resolves ref → commit → tree, recursively walks tree collecting text blobs, greps each for the query, returns matches with context lines
- [x] 2.2 Add `SearchResult` and `SearchMatch` types to `state.rs`: file path, line number, line content, context lines, match count
- [x] 2.3 Add resource bounds: max 500 files walked, max 256KB per blob, max 50 matches, min 2 char query

## 3. Code search — route and template

- [x] 3.1 Add `/{repo_id}/search` route (GET with `?q=` query param) in `routes.rs`
- [x] 3.2 Implement `search` handler — parse query from URL, call `search_code`, render template
- [x] 3.3 Create `search_results` template — query input at top, result list with file paths and matching lines with context, link each line to `/{repo}/blob/{ref}/{path}#L{line}`
- [x] 3.4 Add search-specific CSS for result blocks and highlighted match lines

## 4. Search box in repo navigation

- [x] 4.1 Add a small search `<form>` to the tabs section in repo page templates (overview, tree, commits, issues, patches) that submits GET to `/{repo_id}/search`
- [x] 4.2 Add CSS for the inline search form in tabs

## 5. Line anchors in file viewer

- [x] 5.1 Add `id="L{n}"` to line number cells in `file_view` template so `#L{n}` anchors work

## 6. Testing

- [x] 6.1 Extend patchbay e2e test: search for a string known to be in README.md, verify 200 with result content
- [x] 6.2 Test empty search results (query that matches nothing)
- [x] 6.3 Test repo filter input is present on repo list page
