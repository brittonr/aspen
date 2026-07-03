(import "../profile-template.ncl").make {
  resource_limits = {
    max_queue_depth = 0,
    max_receipt_bytes = 134217728,
    max_store_bytes = 10737418240,
    max_delivery_latency_ms = 5000,
    max_recovery_time_ms = 60000,
  },
}
