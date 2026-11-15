fun modules => fun parameters => {
  profile = "selected-sequence-v2",
  instances = [{ id = "source", operation = "source-u64", values = [1, 2, 3, 4] }, { id = "consumer", operation = "identity-u64", values = [] }],
  connections = [{ from = { instance = "source", port = "out" }, to = { instance = "consumer", port = "in" } }],
  outputs = ["consumer"],
  origins = [{ instance = "source", template = "inline", module = "main" }, { instance = "consumer", template = "inline", module = "main" }],
}
