// The node imports and reads canonical artifacts using the shared ledger
// storage implementation; retention-controlled GC remains a root-only client.
include!("../../../src/ledger/parts/mod/p000/body.rs");
include!("../../../src/ledger/parts/mod/p001/body.rs");
