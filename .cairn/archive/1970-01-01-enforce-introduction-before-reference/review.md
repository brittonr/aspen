# Review: enforce introduction before reference

The last open task (sender-side refusal) was deferred until a receiver-owned sender view existed. The separately governed `recorded-remote-session-source` change now supplies that view, and every build site consumes it. Executed evidence: `evidence/2026-10-30-sender-refusal.md` plus the 2026-10-21 receiver-side verification.

Denials remain local evidence, not protocol error replies; the peer-visible surface is unchanged. Accepted for archive.
