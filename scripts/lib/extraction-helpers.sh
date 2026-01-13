#!/usr/bin/env sh
# Extraction helpers for Aspen CLI output parsing
# Centralized regex patterns for consistent ID extraction across test scripts

# Extract fencing token from CLI output (human or JSON)
# Usage: token=$(extract_fencing_token "$LAST_OUTPUT")
extract_fencing_token() {
    output="$1"
    token=""
    # Human format: "Lock acquired. Fencing token: 12345" or "Registered. Fencing token: 12345"
    token=$(printf '%s' "$output" | grep -oE 'Fencing token: [0-9]+' | grep -oE '[0-9]+' | head -1 || true)
    # JSON fallback: "fencing_token": 12345
    [ -z "$token" ] && token=$(printf '%s' "$output" | grep -oE '"fencing_token":\s*[0-9]+' | grep -oE '[0-9]+' | head -1 || true)
    printf '%s' "$token"
}

# Extract receipt handle from queue dequeue output
# Usage: handle=$(extract_receipt_handle "$LAST_OUTPUT")
extract_receipt_handle() {
    output="$1"
    handle=""
    # Human format: "[id] (handle: some-handle, attempts: n)"
    handle=$(printf '%s' "$output" | grep -oE '\(handle: [^,]+' | sed 's/(handle: //' | head -1 || true)
    # JSON fallback: "receipt_handle": "value"
    [ -z "$handle" ] && handle=$(printf '%s' "$output" | grep -oE '"receipt_handle":\s*"[^"]+"' | sed 's/"receipt_handle":\s*"//' | sed 's/"$//' | head -1 || true)
    printf '%s' "$handle"
}

# Extract lease ID from lease grant output
# Usage: lease_id=$(extract_lease_id "$LAST_OUTPUT")
extract_lease_id() {
    output="$1"
    # Human format: "Lease 12345 granted with TTL 60s"
    printf '%s' "$output" | grep -oE 'Lease [0-9]+' | grep -oE '[0-9]+' | head -1 || true
}

# Extract job ID (UUID format) from job submit output
# Usage: job_id=$(extract_job_id "$LAST_OUTPUT")
extract_job_id() {
    output="$1"
    job_id=""
    # Human format: "Job submitted: uuid-with-dashes"
    job_id=$(printf '%s' "$output" | grep -oE 'Job submitted: [a-f0-9-]{36}' | grep -oE '[a-f0-9-]{36}' | head -1 || true)
    # JSON fallback: "job_id": "uuid"
    [ -z "$job_id" ] && job_id=$(printf '%s' "$output" | grep -oE '"job_id":\s*"[^"]+"' | sed 's/"job_id":\s*"//' | sed 's/"$//' | head -1 || true)
    printf '%s' "$job_id"
}

# Extract 64-char hex hash (blob, tree, commit, repo IDs)
# Usage: hash=$(extract_hash "$LAST_OUTPUT")
extract_hash() {
    output="$1"
    printf '%s' "$output" | grep -oE '[a-f0-9]{64}' | head -1 || true
}

# Extract DLQ item ID from queue dlq output
# Usage: item_id=$(extract_dlq_item_id "$LAST_OUTPUT")
extract_dlq_item_id() {
    output="$1"
    item_id=""
    # Human format: "[item_id] (handle: ..., attempts: n)"
    item_id=$(printf '%s' "$output" | grep -oE '\[[0-9]+\]' | grep -oE '[0-9]+' | head -1 || true)
    # JSON fallback: "item_id": 123
    [ -z "$item_id" ] && item_id=$(printf '%s' "$output" | grep -oE '"item_id":\s*[0-9]+' | grep -oE '[0-9]+' | head -1 || true)
    printf '%s' "$item_id"
}

# Extract service ID from service register output
# Usage: service_id=$(extract_service_id "$LAST_OUTPUT")
extract_service_id() {
    output="$1"
    # Human format: "Registered service: name (id: 12345)"
    printf '%s' "$output" | grep -oE 'id: [0-9]+' | grep -oE '[0-9]+' | head -1 || true
}

# Extract counter value from counter output
# Usage: value=$(extract_counter_value "$LAST_OUTPUT")
extract_counter_value() {
    output="$1"
    # Human format: "Counter value: 123" or just a number
    value=$(printf '%s' "$output" | grep -oE 'value: [0-9]+' | grep -oE '[0-9]+' | head -1 || true)
    [ -z "$value" ] && value=$(printf '%s' "$output" | grep -oE '^[0-9]+$' | head -1 || true)
    printf '%s' "$value"
}

# Extract sequence value from sequence output
# Usage: seq=$(extract_sequence_value "$LAST_OUTPUT")
extract_sequence_value() {
    output="$1"
    # Human format: "Next value: 123" or "Sequence: 123"
    value=$(printf '%s' "$output" | grep -oE '[Nn]ext value: [0-9]+' | grep -oE '[0-9]+' | head -1 || true)
    [ -z "$value" ] && value=$(printf '%s' "$output" | grep -oE '[Ss]equence: [0-9]+' | grep -oE '[0-9]+' | head -1 || true)
    printf '%s' "$value"
}
