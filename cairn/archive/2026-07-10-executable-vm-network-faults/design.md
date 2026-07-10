# Design: executable-vm-network-faults

## Overview

Add an imperative VM-network fault shell around pure descriptor, receipt, support-matrix, and validation cores. The shell chooses a declared backend, applies a bounded fault, runs a child workflow, restores the link, and records canonical evidence.

## Backend model

A backend probe records backend id, target link, topology ref, supported host status, cleanup strategy, diagnostics, and caveats. Supported backends may use NixOS test-driver network controls, VM-local network tools, or a future explicit simulator adapter, but each backend must declare its scope.

## Executable fault flow

For each executable network fault:

1. emit preflight and support evidence;
2. inject delay/drop/partition/rejoin/asymmetric latency with bounded duration;
3. execute a declared child workflow;
4. run cleanup and post-fault connectivity/readback checks;
5. emit a fault receipt and validation receipt.

## Negative boundaries

Missing support, failed injection, missing cleanup, missing child refs, wrong topology, unavailable-as-pass, and log-only claims deny. Cleanup evidence is required before pass evidence can be accepted.
