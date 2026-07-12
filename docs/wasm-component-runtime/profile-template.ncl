let Contracts = import "contracts.ncl" in
let get = fun field fallback overrides => std.record.get_or field fallback overrides in
{
  make = fun args =>
    {
      schema_id = get "schema_id" Contracts.schema_id args,
      schema_version = get "schema_version" Contracts.schema_version args,
      source_language = get "source_language" Contracts.source_language args,
      profile = {
        profile_id = get "profile_id" Contracts.profile_id args,
        evidence_scope = get "evidence_scope" Contracts.evidence_scope args,
        runtime_strategy = get "runtime_strategy" Contracts.runtime_strategy args,
        toolchain = {
          wasmtime = get "wasmtime" Contracts.toolchain.wasmtime args,
          wasm_tools = get "wasm_tools" Contracts.toolchain.wasm_tools args,
          wasmparser = get "wasmparser" Contracts.toolchain.wasmparser args,
          wit_bindgen = get "wit_bindgen" Contracts.toolchain.wit_bindgen args,
          wasmtime_wasi = get "wasmtime_wasi" Contracts.toolchain.wasmtime_wasi args,
          wasi_package = get "wasi_package" Contracts.toolchain.wasi_package args,
        },
        wit = {
          package = get "wit_package" Contracts.wit.package args,
          world = get "wit_world" Contracts.wit.world args,
          source_ref = get "wit_source_ref" Contracts.wit.source_ref args,
        },
        features = {
          component_model = get "component_model" true args,
          multi_value = get "multi_value" true args,
          bulk_memory = get "bulk_memory" true args,
          reference_types = get "reference_types" true args,
          simd = get "simd" true args,
          relaxed_simd = get "relaxed_simd" false args,
          threads = get "threads" false args,
          tail_call = get "tail_call" false args,
          multi_memory = get "multi_memory" false args,
          exceptions = get "exceptions" false args,
          gc = get "gc" false args,
          memory64 = get "memory64" false args,
          extended_const = get "extended_const" false args,
          function_references = get "function_references" false args,
          custom_page_sizes = get "custom_page_sizes" false args,
          wide_arithmetic = get "wide_arithmetic" false args,
          component_async = get "component_async" false args,
        },
        determinism = {
          fuel_interruption = get "fuel_interruption" true args,
          nan_canonicalization = get "nan_canonicalization" true args,
          relaxed_simd_deterministic = get "relaxed_simd_deterministic" true args,
          memory_growth = get "memory_growth" "fixed" args,
          table_growth = get "table_growth" "fixed" args,
          host_inputs = get "host_inputs" "recorded-effects-only" args,
        },
        resources = get "resources" Contracts.resources args,
        allowed_imports = get "allowed_imports" [] args,
        allowed_wasi_interfaces = get "allowed_wasi_interfaces" [] args,
        non_claims = get "non_claims" Contracts.non_claims args,
      },
    } | Contracts.Export,
}
