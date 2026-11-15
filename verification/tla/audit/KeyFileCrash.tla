---- MODULE KeyFileCrash ----
\* Bounded TLA+ model of crash consistency for node and production key
\* rotation writes (issue #46).
\* It is a model, not a filesystem proof or a production profile.
\*
\* Source mapping (brittonr/aspen at 8ae93a5e5):
\*   Mode "inplace"  crates/molten-node-host/src/node/state/filesystem.rs write_regular_file
\*                   (OpenOptions create + truncate, write_all, flush; no sync_all, no rename),
\*                   reached from src/fabric_crypto_identity/file/parts/adapter/p002/body.rs rotation.
\*   Mode "atomic"   the suggested fix, matching crates/molten-node-host/src/node/state/publication.rs
\*                   (create_new + write + sync_all) plus rename and a parent-directory sync.
\*
\* Filesystem abstraction: the page cache holds what the process sees; disk holds
\* what survives a crash. Writeback may persist any pending change at any time
\* (data blocks and metadata independently, as on ext4/xfs without data journaling).
\* fsync forces the file's data to disk; a directory fsync forces its entries.
\* A crash replaces the cache with the disk image.
EXTENDS TLC

CONSTANTS Mode

ASSUME Mode \in {"inplace", "atomic"}

Contents == {"old", "empty", "partial", "new"}

VARIABLES
    pc,            \* program counter of the rotation
    keyCache,      \* contents seen through the key path (in-place inode)
    keyDisk,       \* durable contents of the in-place inode
    tmpCache,      \* contents of the temporary inode
    tmpDisk,
    nameCache,     \* which inode the key path names: "orig" or "tmp"
    nameDisk,
    reported       \* rotation returned Ok to its caller

vars == <<pc, keyCache, keyDisk, tmpCache, tmpDisk, nameCache, nameDisk, reported>>

PCs == {"start", "truncated", "written", "flushed",
        "tmpWritten", "tmpSynced", "renamed", "dirSynced", "done", "crashed"}

TypeOK ==
    /\ pc \in PCs
    /\ keyCache \in Contents /\ keyDisk \in Contents
    /\ tmpCache \in Contents \cup {"none"} /\ tmpDisk \in Contents \cup {"none"}
    /\ nameCache \in {"orig", "tmp"} /\ nameDisk \in {"orig", "tmp"}
    /\ reported \in BOOLEAN

Init ==
    /\ pc = "start"
    /\ keyCache = "old" /\ keyDisk = "old"
    /\ tmpCache = "none" /\ tmpDisk = "none"
    /\ nameCache = "orig" /\ nameDisk = "orig"
    /\ reported = FALSE

\* ---- current code: write_regular_file ----
Truncate ==
    /\ Mode = "inplace" /\ pc = "start"
    /\ pc' = "truncated" /\ keyCache' = "empty"
    /\ UNCHANGED <<keyDisk, tmpCache, tmpDisk, nameCache, nameDisk, reported>>

WriteAll ==
    /\ Mode = "inplace" /\ pc = "truncated"
    /\ pc' = "written" /\ keyCache' = "new"
    /\ UNCHANGED <<keyDisk, tmpCache, tmpDisk, nameCache, nameDisk, reported>>

\* File::flush is a no-op for std/cap-std files: nothing is forced to disk.
Flush ==
    /\ Mode = "inplace" /\ pc = "written"
    /\ pc' = "flushed"
    /\ UNCHANGED <<keyCache, keyDisk, tmpCache, tmpDisk, nameCache, nameDisk, reported>>

ReturnInPlace ==
    /\ Mode = "inplace" /\ pc = "flushed"
    /\ pc' = "done" /\ reported' = TRUE
    /\ UNCHANGED <<keyCache, keyDisk, tmpCache, tmpDisk, nameCache, nameDisk>>

\* ---- suggested fix: temp file, sync, rename, directory sync ----
WriteTmp ==
    /\ Mode = "atomic" /\ pc = "start"
    /\ pc' = "tmpWritten" /\ tmpCache' = "new"
    /\ UNCHANGED <<keyCache, keyDisk, tmpDisk, nameCache, nameDisk, reported>>

SyncTmp ==
    /\ Mode = "atomic" /\ pc = "tmpWritten"
    /\ pc' = "tmpSynced" /\ tmpDisk' = tmpCache
    /\ UNCHANGED <<keyCache, keyDisk, tmpCache, nameCache, nameDisk, reported>>

Rename ==
    /\ Mode = "atomic" /\ pc = "tmpSynced"
    /\ pc' = "renamed" /\ nameCache' = "tmp"
    /\ UNCHANGED <<keyCache, keyDisk, tmpCache, tmpDisk, nameDisk, reported>>

SyncDir ==
    /\ Mode = "atomic" /\ pc = "renamed"
    /\ pc' = "dirSynced" /\ nameDisk' = nameCache
    /\ UNCHANGED <<keyCache, keyDisk, tmpCache, tmpDisk, nameCache, reported>>

ReturnAtomic ==
    /\ Mode = "atomic" /\ pc = "dirSynced"
    /\ pc' = "done" /\ reported' = TRUE
    /\ UNCHANGED <<keyCache, keyDisk, tmpCache, tmpDisk, nameCache, nameDisk>>

\* ---- environment ----
\* Background writeback of the in-place inode. A multi-block write may land partially.
WritebackKey ==
    /\ pc # "crashed"
    /\ keyDisk # keyCache
    /\ keyDisk' \in (IF keyCache = "new" THEN {"partial", "new"} ELSE {keyCache})
    /\ UNCHANGED <<pc, keyCache, tmpCache, tmpDisk, nameCache, nameDisk, reported>>

WritebackTmp ==
    /\ pc # "crashed"
    /\ tmpDisk # tmpCache
    /\ tmpDisk' \in (IF tmpCache = "new" THEN {"partial", "new"} ELSE {tmpCache})
    /\ UNCHANGED <<pc, keyCache, keyDisk, tmpCache, nameCache, nameDisk, reported>>

WritebackDir ==
    /\ pc # "crashed"
    /\ nameDisk # nameCache
    /\ nameDisk' = nameCache
    /\ UNCHANGED <<pc, keyCache, keyDisk, tmpCache, tmpDisk, nameCache, reported>>

Crash ==
    /\ pc # "crashed"
    /\ pc' = "crashed"
    /\ keyCache' = keyDisk /\ tmpCache' = tmpDisk /\ nameCache' = nameDisk
    /\ UNCHANGED <<keyDisk, tmpDisk, nameDisk, reported>>

Next ==
    \/ Truncate \/ WriteAll \/ Flush \/ ReturnInPlace
    \/ WriteTmp \/ SyncTmp \/ Rename \/ SyncDir \/ ReturnAtomic
    \/ WritebackKey \/ WritebackTmp \/ WritebackDir \/ Crash

Spec == Init /\ [][Next]_vars

\* What a restarted node reads through the key path.
KeyAfterRestart == IF nameDisk = "tmp" THEN tmpDisk ELSE keyDisk

\* After any crash a whole key (old or new) is readable.
WholeKeyAfterCrash == pc = "crashed" => KeyAfterRestart \in {"old", "new"}

\* A rotation that returned Ok survives a later crash.
ReportedRotationDurable == (pc = "crashed" /\ reported) => KeyAfterRestart = "new"
====
