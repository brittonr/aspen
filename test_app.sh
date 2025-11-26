#!/usr/bin/env sh
# Comprehensive integration test for Blixard application
# Tests startup, health endpoints, and job queue functionality

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
API_KEY="test_api_key_for_integration_testing_min_32_characters"
HTTP_PORT=${HTTP_PORT:-3020}
BASE_URL="http://localhost:${HTTP_PORT}"
STARTUP_TIMEOUT=30
APP_PID=""
LOG_FILE="test_app.log"

# Cleanup function
cleanup() {
    echo "\n${BLUE}Cleaning up...${NC}"
    if [ -n "$APP_PID" ]; then
        echo "Stopping application (PID: $APP_PID)"
        kill $APP_PID 2>/dev/null || true
        wait $APP_PID 2>/dev/null || true
    fi
    if [ -n "$TEST_DATA_DIR" ] && [ -d "$TEST_DATA_DIR" ]; then
        echo "Cleaning test data directory: $TEST_DATA_DIR"
        rm -rf "$TEST_DATA_DIR"
    fi
    if [ -f "$LOG_FILE" ]; then
        echo "Log file preserved at: $LOG_FILE"
    fi
}

trap cleanup EXIT INT TERM

# Print section header
print_section() {
    echo "\n${BLUE}========================================${NC}"
    echo "${BLUE}$1${NC}"
    echo "${BLUE}========================================${NC}"
}

# Print test result
print_test() {
    if [ $1 -eq 0 ]; then
        echo "${GREEN}✓${NC} $2"
    else
        echo "${RED}✗${NC} $2"
    fi
}

# Wait for HTTP endpoint to be available
wait_for_ready() {
    local timeout=$1
    local elapsed=0

    echo "Waiting for application to be ready (timeout: ${timeout}s)..."

    while [ $elapsed -lt $timeout ]; do
        if curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/health/hiqlite" | grep -q "^[2-5][0-9][0-9]$"; then
            echo "${GREEN}Application is ready!${NC} (${elapsed}s)"
            return 0
        fi
        sleep 1
        elapsed=$((elapsed + 1))

        # Show progress every 5 seconds
        if [ $((elapsed % 5)) -eq 0 ]; then
            echo "  Still waiting... (${elapsed}s)"
        fi
    done

    echo "${RED}Timeout waiting for application to start${NC}"
    return 1
}

# Make API request with proper headers
api_request() {
    local method=$1
    local path=$2
    local data=$3

    if [ -n "$data" ]; then
        curl -s -X "$method" \
            -H "X-API-Key: $API_KEY" \
            -H "Content-Type: application/json" \
            -d "$data" \
            "$BASE_URL$path"
    else
        curl -s -X "$method" \
            -H "X-API-Key: $API_KEY" \
            "$BASE_URL$path"
    fi
}

# Test health endpoint (no auth required)
test_health() {
    local response
    local http_code

    response=$(curl -s -w "\n%{http_code}" "$BASE_URL/health/hiqlite")
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)

    if [ "$http_code" = "200" ]; then
        print_test 0 "Health endpoint returned 200"
        echo "  Response: $body"
        return 0
    else
        print_test 1 "Health endpoint returned $http_code"
        echo "  Response: $body"
        return 1
    fi
}

# Test unauthorized access
test_unauthorized() {
    local http_code

    http_code=$(curl -s -o /dev/null -w "%{http_code}" \
        -X POST \
        -H "Content-Type: application/json" \
        -d '{"test": "data"}' \
        "$BASE_URL/api/queue/publish")

    if [ "$http_code" = "401" ]; then
        print_test 0 "Unauthorized request correctly rejected (401)"
        return 0
    else
        print_test 1 "Expected 401, got $http_code"
        return 1
    fi
}

# Test job submission with generic payload
test_job_submission() {
    local payload=$1
    local description=$2
    local response
    local http_code

    response=$(api_request POST "/api/queue/publish" "$payload" | tee /dev/tty)
    http_code=$(curl -s -o /dev/null -w "%{http_code}" \
        -X POST \
        -H "X-API-Key: $API_KEY" \
        -H "Content-Type: application/json" \
        -d "$payload" \
        "$BASE_URL/api/queue/publish")

    if [ "$http_code" = "200" ]; then
        print_test 0 "Job submission: $description"
        echo "  Response: $response"
        return 0
    else
        print_test 1 "Job submission failed: $description (HTTP $http_code)"
        echo "  Response: $response"
        return 1
    fi
}

# Test queue listing
test_queue_list() {
    local response
    local http_code

    response=$(api_request GET "/api/queue/list")
    http_code=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "X-API-Key: $API_KEY" \
        "$BASE_URL/api/queue/list")

    if [ "$http_code" = "200" ]; then
        print_test 0 "Queue list endpoint"
        echo "  Jobs in queue: $(echo "$response" | grep -o '"id"' | wc -l)"
        return 0
    else
        print_test 1 "Queue list failed (HTTP $http_code)"
        return 1
    fi
}

# Test queue stats
test_queue_stats() {
    local response
    local http_code

    response=$(api_request GET "/api/queue/stats")
    http_code=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "X-API-Key: $API_KEY" \
        "$BASE_URL/api/queue/stats")

    if [ "$http_code" = "200" ]; then
        print_test 0 "Queue stats endpoint"
        echo "  Stats: $response"
        return 0
    else
        print_test 1 "Queue stats failed (HTTP $http_code)"
        return 1
    fi
}

# Main test execution
main() {
    print_section "Blixard Integration Test Suite"

    echo "Configuration:"
    echo "  Base URL: $BASE_URL"
    echo "  API Key: ${API_KEY:0:10}..."
    echo "  Log file: $LOG_FILE"

    # Check if port is already in use
    if netstat -tuln 2>/dev/null | grep -q ":${HTTP_PORT} " || \
       ss -tuln 2>/dev/null | grep -q ":${HTTP_PORT} "; then
        echo "${RED}Error: Port $HTTP_PORT is already in use${NC}"
        echo "Please stop the existing service or change HTTP_PORT"
        exit 1
    fi

    print_section "Starting Application"

    # Create unique test data directory
    TEST_DATA_DIR="/tmp/blixard_test_$$"
    mkdir -p "$TEST_DATA_DIR"

    # Set environment variables
    export SKIP_FLAWLESS=1
    export SKIP_VM_MANAGER=1
    export BLIXARD_API_KEY="$API_KEY"
    export HTTP_PORT="$HTTP_PORT"

    # Hiqlite uses HQL_ prefix, not HIQLITE_
    export HQL_DATA_DIR="$TEST_DATA_DIR/hiqlite"
    export HQL_NODE_ID="1"

    # Generate test secrets if not present (use HQL_ prefix)
    if [ -z "$HQL_SECRET_RAFT" ]; then
        export HQL_SECRET_RAFT="test_raft_secret_for_integration_testing_min_32_chars"
    fi
    if [ -z "$HQL_SECRET_API" ]; then
        export HQL_SECRET_API="test_api_secret_for_integration_testing_min_32_chars"
    fi
    if [ -z "$HQL_ENC_KEY" ]; then
        export HQL_ENC_KEY="dGVzdF9lbmNyeXB0aW9uX2tleV9mb3JfaW50ZWdyYXRpb25fdGVzdGluZw=="
    fi

    echo "Environment:"
    echo "  SKIP_FLAWLESS=$SKIP_FLAWLESS"
    echo "  SKIP_VM_MANAGER=$SKIP_VM_MANAGER"
    echo "  HTTP_PORT=$HTTP_PORT"
    echo "  HQL_DATA_DIR=$HQL_DATA_DIR"

    # Start the application in the background
    echo "Starting mvm-ci server..."
    nix develop -c cargo run --bin mvm-ci > "$LOG_FILE" 2>&1 &
    APP_PID=$!

    echo "Application PID: $APP_PID"

    # Wait for the application to be ready
    if ! wait_for_ready $STARTUP_TIMEOUT; then
        echo "${RED}Failed to start application${NC}"
        echo "\nLast 50 lines of log file:"
        tail -n 50 "$LOG_FILE"
        exit 1
    fi

    print_section "Running Tests"

    # Initialize test counters
    tests_passed=0
    tests_failed=0

    # Test 1: Health endpoint (no auth required)
    echo "\n${YELLOW}Test 1: Health Check${NC}"
    if test_health; then
        tests_passed=$((tests_passed + 1))
    else
        tests_failed=$((tests_failed + 1))
    fi

    # Test 2: Unauthorized access
    echo "\n${YELLOW}Test 2: Authentication${NC}"
    if test_unauthorized; then
        tests_passed=$((tests_passed + 1))
    else
        tests_failed=$((tests_failed + 1))
    fi

    # Test 3: Generic JSON payload
    echo "\n${YELLOW}Test 3: Generic JSON Payload${NC}"
    if test_job_submission '{"task": "process_data", "input": {"value": 42}}' "Generic task"; then
        tests_passed=$((tests_passed + 1))
    else
        tests_failed=$((tests_failed + 1))
    fi

    # Test 4: Complex nested payload
    echo "\n${YELLOW}Test 4: Complex Nested Payload${NC}"
    if test_job_submission '{"workflow": "etl", "steps": [{"name": "extract", "source": "db"}, {"name": "transform", "rules": ["clean", "normalize"]}, {"name": "load", "destination": "warehouse"}]}' "Complex ETL workflow"; then
        tests_passed=$((tests_passed + 1))
    else
        tests_failed=$((tests_failed + 1))
    fi

    # Test 5: Simple string payload
    echo "\n${YELLOW}Test 5: Simple String Payload${NC}"
    if test_job_submission '{"command": "echo Hello World"}' "Simple command"; then
        tests_passed=$((tests_passed + 1))
    else
        tests_failed=$((tests_failed + 1))
    fi

    # Test 6: Array payload
    echo "\n${YELLOW}Test 6: Array-based Payload${NC}"
    if test_job_submission '{"items": [1, 2, 3, 4, 5], "operation": "sum"}' "Array processing"; then
        tests_passed=$((tests_passed + 1))
    else
        tests_failed=$((tests_failed + 1))
    fi

    # Test 7: URL-based payload (backward compatibility)
    echo "\n${YELLOW}Test 7: URL-based Payload (Legacy)${NC}"
    if test_job_submission '{"url": "https://example.com/data", "method": "GET"}' "HTTP request job"; then
        tests_passed=$((tests_passed + 1))
    else
        tests_failed=$((tests_failed + 1))
    fi

    # Test 8: Queue list
    echo "\n${YELLOW}Test 8: Queue List${NC}"
    if test_queue_list; then
        tests_passed=$((tests_passed + 1))
    else
        tests_failed=$((tests_failed + 1))
    fi

    # Test 9: Queue stats
    echo "\n${YELLOW}Test 9: Queue Statistics${NC}"
    if test_queue_stats; then
        tests_passed=$((tests_passed + 1))
    else
        tests_failed=$((tests_failed + 1))
    fi

    # Test 10: Null payload rejection
    echo "\n${YELLOW}Test 10: Null Payload Rejection${NC}"
    response=$(api_request POST "/api/queue/publish" 'null')
    http_code=$(curl -s -o /dev/null -w "%{http_code}" \
        -X POST \
        -H "X-API-Key: $API_KEY" \
        -H "Content-Type: application/json" \
        -d 'null' \
        "$BASE_URL/api/queue/publish")

    if [ "$http_code" = "400" ] || [ "$http_code" = "500" ]; then
        print_test 0 "Null payload correctly rejected"
        tests_passed=$((tests_passed + 1))
    else
        print_test 1 "Null payload not rejected (HTTP $http_code)"
        tests_failed=$((tests_failed + 1))
    fi

    print_section "Test Summary"

    total_tests=$((tests_passed + tests_failed))
    echo "Total tests: $total_tests"
    echo "${GREEN}Passed: $tests_passed${NC}"
    echo "${RED}Failed: $tests_failed${NC}"

    if [ $tests_failed -eq 0 ]; then
        echo "\n${GREEN}All tests passed! ✓${NC}"
        exit 0
    else
        echo "\n${RED}Some tests failed ✗${NC}"
        echo "\nCheck the log file for details: $LOG_FILE"
        exit 1
    fi
}

# Run main function
main "$@"
