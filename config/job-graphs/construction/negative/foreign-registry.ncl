let g = import "./choregraph/graph-construction-v1.ncl" in
g.finish "bad" "lattice-test" "fixture-registry" {} []
  (g.wire "root" [g.operation "emit" "source-u64" { values = g.literal [1] } []] [] [] [])
