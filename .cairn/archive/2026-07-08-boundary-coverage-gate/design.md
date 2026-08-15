## Context

`molten.testing.boundary_coverage` is already accepted. Current tests frequently assert specific deny paths, but coverage is not summarized in a single gate that can say which semantic boundaries are unexercised.

## Design

Define boundary classes such as:

- envelope send and receive;
- dataspace assert, retract, observe;
- policy pass and denial;
- capability pass and denial;
- effect request and response;
- hostcall request and denial;
- resource budget pass and exhaustion;
- replay pass and divergence;
- redaction pass and denial;
- adapter pass, denial, and failure;
- pass-evidence gate and diagnostic-only rejection.

Reports or traceability receipts should include a boundary coverage value with observed classes, missing classes, requirement ids, evidence refs, and exemptions.

The pure core classifies observations and computes coverage from typed report data. The CLI shell renders summaries and writes receipts.

## Validation

Positive tests cover reports with required positive and negative boundary classes. Negative tests cover missing denial path, missing pass path, stale evidence ref, unsupported boundary class, and exemption without evidence.
