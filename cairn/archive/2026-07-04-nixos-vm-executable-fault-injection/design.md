# Design: NixOS VM executable fault injection

## Scope

Extend the multi-node NixOS VM check with executable fault cases. Faults should run inside the NixOS test driver or VM nodes through explicit commands, with canonical receipts recording the injection and outcome.

## Proof checklist

- **Proof claim**: supported VM environments can execute bounded network, process, restart, storage, and state-root faults and bind their effects to canonical VM fault evidence.
- **Out of scope**: real WAN behavior, hardware failure modeling, adversarial transport proof, broad production deployment, and silently skipping unsupported host features as pass evidence.
- **Trusted assumptions**: NixOS test driver commands faithfully execute inside the declared VM topology; host KVM/QEMU support is reported accurately by the check.
- **Positive evidence**: executable partition/rejoin, duplicate send after restart, queued control recovery, and admitted workflow pass cases produce canonical child refs and pass/deny receipts as expected.
- **Negative evidence**: stale tickets, wrong authority, malformed frames, missing artifacts, permission-denied state roots, unsupported host features, and injected receipt corruption deny or mark unavailable before pass evidence is accepted.
- **Canonical refs**: VM topology ref, node evidence refs, fault injection refs, child workflow refs, pre/post state refs, VM fault run ref, unavailable/refusal refs, and diagnostic log refs.
- **Regeneration command**: explicit Nix check or app for VM fault injection, plus focused fixture/validator commands for negative cases.

## Fault execution model

Each executable fault case should have three phases:

1. preflight: validate host and VM support, capture pre-fault receipts or state refs;
2. injection: run the bounded fault command or state mutation with an explicit timeout and target;
3. observation: collect canonical child receipts, compare against expected pass/deny/unavailable behavior, and write the VM fault receipt.

## Functional core

Validation should remain pure: `VmFaultEvidenceInput -> VmFaultDecision`. The core receives parsed receipts, expected topology, host-support status, fault descriptors, pre/post refs, and diagnostics. Shell code owns NixOS driver commands, `iptables`/`tc` or equivalent invocations, filesystem mutations, and artifact copying.

## Evidence boundary

Executable VM fault receipts are platform integration evidence only. They do not grant authority, policy, provenance, resource, source-gate, retention, destructive-operation, deployment, or transport trust beyond the tested topology.
