# Design: retention-remote-clearance-live-two-node-harness

## Overview

The harness provisions requester and peer node roots, starts bound local Iroh gossip endpoints for each live receiver, and uses the existing retention request-send and response-send APIs to publish node-control live ingress envelopes. It drains each receiver through the node-control live receive path so the final workflow receives real `node-control-live-transport-receipt-v1` receive evidence and real ingress refs.

## Evidence setup

Each direction imports the same evidence into both sender and receiver roots:

- a bound live ticket for the receiver;
- a peer-admission receipt binding sender peer id, receiver node id, ticket, and topic;
- an authority grant permitting the sender to perform the `gate` operation against the receiver.

This mirrors operator setup for the multi-host UX while keeping the test local.

## Safety boundary

The final retention import-workflow still runs `retention-remote-gc-clearance-import-v1` before storing usable peer clearance. The test asserts that the live workflow and destructive admission pass only because the import stores matching peer clearance; live send/receive receipts remain binding diagnostics, not authority.

## Validation

The test should run under normal `cargo test`, `cargo nextest`, and Nix nextest. It should fail closed if the live send does not produce a transport receipt, if the receive path does not enqueue ingress, or if the final workflow rejects the real evidence binding.
