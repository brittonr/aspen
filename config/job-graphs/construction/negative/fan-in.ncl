let g = import "./choregraph/graph-construction-v1.ncl" in
g.finish "bad" "molten.job-dag" "selected-sequence-v2" {} []
  (g.wire "root" [
    g.operation "first" "source-u64" { values = g.literal [1] } [],
    g.operation "second" "source-u64" { values = g.literal [2] } [],
    g.operation "joined" "concat-u64" {} [],
  ] [
    g.connect (g.output "bad" "first" "out") (g.input "bad" "joined" "in"),
    g.connect (g.output "bad" "second" "out") (g.input "bad" "joined" "in"),
  ] [] [])
