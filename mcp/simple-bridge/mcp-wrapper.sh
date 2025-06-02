#!/bin/bash
# MCP-compatible wrapper for simple bridge
# This allows the simple bridge to be used as an MCP tool

# Read JSON-RPC request from stdin
read -r request

# Extract method and parameters using basic parsing
method=$(echo "$request" | grep -o '"method":"[^"]*"' | cut -d'"' -f4)
id=$(echo "$request" | grep -o '"id":[0-9]*' | cut -d':' -f2)

# Set agent name for Claude Code
export AGENT_NAME=claude
export KOTA_BRIDGE_DIR="${KOTA_BRIDGE_DIR:-/tmp/kota-bridge}"

case "$method" in
    "initialize")
        cat << EOF
{
    "jsonrpc": "2.0",
    "id": ${id},
    "result": {
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {},
            "resources": {}
        },
        "serverInfo": {
            "name": "kota-simple-bridge",
            "version": "1.0.0"
        }
    }
}
EOF
        ;;
        
    "tools/list")
        cat << EOF
{
    "jsonrpc": "2.0",
    "id": ${id},
    "result": {
        "tools": [
            {
                "name": "send_message",
                "description": "Send a message through the simple bridge",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "to": {"type": "string", "description": "Recipient agent name"},
                        "type": {"type": "string", "enum": ["knowledge", "context", "insight", "command", "query"]},
                        "message": {"type": "string", "description": "Message content"}
                    },
                    "required": ["to", "type", "message"]
                }
            },
            {
                "name": "receive_messages",
                "description": "Check for pending messages",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            }
        ]
    }
}
EOF
        ;;
        
    "tools/call")
        # Extract tool name and arguments
        tool=$(echo "$request" | grep -o '"name":"[^"]*"' | cut -d'"' -f4)
        
        case "$tool" in
            "send_message")
                # Extract parameters
                to=$(echo "$request" | grep -o '"to":"[^"]*"' | cut -d'"' -f4)
                type=$(echo "$request" | grep -o '"type":"[^"]*"' | cut -d'"' -f4)
                message=$(echo "$request" | sed -n 's/.*"message":"\([^"]*\)".*/\1/p')
                
                # Send the message
                $(dirname "$0")/send.sh "$to" "$type" "$message" >/dev/null 2>&1
                
                cat << EOF
{
    "jsonrpc": "2.0",
    "id": ${id},
    "result": {
        "content": [
            {
                "type": "text",
                "text": "✅ Message sent successfully to $to"
            }
        ]
    }
}
EOF
                ;;
                
            "receive_messages")
                # Get messages in JSON format
                messages=$($(dirname "$0")/receive.sh --json 2>/dev/null)
                
                if [ -z "$messages" ]; then
                    text="📭 No pending messages"
                else
                    text="📨 Received messages:\n$messages"
                fi
                
                cat << EOF
{
    "jsonrpc": "2.0",
    "id": ${id},
    "result": {
        "content": [
            {
                "type": "text",
                "text": "$text"
            }
        ]
    }
}
EOF
                ;;
        esac
        ;;
        
    *)
        # Unknown method
        cat << EOF
{
    "jsonrpc": "2.0",
    "id": ${id},
    "error": {
        "code": -32601,
        "message": "Method not found: $method"
    }
}
EOF
        ;;
esac