let g = import "./choregraph/graph-construction-v1.ncl" in
let parameters = import "./molten/parameters.ncl" in
g.finish "diamond" "molten.job-dag" "selected-sequence-v2" {} []
  (g.wire "root" [
    g.operation "emit" "source-u64" { values = g.literal parameters.values } [],
    g.operation "left" "identity-u64" {} [],
    g.operation "right" "identity-u64" {} [],
    g.operation "join" "sum2-u64" {} [],
  ] [
    g.connect (g.output "diamond" "emit" "out") (g.input "diamond" "left" "in"),
    g.connect (g.output "diamond" "emit" "out") (g.input "diamond" "right" "in"),
    g.connect (g.output "diamond" "left" "out") (g.input "diamond" "join" "left"),
    g.connect (g.output "diamond" "right" "out") (g.input "diamond" "join" "right"),
  ] [] [])
