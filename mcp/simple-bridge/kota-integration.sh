#!/bin/bash
# Example integration with KOTA CLI
# This shows how the simple bridge can be integrated into KOTA

# Function to send messages from KOTA
kota_send() {
    local msg_type=$1
    local content=$2
    local to=${3:-"mac-pro"}
    
    # Use the simple bridge
    AGENT_NAME=kota $(dirname "$0")/send.sh "$to" "$msg_type" "$content"
}

# Function to receive messages in KOTA
kota_receive() {
    # Get messages in JSON format for easy parsing
    AGENT_NAME=kota $(dirname "$0")/receive.sh --json | while read -r line; do
        if [ ! -z "$line" ]; then
            # Extract fields from JSON
            type=$(echo "$line" | jq -r '.type')
            from=$(echo "$line" | jq -r '.from')
            body=$(echo "$line" | jq -r '.body')
            
            # Process based on type
            case "$type" in
                command)
                    echo "🎯 Received command from $from: $body"
                    # Could execute with KOTA's secure executor
                    ;;
                context)
                    echo "📋 Context update from $from: $body"
                    # Could add to KOTA's context manager
                    ;;
                knowledge)
                    echo "📚 Knowledge from $from: $body"
                    # Could store in KOTA's knowledge base
                    ;;
                *)
                    echo "📨 Message from $from: $body"
                    ;;
            esac
        fi
    done
}

# Example: KOTA agent sending insights
kota_analyze_and_send() {
    echo "🤔 KOTA analyzing current context..."
    
    # Simulate KOTA analysis
    sleep 1
    
    # Send insight
    kota_send "insight" "Based on recent code changes, consider refactoring the authentication module for better security"
    
    # Send context update
    kota_send "context" "Working on security improvements in src/security/mod.rs"
    
    # Query for updates
    echo "📥 Checking for messages..."
    kota_receive
}

# Example usage in KOTA command
case "$1" in
    send)
        shift
        kota_send "$@"
        ;;
    receive)
        kota_receive
        ;;
    analyze)
        kota_analyze_and_send
        ;;
    *)
        echo "KOTA Bridge Integration Examples"
        echo "================================"
        echo
        echo "Usage:"
        echo "  $0 send <type> <message> [recipient]"
        echo "  $0 receive"
        echo "  $0 analyze"
        echo
        echo "Examples:"
        echo "  $0 send knowledge 'Discovered memory leak in parser'"
        echo "  $0 send command 'run cargo test'"
        echo "  $0 receive"
        echo
        echo "Integration points with KOTA:"
        echo "  - Context Manager: Auto-sync context updates"
        echo "  - Knowledge Base: Store insights and learnings"
        echo "  - Agent System: Enable agent-to-agent communication"
        echo "  - Security: Apply KOTA's security policies to commands"
        ;;
esac