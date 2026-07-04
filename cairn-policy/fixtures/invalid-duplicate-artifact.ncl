let Contracts = import "../contracts.ncl" in
(import "../default.ncl") & {
  schemas | force = [{
    id = "duplicate-artifact-schema",
    description = "duplicate artifact id fixture",
    artifacts = [
      { id = "proposal", generates = "proposal.md", requires = [] },
      { id = "proposal", generates = "proposal-copy.md", requires = [] },
    ],
  }],
} | Contracts.Policy
