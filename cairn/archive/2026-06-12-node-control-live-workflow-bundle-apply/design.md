# Design: Node Control Live Workflow Bundle Apply UX

## Apply command

`molten node live-workflow-bundle-apply` reads a live workflow bundle and writes a `node-control-live-workflow-bundle-apply-receipt-v1` to stdout or `--receipt-out`. It accepts the same expected node/topic/endpoint/peer/operation/scope/freshness options as verify, gate, and import. `--gate-receipt` supplies an operator gate receipt; `--require-gate-receipt` fails closed when it is absent.

The command first recomputes bundle verification. When a gate receipt is supplied, apply parses it, requires a passing decision, and checks that the gate bundle ref and recomputed verify receipt ref match the current invocation. Only after those checks pass does apply run the existing bundle import path to materialize ticket, peer admission, authority grant, bundle, and supporting receipt artifacts in the sender state root.

## Dry-run and send modes

Apply is dry-run by default. With a request argument and no `--send`, apply builds the live ingress envelope and sender-state-root diagnostics without joining/publishing to Iroh. With `--send`, apply delegates to the existing bounded live-send path and records the nested send receipt ref/value. `--send` without a request denies before import.

## Receipt semantics

Apply receipts bind the state root, bundle ref, optional gate receipt ref, recomputed verify receipt ref, import receipt ref, imported refs, mode (`import`, `dry-run`, or `send`), optional envelope/operation/send refs, expected bindings, diagnostics, and next-step checks. Apply receipts are operational evidence only and remain non-authority/non-provenance.
