# Resolved dependency audit

Decision: **deny**. Profile: `molten-default`. Target: `x86_64-unknown-linux-gnu`. As of: `2026-11-13`.

Scanner: `cargo-audit 0.23.0`. RustSec: `4ef2df8e806a65e2f1ecc14f56f51249062b23d6`.

741 resolved packages; 204 findings. Complete paths, expressions, notices, exceptions and exact input refs: `report.json`. Raw maintained scan: `scanner.json`; failures: `status.json` and retained stderr.

- advisory: ansi_term 0.12.1 [actionable] RUSTSEC-2021-0139: ansi_term is Unmaintained
- unavailable-license: arraydeque 0.5.1 [actionable] missing or invalid SPDX expression; license-file alone is not an expression
- unavailable-license: arrayvec 0.5.2 [actionable] missing or invalid SPDX expression; license-file alone is not an expression
- unavailable-license: ascii-canvas 4.0.0 [actionable] missing or invalid SPDX expression; license-file alone is not an expression
- unavailable-license: asn1-rs-impl 0.2.0 [actionable] missing or invalid SPDX expression; license-file alone is not an expression
- unavailable-notices: asn1-rs-impl 0.2.0 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- advisory: atty 0.2.14 [actionable] RUSTSEC-2021-0145: Potential unaligned read
- advisory: atty 0.2.14 [actionable] RUSTSEC-2024-0375: `atty` is unmaintained
- unavailable-notices: backtrace-ext 0.2.1 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- unavailable-license: base64 0.13.1 [actionable] missing or invalid SPDX expression; license-file alone is not an expression
- unavailable-license: bigdecimal 0.4.10 [actionable] missing or invalid SPDX expression; license-file alone is not an expression
- unavailable-notices: binary-merge 0.1.2 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- advisory: bincode 1.3.3 [actionable] RUSTSEC-2025-0141: Bincode is unmaintained
- unavailable-license: bitflags 1.3.2 [actionable] missing or invalid SPDX expression; license-file alone is not an expression
- advisory: bitmaps 2.1.0 [actionable] RUSTSEC-2026-0247: bitmaps is unmaintained
- license-policy: bitmaps 2.1.0 [actionable] SPDX expression is outside reviewed audit license choices
- advisory: bitmaps 3.2.1 [actionable] RUSTSEC-2025-0167: `Bitmap::try_from(&[u8])` can create invalid values
- advisory: bitmaps 3.2.1 [actionable] RUSTSEC-2026-0247: bitmaps is unmaintained
- license-policy: bitmaps 3.2.1 [actionable] SPDX expression is outside reviewed audit license choices
- unavailable-notices: bounded-exec 0.1.0 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- unavailable-notices: bounded-exec-core 0.1.0 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- source-policy: bounded-http 0.1.0 [actionable] source/revision has no exact existing release-profile binding
- unavailable-notices: bounded-http 0.1.0 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- source-policy: bounded-http-core 0.1.0 [actionable] source/revision has no exact existing release-profile binding
- unavailable-notices: bounded-http-core 0.1.0 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- source-policy: bounded-tree-cap 0.1.0 [actionable] source/revision has no exact existing release-profile binding
- unavailable-notices: bounded-tree-cap 0.1.0 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- source-policy: bounded-tree-core 0.1.0 [actionable] source/revision has no exact existing release-profile binding
- unavailable-notices: bounded-tree-core 0.1.0 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- unavailable-license: bs58 0.5.1 [actionable] missing or invalid SPDX expression; license-file alone is not an expression
- unavailable-notices: cairn-core 0.1.0 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- advisory: cap-primitives 3.4.5 [actionable] RUSTSEC-2026-0482: Paths or symlinks ending in a slash escape the sandbox
- unavailable-notices: chaoscontrol-snapshot-descriptor 0.1.0 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- unavailable-notices: choregraph-history 0.1.0 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- unavailable-notices: codespan-reporting 0.11.1 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- source-policy: content-identity-core 0.1.0 [actionable] source/revision has no exact existing release-profile binding
- unavailable-notices: convert_case 0.4.0 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- unavailable-notices: cranelift-assembler-x64 0.136.5 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- unavailable-notices: cranelift-assembler-x64-meta 0.136.5 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- unavailable-notices: cranelift-bitset 0.136.5 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- unavailable-notices: cranelift-isle 0.136.5 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- unavailable-notices: cranelift-srcgen 0.136.5 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- advisory: crossbeam-epoch 0.9.18 [actionable] RUSTSEC-2026-0204: Invalid pointer dereference in `fmt::Pointer` impl for `Atomic` and `Shared` when the underlying pointer is invalid
- unavailable-notices: crunch-wasm-component-core 0.1.0 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- advisory: crypto-bigint 0.5.5 [actionable] RUSTSEC-2026-0387: Multiple timing variability issues in operations intended to be constant-time
- advisory: ctutils 0.4.2 [actionable] RUSTSEC-2026-0385: Non-constant-time mask generation and `u128` comparisons
- unavailable-license: curve25519-dalek-derive 0.1.1 [actionable] missing or invalid SPDX expression; license-file alone is not an expression
- unavailable-notices: defmt-parser 1.0.0 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- unavailable-notices: durable-authority-state-redb 0.1.0 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- unavailable-notices: enum-assoc 1.3.0 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- advisory: event-listener 5.4.1 [actionable] RUSTSEC-2026-0221: `event-listener` allows `!Send` tags to cross thread boundaries via `StackSlot`
- unavailable-license: ff 0.13.1 [actionable] missing or invalid SPDX expression; license-file alone is not an expression
- unavailable-license: filetime 0.2.29 [actionable] missing or invalid SPDX expression; license-file alone is not an expression
- unavailable-license: fixedbitset 0.4.2 [actionable] missing or invalid SPDX expression; license-file alone is not an expression
- unavailable-license: fnv 1.0.7 [actionable] missing or invalid SPDX expression; license-file alone is not an expression
- unavailable-license: fs2 0.4.3 [actionable] missing or invalid SPDX expression; license-file alone is not an expression
- unavailable-notices: fxprof-processed-profile 0.8.1 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- unavailable-notices: genawaiter 0.99.1 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- unavailable-license: genawaiter-macro 0.99.1 [actionable] missing or invalid SPDX expression; license-file alone is not an expression
- unavailable-notices: genawaiter-macro 0.99.1 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- unavailable-license: genawaiter-proc-macro 0.99.1 [actionable] missing or invalid SPDX expression; license-file alone is not an expression
- unavailable-notices: genawaiter-proc-macro 0.99.1 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- unavailable-license: group 0.13.0 [actionable] missing or invalid SPDX expression; license-file alone is not an expression
- advisory: h2 0.4.14 [actionable] RUSTSEC-2026-0258: h2 unbounded empty DATA frames
- unavailable-notices: hardened-wasmtime-runtime 0.1.0 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- unavailable-notices: hardened-wasmtime-shell 0.1.0 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- advisory: hickory-net 0.26.1 [actionable] RUSTSEC-2026-0506: Inverted NSEC3 comparison lets one record "prove" that almost any name does not exist
- advisory: hickory-net 0.26.1 [actionable] RUSTSEC-2026-0507: RRSIG signer name was not checked against the zone of the signed records
- advisory: hickory-net 0.26.1 [actionable] RUSTSEC-2026-0508: NSEC wraparound detection trusted an unvalidated SOA record
- advisory: hickory-net 0.26.1 [actionable] RUSTSEC-2026-0509: NSEC3 records from another zone were accepted in proofs of nonexistence
- advisory: hickory-net 0.26.1 [actionable] RUSTSEC-2026-0517: Spoofed malformed UDP datagrams abort DNS lookups
- advisory: hickory-net 0.26.1 [actionable] RUSTSEC-2026-0518: DNSSEC validation cache could reuse an Insecure result for a different name
- advisory: hickory-net 0.26.1 [actionable] RUSTSEC-2026-0519: NXDOMAIN accepted for an empty non-terminal that the NSEC record proves exists
- advisory: hickory-net 0.26.1 [actionable] RUSTSEC-2026-0520: DNSSEC validation accepted a validly signed but unrelated RRset as the answer
- advisory: hickory-net 0.26.1 [actionable] RUSTSEC-2026-0539: Parent-side NSEC at a zone cut accepted as proof of nonexistence below the cut
- advisory: hickory-net 0.26.1 [actionable] RUSTSEC-2026-0540: Child-side NSEC or NSEC3 record accepted as proof that a delegation has no DS record
- advisory: hickory-net 0.26.1 [actionable] RUSTSEC-2026-0541: Parent-side NSEC3 at a zone cut accepted as proof of nonexistence at and below the cut
- advisory: hickory-proto 0.26.1 [actionable] RUSTSEC-2026-0487: Panic on a malformed `alpn` value when parsing SVCB or HTTPS records from text
- advisory: hickory-proto 0.26.1 [actionable] RUSTSEC-2026-0488: Integer underflow when reporting a malformed TSIG record
- advisory: hickory-proto 0.26.1 [actionable] RUSTSEC-2026-0489: Panic when an SVCB or HTTPS parameter value is a single quote character
- advisory: hickory-proto 0.26.1 [actionable] RUSTSEC-2026-0490: Zone file lexer panics on a token of 4096 or more characters
- advisory: hickory-proto 0.26.1 [actionable] RUSTSEC-2026-0491: Memory amplification from record counts in the DNS message header
- advisory: hickory-resolver 0.26.1 [actionable] RUSTSEC-2026-0513: Unbounded retries on truncated responses in the name server pool
- advisory: hickory-resolver 0.26.1 [actionable] RUSTSEC-2026-0514: `Resolver::lookup` and `lookup_ip` returned records that failed DNSSEC validation
- advisory: hickory-resolver 0.26.1 [actionable] RUSTSEC-2026-0515: Recursor followed every glueless name server in a referral, with no per-query budget
- advisory: hickory-resolver 0.26.1 [actionable] RUSTSEC-2026-0516: Recursor kept querying cyclic glueless sibling delegations
- unavailable-license: id-arena 2.3.0 [actionable] missing or invalid SPDX expression; license-file alone is not an expression
- unavailable-license: ident_case 1.0.1 [actionable] missing or invalid SPDX expression; license-file alone is not an expression
- advisory: im-rc 15.1.0 [actionable] RUSTSEC-2026-0250: im-rc is unmaintained
- license-policy: im-rc 15.1.0 [actionable] SPDX expression is outside reviewed audit license choices
- advisory: imbl-sized-chunks 0.1.3 [actionable] RUSTSEC-2026-0292: Double free / use-after-free in `Chunk` and `InlineArray` removal methods when an element's `Drop` panics
- license-policy: imbl-sized-chunks 0.1.3 [actionable] SPDX expression is outside reviewed audit license choices
- unavailable-notices: inplace-vec-builder 0.1.1 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- unavailable-notices: inquire 0.9.4 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- unavailable-notices: iroh-base 1.0.0 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- unavailable-notices: iroh-dns 1.0.0 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- unavailable-license: iroh-docs 0.101.0 [actionable] missing or invalid SPDX expression; license-file alone is not an expression
- unavailable-license: iroh-gossip 0.101.0 [actionable] missing or invalid SPDX expression; license-file alone is not an expression
- unavailable-notices: iroh-metrics-derive 1.0.1 [actionable] no complete, bound local license/notice evidence; legal obligations remain a separate review
- unavailable-license: irpc 0.17.0 [actionable] missing or invalid SPDX expression; license-file alone is not an expression

Summary truncated at 100 findings; machine report is complete.

No legal/distribution approval, release authority or runtime security qualification is implied.
