let g = import "./choregraph/graph-construction-v1.ncl" in
let parameters = import "./molten/parameters.ncl" in
# Compact data: one 16-operation template instantiated `count` times. Native
# expansion, not the author's generation cap, decides whether it fits.
let block = g.template "block" []
  (g.wire "body"
    (g.generate 16 (std.array.map std.to_string (std.array.range 0 16))
      (fun key => g.operation "n%{key}" "source-u64" { values = g.literal [] } []))
    [] [] []) in
g.finish "bulk" "molten.job-dag" "selected-sequence-v2" {} [block]
  (g.wire "root"
    (g.generate 4096 (std.array.map std.to_string (std.array.range 0 parameters.count))
      (fun key => g.instantiate "i%{key}" "block" {}))
    [] [] [])
