fun modules => fun parameters =>
let source = { id = "source", operation = "source-u64", values = parameters.values } in
let stage = fun instance_key => fun stage_operation => { id = instance_key, operation = stage_operation, values = [] } in
let wire = fun producer => fun output_name => fun consumer => { from = { instance = producer, port = output_name }, to = { instance = consumer, port = "in" } } in
{
  profile = "selected-sequence-v2",
  instances = [source, stage "split" "partition-even-u64", stage "left" "identity-u64", stage "right" "identity-u64", stage "join" "concat-u64"],
  connections = [wire "source" "out" "split", wire "split" "left" "left", wire "split" "right" "right", wire "left" "out" "join", wire "right" "out" "join"],
  outputs = ["join"], origins = [],
}
