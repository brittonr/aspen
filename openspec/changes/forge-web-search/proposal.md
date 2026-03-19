## Why

There's no way to find anything in the forge web UI. With multiple repos or large codebases, you're stuck browsing file trees manually. Repo filtering and code search are table-stakes for any code host.

## What Changes

- Add a search bar to the repo list page that filters repos by name (client-side, instant)
- Add a code search page per repo (`/{repo}/search?q=...`) that walks the tree at HEAD, greps blob contents, and returns matching files with context lines
- Add a search box to the repo navigation tabs

## Capabilities

### New Capabilities

- `repo-search`: Client-side repo name filtering on the repo list page
- `code-search`: Server-side full-text search across file contents at HEAD for a given repo

### Modified Capabilities
