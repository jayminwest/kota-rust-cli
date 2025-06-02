#!/bin/bash

# KOTA MCP Server Launcher for Claude Code
# This script starts the MCP server that Claude Code can connect to

set -e

echo "🌉 Starting KOTA MCP Server for Claude Code..."

# Change to MCP server directory
cd "$(dirname "$0")"

# Check if .env file exists
if [ ! -f .env ]; then
    echo "⚠️  .env file not found, creating from template..."
    
    # Prompt for bridge server details
    read -p "Enter bridge server host (default: localhost): " BRIDGE_HOST
    BRIDGE_HOST=${BRIDGE_HOST:-localhost}
    
    read -p "Enter bridge server port (default: 8081): " BRIDGE_PORT
    BRIDGE_PORT=${BRIDGE_PORT:-8081}
    
    # Create .env file
    cat > .env << EOF
# KOTA MCP Server Configuration for Claude Code
BRIDGE_HOST=$BRIDGE_HOST
BRIDGE_PORT=$BRIDGE_PORT
BRIDGE_SECRET=kota-bridge-secret-2025
RUST_LOG=info
MCP_SERVER_NAME=kota-mcp-server
MCP_SERVER_VERSION=0.1.0
ENABLE_PROACTIVE_INSIGHTS=true
ENABLE_CONTEXT_SYNC=true
ENABLE_AUTO_DISCOVERY=true
EOF
    
    echo "✅ Created .env file with bridge server: $BRIDGE_HOST:$BRIDGE_PORT"
fi

# Build the MCP server
echo "🔨 Building MCP server..."
if cargo build --release; then
    echo "✅ Build successful"
else
    echo "❌ Build failed"
    exit 1
fi

# Test bridge connection
echo "🔍 Testing bridge server connection..."
BRIDGE_HOST=$(grep BRIDGE_HOST .env | cut -d '=' -f2)
BRIDGE_PORT=$(grep BRIDGE_PORT .env | cut -d '=' -f2)

if curl -s --connect-timeout 5 "http://$BRIDGE_HOST:$BRIDGE_PORT/health" > /dev/null; then
    echo "✅ Bridge server is accessible at $BRIDGE_HOST:$BRIDGE_PORT"
else
    echo "⚠️  Warning: Bridge server not accessible at $BRIDGE_HOST:$BRIDGE_PORT"
    echo "   Make sure the rust-bridge-server is running first"
    echo "   Run: ../../run_bridge_server.sh"
fi

# Check if running standalone or for Claude Code
if [ "$1" = "--standalone" ]; then
    echo "🚀 Starting MCP server in standalone mode..."
    echo "   This mode is for testing - Claude Code uses stdio communication"
    echo "   Press Ctrl+C to stop"
    echo ""
    cargo run --release
else
    echo "✅ MCP server ready for Claude Code"
    echo ""
    echo "📋 Claude Code Configuration:"
    echo "   Add this to your Claude Code MCP settings:"
    echo ""
    cat claude-code-config.json | jq '.'
    echo ""
    echo "📖 Usage Instructions:"
    echo "   1. Copy the configuration above to Claude Code"
    echo "   2. Restart Claude Code"
    echo "   3. The MCP tools will be available in Claude Code"
    echo ""
    echo "🔧 Available Tools:"
    echo "   - send_to_mac_pro: Send data to Mac Pro system"
    echo "   - query_mac_pro_data: Query calendar, finance, etc."
    echo "   - get_mac_pro_status: Get system status"
    echo "   - analyze_kota_context: Analyze current development context"
    echo "   - send_proactive_insight: Send insights to Mac Pro"
    echo "   - sync_project_status: Update project status"
    echo "   - request_mac_pro_assistance: Request help from Mac Pro"
fi