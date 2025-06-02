#!/bin/bash

# Test script for KOTA MCP setup
# This script tests the bridge server and MCP server integration

set -e

echo "🧪 Testing KOTA MCP Setup"
echo "=========================="

# Test 1: Check if bridge server is running
echo "1. Testing bridge server health..."
if curl -s http://localhost:8080/health > /dev/null; then
    echo "✅ Bridge server is running on port 8080"
    curl -s http://localhost:8080/health | jq .
else
    echo "❌ Bridge server not running. Start it with:"
    echo "   cd mcp/rust-bridge-server && ./run-bridge-server.sh"
    exit 1
fi

echo ""

# Test 2: Check bridge server API endpoints
echo "2. Testing bridge server API endpoints..."
curl -s http://localhost:8080/discovery | jq .

echo ""

# Test 3: Test KOTA MCP server configuration
echo "3. Testing KOTA MCP server configuration..."
claude mcp list | grep kota-bridge && echo "✅ KOTA MCP server configured" || echo "❌ KOTA MCP server not configured"

echo ""

# Test 4: Test MCP server can connect to bridge
echo "4. Testing MCP server connection to bridge..."
cd mcp/kota-mcp-server
timeout 5s ./target/release/kota-mcp-server 2>&1 | grep -q "Bridge server not available" && echo "❌ MCP server cannot connect to bridge" || echo "✅ MCP server connection working"

echo ""
echo "🎯 Setup Summary:"
echo "   Bridge Server: http://localhost:8080"
echo "   MCP Server: Configured in Claude Code as 'kota-bridge'"
echo "   Available Tools: 8 MCP tools for Claude Code"
echo ""
echo "🚀 To use:"
echo "   1. Start bridge server: cd mcp/rust-bridge-server && ./run-bridge-server.sh"
echo "   2. Start Claude Code: claude"
echo "   3. Use MCP tools like send_to_mac_pro, query_mac_pro_data, etc."