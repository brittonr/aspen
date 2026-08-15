# Design: executable VM fault support matrix

Add a matrix builder that declares fault kind, required host or VM capability, target node or link, command profile, expected outcome, preflight refs, required injection refs, required child workflow refs, post-fault refs, diagnostic log refs, and caveats. The builder is pure over explicit descriptor and receipt inputs.

Fault classes include network delay/drop/partition/rejoin when network control exists, process crash/restart through systemd or test-driver controls, duplicate send after restart, receipt write/readback, missing artifact, permission-denied state root, bounded disk pressure, unsupported host feature, tampered fault receipt, wrong topology, and log-only pass negative fixtures.

The VM shell owns capability probing and injection. It must produce `supported`, `unavailable`, or `denied` host-support evidence. Unsupported execution cannot be converted into pass evidence. Supported injections must bind pre-fault, injection, child workflow, and post-fault refs.

Validation should render a compact support table for reviewers and emit canonical deny diagnostics for unsupported pass claims, missing injection, missing child evidence, wrong topology, and log-only pass attempts.
