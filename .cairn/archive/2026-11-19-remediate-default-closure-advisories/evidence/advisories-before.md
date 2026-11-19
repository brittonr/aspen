# Advisories before the refresh (b6971ceab)

| class | crate | advisory | title |
| --- | --- | --- | --- |
| unmaintained | ansi_term@0.12.1 | RUSTSEC-2021-0139 | ansi_term is Unmaintained |
| unmaintained | atty@0.2.14 | RUSTSEC-2024-0375 | `atty` is unmaintained |
| unmaintained | bincode@1.3.3 | RUSTSEC-2025-0141 | Bincode is unmaintained |
| unmaintained | bitmaps@2.1.0 | RUSTSEC-2026-0247 | bitmaps is unmaintained |
| unmaintained | bitmaps@3.2.1 | RUSTSEC-2026-0247 | bitmaps is unmaintained |
| unmaintained | im-rc@15.1.0 | RUSTSEC-2026-0250 | im-rc is unmaintained |
| unmaintained | paste@1.0.15 | RUSTSEC-2024-0436 | paste - no longer maintained |
| unmaintained | proc-macro-error@0.4.12 | RUSTSEC-2024-0370 | proc-macro-error is unmaintained |
| unmaintained | proc-macro-error@1.0.4 | RUSTSEC-2024-0370 | proc-macro-error is unmaintained |
| unmaintained | sized-chunks@0.6.5 | RUSTSEC-2026-0251 | sized-chunks is unmaintained |
| unmaintained | structopt@0.3.26 | RUSTSEC-2022-0104 | `structopt` is in maintenance mode |
| unmaintained | unsafe-libyaml@0.2.11 | RUSTSEC-2024-0449 | unsafe-libyaml is unmaintained |
| unsound | atty@0.2.14 | RUSTSEC-2021-0145 | Potential unaligned read |
| unsound | bitmaps@3.2.1 | RUSTSEC-2025-0167 | `Bitmap::try_from(&[u8])` can create invalid values |
| unsound | event-listener@5.4.1 | RUSTSEC-2026-0221 | `event-listener` allows `!Send` tags to cross thread boundaries via `S |
| unsound | lru@0.18.0 | RUSTSEC-2026-0253 | Potential use-after-free due to lack of panic safety in `LruCache::pop |
| unsound | rustix@1.1.4 | RUSTSEC-2026-0486 | xattr functions mark unwritten bytes as initialized when given a zero- |
| unsound | sized-chunks@0.6.5 | RUSTSEC-2026-0255 | Panic-safety unsoundness in `Chunk`, `RingBuffer`, and `InlineArray` ( |
| unsound | tokio@1.52.3 | RUSTSEC-2026-0386 | `AsyncFd::new` and related constructors violate I/O safety |
| unsound | zstd-safe@7.2.4 | RUSTSEC-2026-0395 | Use-after-free with `CParameter::StableInBuffer` |
| unsound | zstd-safe@7.2.4 | RUSTSEC-2026-0396 | `Seekable` reads a NULL or out-of-bounds seek table |
| unsound | zstd-safe@7.2.4 | RUSTSEC-2026-0397 | `OutBuffer::around_pos` exposes uninitialized memory |
| unsound | zstd-safe@7.2.4 | RUSTSEC-2026-0399 | Use-after-free through `init_using_cdict` and `init_using_ddict` |
| unsound | zstd-safe@7.2.4 | RUSTSEC-2026-0400 | Use-after-free of a dictionary after a one-shot call on a context with |
| unsound | zstd-safe@7.2.4 | RUSTSEC-2026-0401 | Out-of-bounds read in `train_from_buffer` when sample sizes overflow |
| unsound | zstd-safe@7.2.4 | RUSTSEC-2026-0402 | Heap buffer overflow in `CCtx::try_clone` on a multithreaded context |
| unsound | zstd-safe@7.2.4 | RUSTSEC-2026-0404 | Double free in `DCtx::try_clone` |
| unsound | zstd-safe@7.2.4 | RUSTSEC-2026-0405 | Out-of-bounds read with an out-of-range `InBuffer::pos` |
| unsound | zstd-safe@7.2.4 | RUSTSEC-2026-0406 | Use-after-free when changing a multithreaded context with an unfinishe |
| vulnerability | cap-primitives@3.4.5 | RUSTSEC-2026-0482 | Paths or symlinks ending in a slash escape the sandbox |
| vulnerability | crossbeam-epoch@0.9.18 | RUSTSEC-2026-0204 | Invalid pointer dereference in `fmt::Pointer` impl for `Atomic` and `S |
| vulnerability | crypto-bigint@0.5.5 | RUSTSEC-2026-0387 | Multiple timing variability issues in operations intended to be consta |
| vulnerability | ctutils@0.4.2 | RUSTSEC-2026-0385 | Non-constant-time mask generation and `u128` comparisons |
| vulnerability | h2@0.4.14 | RUSTSEC-2026-0258 | h2 unbounded empty DATA frames |
| vulnerability | hickory-net@0.26.1 | RUSTSEC-2026-0506 | Inverted NSEC3 comparison lets one record "prove" that almost any name |
| vulnerability | hickory-net@0.26.1 | RUSTSEC-2026-0507 | RRSIG signer name was not checked against the zone of the signed recor |
| vulnerability | hickory-net@0.26.1 | RUSTSEC-2026-0508 | NSEC wraparound detection trusted an unvalidated SOA record |
| vulnerability | hickory-net@0.26.1 | RUSTSEC-2026-0509 | NSEC3 records from another zone were accepted in proofs of nonexistenc |
| vulnerability | hickory-net@0.26.1 | RUSTSEC-2026-0517 | Spoofed malformed UDP datagrams abort DNS lookups |
| vulnerability | hickory-net@0.26.1 | RUSTSEC-2026-0518 | DNSSEC validation cache could reuse an Insecure result for a different |
| vulnerability | hickory-net@0.26.1 | RUSTSEC-2026-0519 | NXDOMAIN accepted for an empty non-terminal that the NSEC record prove |
| vulnerability | hickory-net@0.26.1 | RUSTSEC-2026-0520 | DNSSEC validation accepted a validly signed but unrelated RRset as the |
| vulnerability | hickory-net@0.26.1 | RUSTSEC-2026-0539 | Parent-side NSEC at a zone cut accepted as proof of nonexistence below |
| vulnerability | hickory-net@0.26.1 | RUSTSEC-2026-0540 | Child-side NSEC or NSEC3 record accepted as proof that a delegation ha |
| vulnerability | hickory-net@0.26.1 | RUSTSEC-2026-0541 | Parent-side NSEC3 at a zone cut accepted as proof of nonexistence at a |
| vulnerability | hickory-proto@0.26.1 | RUSTSEC-2026-0487 | Panic on a malformed `alpn` value when parsing SVCB or HTTPS records f |
| vulnerability | hickory-proto@0.26.1 | RUSTSEC-2026-0488 | Integer underflow when reporting a malformed TSIG record |
| vulnerability | hickory-proto@0.26.1 | RUSTSEC-2026-0489 | Panic when an SVCB or HTTPS parameter value is a single quote characte |
| vulnerability | hickory-proto@0.26.1 | RUSTSEC-2026-0490 | Zone file lexer panics on a token of 4096 or more characters |
| vulnerability | hickory-proto@0.26.1 | RUSTSEC-2026-0491 | Memory amplification from record counts in the DNS message header |
| vulnerability | hickory-resolver@0.26.1 | RUSTSEC-2026-0513 | Unbounded retries on truncated responses in the name server pool |
| vulnerability | hickory-resolver@0.26.1 | RUSTSEC-2026-0514 | `Resolver::lookup` and `lookup_ip` returned records that failed DNSSEC |
| vulnerability | hickory-resolver@0.26.1 | RUSTSEC-2026-0515 | Recursor followed every glueless name server in a referral, with no pe |
| vulnerability | hickory-resolver@0.26.1 | RUSTSEC-2026-0516 | Recursor kept querying cyclic glueless sibling delegations |
| vulnerability | imbl-sized-chunks@0.1.3 | RUSTSEC-2026-0292 | Double free / use-after-free in `Chunk` and `InlineArray` removal meth |
| vulnerability | rsa@0.9.10 | RUSTSEC-2023-0071 | Marvin Attack: potential key recovery through timing sidechannels |
| vulnerability | rustls-webpki@0.103.13 | RUSTSEC-2026-0382 | Certificate verification may accept distrusted intermediates |
| vulnerability | rustls@0.23.40 | RUSTSEC-2026-0285 | TLS 1.3 handshake messages incorrectly accepted across encryption leve |
| vulnerability | rustls@0.23.40 | RUSTSEC-2026-0346 | `Debug` impls for client session values expose TLS secrets |
| vulnerability | rustls@0.23.40 | RUSTSEC-2026-0348 | Brotli certificate decompression can allocate up to 1 GiB before authe |
| vulnerability | rustls@0.23.40 | RUSTSEC-2026-0349 | Unauthenticated ServerHello can downgrade later connections to classic |
| vulnerability | tracing-subscriber@0.2.25 | RUSTSEC-2025-0055 | Logging user input may result in poisoning logs with ANSI escape seque |
| vulnerability | webpki-roots@1.0.7 | RUSTSEC-2026-0381 | Bundled root certificates allow trust in distrusted emSign intermediat |
| vulnerability | zstd-safe@7.2.4 | RUSTSEC-2026-0398 | `Seekable::decompress` sets a `Vec` length beyond its capacity for off |
| vulnerability | zstd-safe@7.2.4 | RUSTSEC-2026-0403 | Uninitialized memory exposed when streaming decompression of a legacy  |
| vulnerability | zstd-sys@2.0.16+zstd.1.5.7 | RUSTSEC-2026-0410 | WebAssembly allocator shim corrupts memory on allocation failure |
| vulnerability | zstd@0.13.3 | RUSTSEC-2026-0391 | Data from an abandoned stream leaks into the next stream on a reused c |
