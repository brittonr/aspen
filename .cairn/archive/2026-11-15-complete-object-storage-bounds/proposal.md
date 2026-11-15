# Complete bounded object storage

## Why
Issues #8, #13 and #14 require a bounded recovery tier and honest parity evidence. Existing shells already implement signing and transition-based verification; this change closes specific bounds, classification and admission gaps without a second content core.

## What Changes
Fix cache accounting/reopen bounds, use verified local hits in the command shell, bound requests and freeze numeric endpoint/credentials at admission, preserve typed network faults, require canonical deletion admission, and supply reproducible endpoint and qualification instructions. Keep execution evidence separate from source completeness.

## Impact
Only optional object storage and its dedicated tests/profile sources; no new dependency or authority. No performance or NVMe claim from temporary directories.
