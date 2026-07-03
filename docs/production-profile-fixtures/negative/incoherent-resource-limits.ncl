(import "../profile-template.ncl").make {
  resource_limits = {
    max_queue_depth = 1024,
    max_receipt_bytes = 134217728,
    max_store_bytes = 1024,
    max_delivery_latency_ms = 60000,
    max_recovery_time_ms = 5000,
  },
}
