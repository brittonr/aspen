# r[verify molten.prod_release.pilot_evidence_publication]
let Contracts = import "../../molten-0.1.0-pilot-contracts.ncl" in
let Base = import "../../molten-0.1.0-pilot.ncl" in
(Base & {
  evidence.source | force = "blake3:48f801b1aad953111afc15fb935e55d2b6e4224faf72e563b2089533ee9ddde2",
}) | Contracts.Manifest
