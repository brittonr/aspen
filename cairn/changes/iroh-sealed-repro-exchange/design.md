# Design: Iroh sealed repro exchange

## Flow

1. Publisher verifies a sealed bundle locally.
2. Publisher exports the bundle and receipt chain as immutable blob content.
3. Publisher produces an exchange receipt naming node identity, bundle ref, blob ticket/ref, and included receipt refs.
4. Fetcher retrieves by explicit ticket/ref.
5. Fetcher verifies canonical hashes, sealed bundle evidence, redaction evidence, and signed receipts if required.
6. Fetcher imports verified content into the local evidence ledger or writes plain files.

## Receipts

`<repro-exchange-receipt-v1 "molten.transport.repro-exchange.v1" ...>` should bind:

- direction (`publish` or `fetch`);
- local node identity ref;
- remote peer identity ref when known;
- blob ticket/ref;
- bundle ref;
- receipt-chain refs;
- verification result;
- redaction/reveal status;
- import/export target refs.

## Trust boundary

Iroh provides transport and content movement. Molten gate acceptance still depends on canonical validation, receipt verification, policy/capability/budget/redaction evidence, and optional signature/trust-root requirements.

## Confidentiality

Encrypted/private bundle parts are fetched only as opaque encrypted content unless reveal authority is supplied. Fetching a blob never grants reveal rights.
