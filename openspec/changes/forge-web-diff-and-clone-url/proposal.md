## Why

The forge web UI can browse repos, files, commits, issues, and patches — but you can't see what a commit actually changed, and there's no indication of how to clone a repo. Diffs are the core unit of code review; without them the commit log and patch pages are lists of hashes. The clone URL answers the first question any visitor has.

## What Changes

- Add a commit detail page (`/{repo}/commit/{hash}`) showing parent-child tree diff with unified diff output
- Add a tree-diff RPC operation to compare two trees and return changed entries with blob content
- Display the `aspen://` clone URL and `git clone` command on the repo overview page
- Render issue/patch body and comments as markdown instead of plain text

## Capabilities

### New Capabilities

- `commit-diff`: Commit detail page with unified diff rendering — compare a commit's tree against its parent, show file-level changes (added/removed/modified) with line-level diffs
- `clone-url-display`: Show clone instructions on the repo overview page — the `aspen://` URL, `git clone` command, and ticket info

### Modified Capabilities

## Impact

- `crates/aspen-forge-handler/src/executor.rs` — may need a new RPC operation for tree-diff, or the web frontend can compose it from existing GetTree/GetBlob calls
- `crates/aspen-forge-web/src/routes.rs` — new route for commit detail page
- `crates/aspen-forge-web/src/templates.rs` — new commit detail template, clone URL section on repo overview, markdown rendering for issue/patch bodies
- `crates/aspen-forge-web/src/state.rs` — new methods for fetching diff data
- No new dependencies — `pulldown-cmark` already exists for markdown, diff computation can be done with `similar` crate or inline
