let g = import "./choregraph/graph-construction-v1.ncl" in
# Each source is wrapped without exposures, so the sequence exchanges no data.
# The equal-length keys allocate identities opposite to execution order; only the
# Unit dependency orders them.
let hidden = fun key operation => g.wire key [operation] [] [] [] in
g.finish "ordered" "molten.job-dag" "selected-sequence-v2" {} []
  (g.sequence "root" [
    hidden "z_first" (g.operation "emit" "source-u64" { values = g.literal [1] } []),
    hidden "a_later" (g.operation "emit" "source-u64" { values = g.literal [2] } []),
  ])
