(import "../profile-template.ncl").make {
  state_layout = {
    state_root = "/var/lib/molten",
    ledger_dir = "../ledger",
    redb_dir = "redb",
    chunk_dir = "chunks",
    identity_dir = "identity",
    retention_dir = "retention",
    inbox_dir = "inbox",
  },
}
