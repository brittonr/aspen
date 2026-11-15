fun modules => fun parameters => {
  profile = "selected-sequence-v2",
  instances = std.array.map
    (fun item => { id = item.key, operation = "source-u64", values = item.payload })
    parameters.instances,
  connections = [],
  outputs = std.array.map (fun item => item.key) parameters.instances,
  origins = [],
}
