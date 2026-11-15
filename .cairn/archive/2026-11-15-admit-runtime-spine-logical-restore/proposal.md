# Admit runtime-spine logical restore

Issue: brittonr/aspen#18. Replace the public file-copy-only composition with one explicit runtime-spine logical profile. Compose the existing restore shell, node-host typed local stores and RuntimeState owner. Observe the running executable cohort; never derive destination facts from the supplied descriptor. Preserve opaque/VM denial.

The resume boundary is a committed runtime-spine turn: complete assertions, observers, facets, pending messages, logical clock, RNG and effect sequence. External effects are retained as unresolved records and never dispatched by restoration. This does not resume native services, Wasm instances, sockets, distributed membership or arbitrary task models.
