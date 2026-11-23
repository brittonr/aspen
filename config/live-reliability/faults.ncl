let Contracts = import "contracts.ncl" in
# Select one profile against the exact register endpoints. This does not grant a
# provider permission or supply an effect reference for a disruptive run.
{
  no_fault = fun endpoints bounds =>
    (bounds & { fault = { kind = "no-fault" } }) | Contracts.NoFault endpoints,
  process_restart = fun endpoints bounds restart =>
    (bounds & { fault = restart & { kind = "process-restart" } }) | Contracts.ProcessRestart endpoints,
  temporary_partition = fun endpoints bounds partition =>
    (bounds & { fault = partition & { kind = "temporary-partition" } }) | Contracts.TemporaryPartition endpoints,
}
