# Advisories after the refresh

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
| vulnerability | crypto-bigint@0.5.5 | RUSTSEC-2026-0387 | Multiple timing variability issues in operations intended to be consta |
| vulnerability | imbl-sized-chunks@0.1.3 | RUSTSEC-2026-0292 | Double free / use-after-free in `Chunk` and `InlineArray` removal meth |
| vulnerability | rsa@0.9.10 | RUSTSEC-2023-0071 | Marvin Attack: potential key recovery through timing sidechannels |
| vulnerability | tracing-subscriber@0.2.25 | RUSTSEC-2025-0055 | Logging user input may result in poisoning logs with ANSI escape seque |
