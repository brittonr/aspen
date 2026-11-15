fun modules => fun parameters => {
  profile = "selected-sequence-v2",
  instances = [modules.source "first" parameters.first, modules.source "second" parameters.second, { id = "join", operation = "concat-u64", values = [] }],
  connections = [{ from = { instance = "first", port = "out" }, to = { instance = "join", port = "in" } }, { from = { instance = "second", port = "out" }, to = { instance = "join", port = "in" } }],
  outputs = ["join"],
  origins = [{ instance = "first", template = "source", module = "source" }, { instance = "second", template = "source", module = "source" }],
}
