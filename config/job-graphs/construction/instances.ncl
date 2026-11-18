let g = import "./choregraph/graph-construction-v1.ncl" in
let parameters = import "./molten/parameters.ncl" in
let stages = import "./stages.ncl" in
g.finish "instances" "molten.job-dag" "selected-sequence-v2" {} [stages.emit]
  (g.wire "root" [
    g.instantiate "first" "emit" { values = g.literal parameters.first },
    g.instantiate "second" "emit" { values = g.literal parameters.second },
    g.operation "join" "sum2-u64" {} [],
  ] [
    g.connect (g.output "instances" "first" "out") (g.input "instances" "join" "left"),
    g.connect (g.output "instances" "second" "out") (g.input "instances" "join" "right"),
  ] [] [])
