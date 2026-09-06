# Design

The pure core admits an explicit operator-owned read grant for one canonical manifest and a finite reader-key set. The server adapter compares the authenticated Iroh peer against that grant before invoking the blob handler. The router serves only verified chunks of that manifest. The read grant does not imply retention, import, execution, or deletion authority.

A public handoff contains the canonical manifest value and bounded ordered blob locators. Parsing uses the existing manifest owner and an independently expected manifest ref. Hints cannot replace identity. The client uses the existing verification transition and BAO stream path through a remote descriptor rather than a borrowed server router. Private transport keys remain in capability-rooted identity state.

A fixture executable runs only in test VMs. The storage role uses a persistent test disk and canonical pin; the client receives no payload seed. Missing, malformed, reordered, overbound, and mismatched handoffs deny before networking. Wrong reader connections deny server-side. Raw locator/private state files are excluded from public receipts.

The baseline consumer cohort is the already-reviewed v0.1.0 revision a4f111690b6962f04d9320fd93d09c7dd1ad2fd0. Adapter, core adapter, and crypto sources are unchanged at upstream bb6f3830ee7327da9875ea85a8c8e25697eddc35. Current upstream's unrelated optional executable-extent dependency cannot be fetched on this worker; retain this blocker rather than weakening pins. Test the minimal delta in the reviewed cohort and report the exact source scope. Full latest-workspace acceptance remains separate.

The installed Cairn CLI currently rejects this repository's supplied policy with missing traceability_policy.profiles.assurance_level. Do not claim producer lifecycle gates until a compatible tool/policy cohort is available. Onix's consumer change remains independently gated.
