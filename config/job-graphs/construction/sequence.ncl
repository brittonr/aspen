let g = import "./choregraph/graph-construction-v1.ncl" in
let parameters = import "./molten/parameters.ncl" in
g.finish "pipeline" "molten.job-dag" "selected-sequence-v2" {} []
  (g.wire "root" [
    g.sequence "steps" [
      g.operation "emit" "source-u64" { values = g.literal parameters.values } [],
      g.operation "copy" "identity-u64" {} [],
      g.operation "total" "sum-u64" {} [],
    ],
  ] [] [] [])
