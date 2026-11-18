let g = import "./choregraph/graph-construction-v1.ncl" in
g.finish "single" "molten.job-dag" "selected-sequence-v2" {} []
  (g.wire "root" [g.operation "emit" "source-u64" { values = g.literal [1, 2] } []] [] [] [])
