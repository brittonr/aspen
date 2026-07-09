# Runtime limit profiles

Runtime limit profiles select effective operator budgets under compiled, named Rust hard caps. They tune resource use for service ticks, queues, live transport attempts/timeouts, framed sessions, chunks, retention scans, and harness runs; they do not raise hard caps or grant authority, policy admission, provenance, source-gate trust, retention clearance, transport authority, execution permission, or release eligibility.

Admission is a pure core over explicit profile values, hard-cap descriptors, and CLI/profile overrides. It denies non-positive values, one-past-hard-cap values, widening overrides unless explicitly allowed by the caller, and incoherent unit relationships such as join timeout greater than listener timeout, frame bytes greater than session bytes, attempts greater than queue depth, or retention scan bounds smaller than queue depth.

Receipts/readbacks bind the admitted profile ref, effective values, diagnostics, and a default-budget caveat when built-in local defaults are used instead of a reviewed operator profile.
