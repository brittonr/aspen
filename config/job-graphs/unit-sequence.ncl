fun modules => fun parameters => {
  profile = "selected-sequence-v2",
  instances = [{ id = "z_first", operation = "source-u64", values = [] }, { id = "a_second", operation = "source-u64", values = [7] }],
  connections = [{ from = { instance = "z_first", port = "done" }, to = { instance = "a_second", port = "after" } }],
  outputs = ["a_second"], origins = [],
}
