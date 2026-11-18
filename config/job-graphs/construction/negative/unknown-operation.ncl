let g = import "./choregraph/graph-construction-v1.ncl" in
g.finish "bad" "molten.job-dag" "selected-sequence-v2" {} []
  (g.wire "root" [g.operation "emit" "source-i64" { values = g.literal [1] } []] [] [] [])
