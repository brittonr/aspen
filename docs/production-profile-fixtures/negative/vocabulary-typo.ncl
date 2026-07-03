(import "../profile-template.ncl").make {
  required_adapters = [
    "node-control",
    "evidence-ledger",
    "chunk-store",
    "retention-index",
    "live-iroh-typo",
  ],
}
