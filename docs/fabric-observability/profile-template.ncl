let Contracts = import "contracts.ncl" in
let get = fun field fallback args => std.record.get_or field fallback args in
let MaxDescriptors = 256 in
let MaxLabelsPerSample = 16 in
let MaxLabelNameBytes = 64 in
let MaxLabelValueBytes = 128 in
let MaxSeries = 4096 in
let MaxEvents = 1024 in
let MaxEventDetailBytes = 512 in
let MaxQueuedBytes = 1048576 in
let MaxSnapshots = 64 in
let MaxScanItems = 4096 in
let MaxFindings = 256 in
let MaxDiagnostics = 128 in
let MinExportIntervalTicks = 1 in
{
  make = fun args => {
    schema_id = Contracts.schema_id,
    schema_version = Contracts.schema_version,
    source_language = Contracts.source_language,
    adapter_classes = Contracts.adapter_classes,
    claim_scopes = Contracts.claim_scopes,
    bounds = {
      max_descriptors = get "max_descriptors" MaxDescriptors args,
      max_labels_per_sample = get "max_labels_per_sample" MaxLabelsPerSample args,
      max_label_name_bytes = get "max_label_name_bytes" MaxLabelNameBytes args,
      max_label_value_bytes = get "max_label_value_bytes" MaxLabelValueBytes args,
      max_series = get "max_series" MaxSeries args,
      max_events = get "max_events" MaxEvents args,
      max_event_detail_bytes = get "max_event_detail_bytes" MaxEventDetailBytes args,
      max_queued_bytes = get "max_queued_bytes" MaxQueuedBytes args,
      max_snapshots = get "max_snapshots" MaxSnapshots args,
      max_scan_items = get "max_scan_items" MaxScanItems args,
      max_findings = get "max_findings" MaxFindings args,
      max_diagnostics = get "max_diagnostics" MaxDiagnostics args,
      min_export_interval_ticks = get "min_export_interval_ticks" MinExportIntervalTicks args,
    },
    integrity = {
      read_only_by_default = get "read_only_by_default" true args,
      separate_mutation_authority_required = get "separate_mutation_authority_required" true args,
    },
    redaction = {
      deny_unclassified_sensitive_values = get "deny_unclassified_sensitive_values" true args,
      deny_credentials = true,
      deny_raw_tickets = true,
      deny_payloads = true,
      deny_private_paths = true,
    },
    scope = {
      allow_implicit_promotion = get "allow_implicit_promotion" false args,
    },
    export = {
      allow_unbounded_queue = get "allow_unbounded_queue" false args,
      terminal_failures_explicit = get "terminal_failures_explicit" true args,
    },
    non_claims = get "non_claims" Contracts.non_claims args,
  } | Contracts.Export,
}
