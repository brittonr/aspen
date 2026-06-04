# Tasks: adapter-remote-proxy-preflight

- [x] [serial] r[molten.runtime.adapter_remote_proxy_preflight.adapter] Define and validate `<adapter-preflight-v1 ...>` receipts for manifests, artifact refs, ABI/schema refs, sandbox profiles, permissions, hostcalls, and conformance refs.
- [x] [serial] r[molten.runtime.adapter_remote_proxy_preflight.remote_proxy] Define and validate `<remote-proxy-preflight-v1 ...>` receipts for peer identity, endpoint/protocol refs, actor contracts, attenuation refs, transport profile, and trust requirements.
- [x] [serial] r[molten.runtime.adapter_remote_proxy_preflight.envelopes] Route adapter/proxy input, output, hostcalls, effects, and transcripts through canonical Preserves envelopes.
- [x] [serial] r[molten.runtime.adapter_remote_proxy_preflight.replay] Require verified execution transcripts/effect logs before adapter/proxy actors can satisfy deterministic gates.
- [x] [serial] r[molten.runtime.adapter_remote_proxy_preflight.authority] Keep transport/process success separate from capability authority, signer trust, and gate acceptance.
- [x] [parallel] r[molten.runtime.adapter_remote_proxy_preflight.gates] Add gate checks for adapter/proxy preflight binding, transcript replay, permission binding, and non-replayable exclusion.
- [x] [parallel] r[molten.runtime.adapter_remote_proxy_preflight.tests] Add negative tests for missing/tampered manifests, undeclared permissions, unknown peers, transcript mismatch, stale signatures, and ambient side-channel attempts.
