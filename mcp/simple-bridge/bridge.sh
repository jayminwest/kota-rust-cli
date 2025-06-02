#!/bin/bash
# Simple bridge router for KOTA
# Moves messages from outboxes to appropriate inboxes
# Usage: ./bridge.sh [--verbose]

# Configuration
BRIDGE_DIR="${KOTA_BRIDGE_DIR:-/tmp/kota-bridge}"
VERBOSE=false

if [ "$1" == "--verbose" ]; then
    VERBOSE=true
fi

# Ensure all directories exist
mkdir -p "${BRIDGE_DIR}"/{inbox,outbox,archive}/{claude,mac,mac-pro}

echo "🌉 KOTA Simple Bridge Started"
echo "📁 Bridge directory: ${BRIDGE_DIR}"
echo "Press Ctrl+C to stop"
echo

# Stats
messages_routed=0
start_time=$(date +%s)

# Route messages from one outbox to appropriate inbox
route_messages() {
    local from=$1
    local to=$2
    local moved=0
    
    # Use find to handle missing directory gracefully
    find "${BRIDGE_DIR}/outbox/${from}" -name "*.msg" -type f 2>/dev/null | while read -r msg; do
        if [ -e "$msg" ]; then
            # Read the TO field from the message
            local target=$(grep "^TO:" "$msg" | cut -d' ' -f2- | tr -d ' ')
            
            # Route to appropriate inbox
            case "$target" in
                claude*)
                    mv "$msg" "${BRIDGE_DIR}/inbox/claude/"
                    ;;
                mac*|mac-pro*)
                    mv "$msg" "${BRIDGE_DIR}/inbox/mac/"
                    ;;
                *)
                    # Default routing based on from
                    if [ "$from" == "claude" ]; then
                        mv "$msg" "${BRIDGE_DIR}/inbox/mac/"
                    else
                        mv "$msg" "${BRIDGE_DIR}/inbox/claude/"
                    fi
                    ;;
            esac
            
            moved=$((moved + 1))
            messages_routed=$((messages_routed + 1))
            
            if [ "$VERBOSE" == "true" ]; then
                echo "📤 Routed: $(basename $msg) from $from to $target"
            fi
        fi
    done
    
    return $moved
}

# Main routing loop
while true; do
    moved_total=0
    
    # Route from all known agents
    for agent in claude mac mac-pro; do
        route_messages "$agent" ""
        moved_total=$((moved_total + $?))
    done
    
    # Show stats periodically
    if [ $((messages_routed % 10)) -eq 0 ] && [ $messages_routed -gt 0 ]; then
        runtime=$(($(date +%s) - start_time))
        if command -v bc >/dev/null 2>&1; then
            rate=$(echo "scale=2; $messages_routed / $runtime" | bc)
        else
            rate="N/A"
        fi
        echo "📊 Stats: $messages_routed messages routed | Rate: $rate msg/sec"
    fi
    
    # Adaptive sleep - sleep less when busy
    if [ $moved_total -gt 0 ]; then
        sleep 0.1  # Fast when active
    else
        sleep 0.5  # Slower when idle
    fi
done