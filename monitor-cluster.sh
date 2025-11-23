#!/usr/bin/env bash
# Interactive cluster monitoring tool

CONTROL_PLANE="http://localhost:3020"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Terminal control
CURSOR_HOME='\033[H'     # Move cursor to home position
CLEAR_SCREEN='\033[2J'   # Clear entire screen
CLEAR_LINE='\033[K'      # Clear from cursor to end of line
SAVE_CURSOR='\033[s'     # Save cursor position
RESTORE_CURSOR='\033[u'  # Restore cursor position
HIDE_CURSOR='\033[?25l'  # Hide cursor
SHOW_CURSOR='\033[?25h'  # Show cursor

# Initialize screen
init_screen() {
    echo -ne "${HIDE_CURSOR}${CLEAR_SCREEN}${CURSOR_HOME}"
}

# Update screen in place (no flicker)
update_screen() {
    # Move cursor to home without clearing
    echo -ne "${CURSOR_HOME}"
}

show_header() {
    echo -e "${BOLD}${BLUE}=== MVM-CI Cluster Monitor ===${NC}${CLEAR_LINE}"
    echo -e "Time: $(date '+%Y-%m-%d %H:%M:%S')${CLEAR_LINE}"
    echo -e "${CLEAR_LINE}"
}

show_cluster_status() {
    echo -e "${BLUE}--- Cluster Health ---${NC}${CLEAR_LINE}"

    # Control plane health
    HEALTH=$(curl -s "${CONTROL_PLANE}/hiqlite/health" 2>/dev/null | jq -r '.is_healthy' 2>/dev/null || echo "error")
    if [ "$HEALTH" = "true" ]; then
        echo -e "Control Plane: ${GREEN}✓ Healthy${NC}${CLEAR_LINE}"
    else
        echo -e "Control Plane: ${RED}✗ Unhealthy${NC}${CLEAR_LINE}"
    fi

    # Worker status
    WORKER_COUNT=$(docker ps --filter "name=worker" --filter "status=running" --format "{{.Names}}" 2>/dev/null | wc -l)
    echo -e "Workers Running: ${GREEN}${WORKER_COUNT}${NC}${CLEAR_LINE}"

    echo -e "${CLEAR_LINE}"
}

show_queue_stats() {
    echo -e "${BLUE}--- Queue Statistics ---${NC}${CLEAR_LINE}"

    STATS=$(curl -s "${CONTROL_PLANE}/queue/stats" 2>/dev/null)
    if [ $? -eq 0 ]; then
        PENDING=$(echo "$STATS" | jq -r '.pending' 2>/dev/null || echo "?")
        CLAIMED=$(echo "$STATS" | jq -r '.claimed' 2>/dev/null || echo "?")
        IN_PROGRESS=$(echo "$STATS" | jq -r '.in_progress' 2>/dev/null || echo "?")
        COMPLETED=$(echo "$STATS" | jq -r '.completed' 2>/dev/null || echo "?")
        FAILED=$(echo "$STATS" | jq -r '.failed' 2>/dev/null || echo "?")

        echo -e "  Pending:     ${YELLOW}${PENDING}${NC}${CLEAR_LINE}"
        echo -e "  Claimed:     ${BLUE}${CLAIMED}${NC}${CLEAR_LINE}"
        echo -e "  In Progress: ${BLUE}${IN_PROGRESS}${NC}${CLEAR_LINE}"
        echo -e "  Completed:   ${GREEN}${COMPLETED}${NC}${CLEAR_LINE}"
        echo -e "  Failed:      ${RED}${FAILED}${NC}${CLEAR_LINE}"
    else
        echo -e "${RED}Unable to fetch queue stats${NC}${CLEAR_LINE}"
    fi

    echo -e "${CLEAR_LINE}"
}

show_recent_jobs() {
    echo -e "${BLUE}--- Recent Jobs (last 10) ---${NC}${CLEAR_LINE}"

    JOBS=$(curl -s "${CONTROL_PLANE}/queue/list" 2>/dev/null)
    if [ $? -eq 0 ]; then
        local count=0
        while IFS= read -r line && [ $count -lt 10 ]; do
            if [[ "$line" =~ "Completed" ]]; then
                echo -e "  ${GREEN}${line}${NC}${CLEAR_LINE}"
            elif [[ "$line" =~ "Failed" ]]; then
                echo -e "  ${RED}${line}${NC}${CLEAR_LINE}"
            elif [[ "$line" =~ "InProgress" ]]; then
                echo -e "  ${BLUE}${line}${NC}${CLEAR_LINE}"
            else
                echo -e "  ${YELLOW}${line}${NC}${CLEAR_LINE}"
            fi
            ((count++))
        done < <(echo "$JOBS" | jq -r '.[] | "\(.job_id): \(.status) (\(.claimed_by[:10] // "none"))"' 2>/dev/null | tail -10)

        # Clear any remaining job lines from previous display
        while [ $count -lt 10 ]; do
            echo -e "${CLEAR_LINE}"
            ((count++))
        done
    else
        echo -e "${RED}Unable to fetch job list${NC}${CLEAR_LINE}"
        # Clear remaining lines
        for i in {1..9}; do echo -e "${CLEAR_LINE}"; done
    fi

    echo -e "${CLEAR_LINE}"
}

show_worker_activity() {
    echo -e "${BLUE}--- Worker Activity ---${NC}${CLEAR_LINE}"

    for worker in mvm-ci-worker1 mvm-ci-worker2; do
        if docker ps --filter "name=${worker}" --filter "status=running" --format "{{.Names}}" 2>/dev/null | grep -q "${worker}"; then
            LAST_LOG=$(docker logs "${worker}" 2>&1 | grep -E "(Claimed job|Job completed|No work available)" | tail -1)
            echo -e "  ${worker}:${CLEAR_LINE}"
            if [[ "$LAST_LOG" =~ "Claimed job" ]]; then
                echo -e "    ${BLUE}${LAST_LOG}${NC}${CLEAR_LINE}"
            elif [[ "$LAST_LOG" =~ "completed" ]]; then
                echo -e "    ${GREEN}${LAST_LOG}${NC}${CLEAR_LINE}"
            else
                echo -e "    ${LAST_LOG}${CLEAR_LINE}"
            fi
        else
            echo -e "  ${worker}: ${RED}Not running${NC}${CLEAR_LINE}"
            echo -e "${CLEAR_LINE}"
        fi
    done

    echo -e "${CLEAR_LINE}"
}

show_help() {
    echo -e "${BLUE}--- Commands ---${NC}${CLEAR_LINE}"
    echo -e "  q - Quit${CLEAR_LINE}"
    echo -e "  j - Submit test job${CLEAR_LINE}"
    echo -e "  l - View logs (interactive)${CLEAR_LINE}"
    echo -e "  r - Refresh now${CLEAR_LINE}"
    echo -e "${CLEAR_LINE}"
}

submit_test_job() {
    URL="https://test-$(date +%s).example.com"
    curl -s -X POST "${CONTROL_PLANE}/new-job" \
        -H "Content-Type: application/x-www-form-urlencoded" \
        -d "url=${URL}" > /dev/null
    echo -e "${GREEN}✓ Submitted job: ${URL}${NC}"
    sleep 1
}

view_logs_menu() {
    clear_screen
    echo -e "${BLUE}--- Log Viewer ---${NC}"
    echo "1) node1 (control plane)"
    echo "2) node2 (control plane)"
    echo "3) node3 (control plane)"
    echo "4) worker1"
    echo "5) worker2"
    echo "6) All workers"
    echo "b) Back to main"
    echo ""
    read -p "Select: " choice

    case $choice in
        1) docker logs --tail 50 -f mvm-ci-node1 ;;
        2) docker logs --tail 50 -f mvm-ci-node2 ;;
        3) docker logs --tail 50 -f mvm-ci-node3 ;;
        4) docker logs --tail 50 -f mvm-ci-worker1 ;;
        5) docker logs --tail 50 -f mvm-ci-worker2 ;;
        6) docker logs --tail 50 -f mvm-ci-worker1 & docker logs --tail 50 -f mvm-ci-worker2 ;;
        *) return ;;
    esac
}

# Cleanup on exit
cleanup() {
    echo -ne "${SHOW_CURSOR}${CLEAR_LINE}\n"
    exit 0
}

trap cleanup EXIT INT TERM

# Main loop
if [ "$1" = "--once" ]; then
    # Single run mode (no fancy terminal control)
    clear
    echo -e "${BLUE}=== MVM-CI Cluster Monitor ===${NC}"
    echo -e "Time: $(date '+%Y-%m-%d %H:%M:%S')"
    echo ""
    show_cluster_status
    show_queue_stats
    show_recent_jobs
    show_worker_activity
    exit 0
fi

# Interactive mode with smooth updates
REFRESH_INTERVAL=2
FIRST_RUN=true

while true; do
    if [ "$FIRST_RUN" = true ]; then
        init_screen
        FIRST_RUN=false
    else
        update_screen
    fi

    show_header
    show_cluster_status
    show_queue_stats
    show_recent_jobs
    show_worker_activity
    show_help

    echo -e "${BLUE}Auto-refreshing in ${REFRESH_INTERVAL}s... (press any key for commands)${NC}${CLEAR_LINE}"

    # Wait for input with timeout
    read -t $REFRESH_INTERVAL -n 1 key

    case $key in
        q|Q)
            cleanup
            ;;
        j|J)
            submit_test_job
            ;;
        l|L)
            view_logs_menu
            FIRST_RUN=true  # Re-init screen after viewing logs
            ;;
        r|R)
            # Just refresh immediately
            continue
            ;;
    esac
done
