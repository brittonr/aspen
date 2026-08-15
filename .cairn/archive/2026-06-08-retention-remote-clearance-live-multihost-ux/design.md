# Design: retention-remote-clearance-live-multihost-ux

## Overview

The multi-host UX decomposes the existing loopback proof into explicit operator steps:

1. requester stores a canonical remote-clearance request and sends a node-control live ingress request whose target is the request ref;
2. peer reads the request artifact, stores a canonical response, and sends a node-control live ingress request whose target is the response ref and payload is the request ref;
3. requester imports the response through `retention-remote-gc-clearance-import-v1` and stores a `retention-remote-gc-clearance-live-workflow-v1` record that binds request, response, import, live send, receive, and ingress refs.

## Evidence boundary

The request/response send steps reuse `node-control-live-send-receipt-v1` and `node-control-live-transport-receipt-v1`. These receipts are diagnostics and binding evidence only. They do not store peer clearance in the retention store and do not grant deletion authority.

The import/workflow step first runs the existing import gate. Only a passing import stores embedded peer clearance for destructive admission. The live workflow then records the send/receive/ingress evidence refs and diagnostics around that import.

## Operator ergonomics

The CLI writes intermediate artifacts to files so operators can exchange request and response values by file, bundle, or live node-control envelope. A missing or denied send receipt is still materialized as evidence so the final workflow can fail closed with live diagnostics instead of silently accepting partial transport.

## Validation

Tests cover request-send denial on an offline ticket, response-send artifact construction, final workflow assembly from explicit live evidence, and the unchanged destructive-admission rule that only imported clearance is usable.
