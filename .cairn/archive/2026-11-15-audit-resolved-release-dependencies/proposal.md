# Resolved release dependency audit

Issue #28 requires an actual maintained RustSec scan, not a caller-authored exact-version advisory list. Add a pinned offline cargo-audit adapter and closure-wide SPDX, source and notice observations for the default release profile. Keep owned-license and release-dependency gates authoritative. No clean result is possible with missing, stale or mismatched scanner/database/source evidence. No legal or runtime-security qualification is claimed.
