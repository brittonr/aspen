# Unison-inspired reference execution

Molten remote execution is artifact-ref based. A sender names the exact root artifact ref and a dependency closure descriptor, then supplies canonical arguments, an effect manifest ref, a requested handler profile, capability refs, policy/provenance/source-gate/resource evidence, and a reply route.

The receiver stays authoritative for admission. It computes the missing dependency set from the descriptor and its local state, chooses fetch refs, verifies fetched artifact hashes, applies local install/admission gates, binds handler-profile and capability/resource/provenance evidence, and emits a canonical admission receipt before any adapter can start.

The request format deliberately rejects mobile heap closures, raw live closures, host paths, process state, file descriptors, ambient environment, and transport-only authority. Sender-pushed refs outside the receiver-selected missing set are diagnostic evidence and do not authorize import or execution.

Core fixtures live under `workload::tests::remote_execution_*` and cover pass admission, missing dependency denial, unverified fetched refs, sender-pushed extras, mobile payload rejection, handler-profile mismatch, missing capabilities, local policy denial, and missing provenance/resource evidence.
