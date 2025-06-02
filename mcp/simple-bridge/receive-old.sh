#!/bin/bash
# Simple message receiver for KOTA bridge
# Usage: ./receive.sh [--watch] [--json]
#
# Options:
#   --watch  Continuously watch for new messages
#   --json   Output in JSON format (for tool integration)

# Configuration
BRIDGE_DIR="${KOTA_BRIDGE_DIR:-/tmp/kota-bridge}"
AGENT_NAME="${AGENT_NAME:-claude}"
INBOX="${BRIDGE_DIR}/inbox/${AGENT_NAME}"
ARCHIVE="${BRIDGE_DIR}/archive/${AGENT_NAME}"

# Ensure directories exist
mkdir -p "${INBOX}" "${ARCHIVE}"

# Process a single message
process_message() {
    local file=$1
    local json_mode=$2
    
    if [ ! -f "$file" ]; then
        return
    fi
    
    # Read message content
    local content=$(cat "$file")
    local type=$(echo "$content" | grep "^TYPE:" | cut -d' ' -f2-)
    local from=$(echo "$content" | grep "^FROM:" | cut -d' ' -f2-)
    local time=$(echo "$content" | grep "^TIME:" | cut -d' ' -f2-)
    local id=$(echo "$content" | grep "^ID:" | cut -d' ' -f2-)
    local body=$(echo "$content" | sed '1,/^$/d')
    
    if [ "$json_mode" == "true" ]; then
        # Output as JSON
        jq -n \
            --arg type "$type" \
            --arg from "$from" \
            --arg time "$time" \
            --arg id "$id" \
            --arg body "$body" \
            --arg filename "$(basename $file)" \
            '{type: $type, from: $from, time: $time, id: $id, body: $body, filename: $filename}'
    else
        # Human-readable output
        echo "╔════════════════════════════════════════"
        echo "║ 📨 New Message from: $from"
        echo "║ Type: $type | ID: $id"
        echo "║ Time: $time"
        echo "╠════════════════════════════════════════"
        echo "$body" | sed 's/^/║ /'
        echo "╚════════════════════════════════════════"
        echo
    fi
    
    # Archive the message
    mv "$file" "${ARCHIVE}/$(basename $file)"
}

# Check for arguments
WATCH_MODE=false
JSON_MODE=false

for arg in "$@"; do
    case $arg in
        --watch)
            WATCH_MODE=true
            ;;
        --json)
            JSON_MODE=true
            ;;
        --help)
            echo "Usage: $0 [--watch] [--json]"
            echo "  --watch  Continuously watch for new messages"
            echo "  --json   Output in JSON format"
            exit 0
            ;;
    esac
done

if [ "$WATCH_MODE" == "true" ]; then
    echo "👀 Watching for messages in ${INBOX}..."
    echo "Press Ctrl+C to stop"
    echo
    
    while true; do
        for msg in ${INBOX}/*.msg 2>/dev/null; do
            [ -e "$msg" ] && process_message "$msg" "$JSON_MODE"
        done
        sleep 0.5
    done
else
    # Process all pending messages once
    found=0
    for msg in ${INBOX}/*.msg 2>/dev/null; do
        if [ -e "$msg" ]; then
            process_message "$msg" "$JSON_MODE"
            found=$((found + 1))
        fi
    done
    
    if [ $found -eq 0 ] && [ "$JSON_MODE" != "true" ]; then
        echo "📭 No pending messages in ${INBOX}"
    fi
fi