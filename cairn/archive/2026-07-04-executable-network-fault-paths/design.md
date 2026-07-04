# Design: executable-network-fault-paths

## Overview

Add a capability-probed network fault executor under the VM fault shell and keep all admission decisions in pure receipt validation. The executor attempts only declared operations against declared node/link targets, writes injection evidence, restores the network state, and returns observed child receipt refs to the existing fault receipt model.

## Capability probe

The probe records whether the VM image and NixOS test driver can perform the requested network-control operation. Supported backends may include test-driver network helpers, nftables, iptables, or traffic-control tools when available in the VM image. The selected backend, target link, cleanup action, and probe result are bound to canonical preflight evidence.

## Fault execution

Each executable network fault has the same phases:

1. preflight: validate topology, target link, backend support, and required child workflow refs;
2. inject: apply the bounded fault and write injection evidence;
3. observe: run the declared workflow command and collect child receipts;
4. cleanup: remove the fault and write cleanup evidence;
5. validate: emit a VM fault receipt with pass, deny, or unavailable decision.

## Negative boundaries

Pass evidence requires supported host status, injection evidence, child workflow refs, cleanup evidence, matching topology, and canonical diagnostics for denial or unavailable outcomes. Missing cleanup, stale topology, unsupported host pass, unrejoined partition, and log-only success all deny.
