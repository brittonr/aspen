let Base = import "../../source.ncl" in
let Contracts = import "../../source-contracts.ncl" in
let DriftedNotices = [
  {
    path = "APACHE_LICENSE",
    license = "Apache-2.0",
    content_ref = "blake3:0000000000000000000000000000000000000000000000000000000000000000",
  },
  {
    path = "LICENSE.md",
    license = "SQLite-public-domain-and-project-notices",
    content_ref = "blake3:4f91d1a7d7b99eefb5c81ddb148446616d8260fc0c8113999cf2a48d3589267c",
  },
]
in
(Base & { notices | force = DriftedNotices }) | Contracts.Source
