let g = import "./choregraph/graph-construction-v1.ncl" in
g.finish "bad" "molten.job-dag" "selected-sequence-v2" {} []
  (g.wire "root" [
    g.operation "a" "identity-u64" {} [],
    g.operation "b" "identity-u64" {} [],
  ] [
    g.connect (g.output "bad" "a" "out") (g.input "bad" "b" "in"),
    g.connect (g.output "bad" "b" "out") (g.input "bad" "a" "in"),
  ] [] [])
