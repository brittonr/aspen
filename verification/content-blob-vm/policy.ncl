{
  schema | String | doc "Fixture policy schema; not production authority." = "molten.content-blob-vm-policy.v1",
  max_archive_bytes | Number | doc "Maximum public archive bytes accepted by the VM fixture." = 1048576,
  chunk_bytes | Number | doc "Fixed Molten chunk size for this fixture." = 65536,
  max_connections | Number | doc "Maximum concurrent authorized reader connections." = 2,
  service_seconds | Number | doc "Hard fixture service lifetime inside a VM." = 300,
  read_seconds | Number | doc "Maximum wait for each bounded Iroh read." = 10,
}
