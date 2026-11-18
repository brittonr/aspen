let g = import "./choregraph/graph-construction-v1.ncl" in
{
  emit = g.template "emit" ["values"]
    (g.operation "body" "source-u64" { values = g.parameter "values" } []),
}
