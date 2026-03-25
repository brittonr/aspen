## Context

The forge web UI serves HTML over HTTP/3 via iroh, proxied through TCP for browsers. It talks to the cluster via `AspenClient` RPC. All git objects (blobs, trees, commits) are stored in iroh-blobs, refs in Raft KV. The existing `ForgeServiceExecutor` handles GetTree, GetBlob, GetCommit, and Log operations. The web frontend uses maud templates and pulldown-cmark for markdown.

## Goals / Non-Goals

**Goals:**

- Show what changed in a commit: file list with add/modify/delete indicators, unified diff for text files
- Show clone instructions on repo overview: the exact `git clone aspen://...` command a user can copy
- Render markdown in issue/patch bodies and comments

**Non-Goals:**

- Side-by-side diff view (unified only for now)
- Syntax-highlighted diffs (plain monospace is fine)
- Binary file diffing
- Blame / line-level history
- New RPC operations — compose diffs from existing GetTree + GetBlob calls on the web frontend side

## Decisions

**Diff computation happens in the web frontend, not the cluster node.**

The web frontend already fetches trees and blobs. For a commit diff: fetch the commit, fetch parent commit, fetch both root trees, compare entries recursively, fetch blob content for changed files, compute line-level diff. This avoids adding a new RPC operation and keeps the cluster node stateless about presentation concerns. The tradeoff is multiple sequential RPC calls per diff view, but for typical commits (touching <20 files) this is fine.

Alternative: a `ForgeTreeDiff` RPC that does the comparison server-side and returns everything in one round trip. Deferred — optimize only if latency becomes a problem.

**Use the `similar` crate for line-level diffs.**

It's a well-maintained Rust diff library (used by insta). Produces unified diff output. No unsafe, no heavy dependencies. Alternative: implement Myers diff by hand — not worth it.

**Clone URL uses the ticket the web frontend was started with.**

The `AspenClient` holds the cluster ticket. The web frontend passes it through to the template. The URL format is `aspen://{ticket}/{repo_id}`. The `git clone` command is `git clone aspen://{ticket}/{repo_id} {repo_name}`.

**Markdown rendering reuses the existing `render_markdown` function from routes.rs.**

Move it to a shared location (templates.rs or a helper module) so it can be called from issue/patch templates too.

## Risks / Trade-offs

**[Many RPC calls for large diffs]** → Cap the number of files diffed (e.g. 50) and the max blob size for inline diff (e.g. 256KB). Show "file too large" or "too many files changed" placeholders.

**[Recursive tree walk for nested changes]** → Most commits don't touch deeply nested files. If a subtree hash is identical between parent and child, skip it entirely. Only recurse into subtrees that differ.

**[Ticket exposure in clone URL]** → The ticket is already embedded in every `git push` command and passed on the CLI. Showing it on the web page is consistent with the current auth model (no auth). When capability tokens land, the clone URL should use a read-only token instead.
