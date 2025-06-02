#!/bin/bash

# KOTA Rust Bridge Server Runner
# Connects to Mac Pro bridge server and provides local API endpoints

set -e

cd "$(dirname "$0")"

echo "🚀 Starting KOTA Rust Bridge Server..."
echo "📁 Working directory: $(pwd)"

# Get network information
HOSTNAME=$(hostname)
LOCAL_IP=$(ifconfig | grep -Eo 'inet (addr:)?([0-9]*\.){3}[0-9]*' | grep -Eo '([0-9]*\.){3}[0-9]*' | grep -v '127.0.0.1' | head -1)
TAILSCALE_IP=$(tailscale ip --4 2>/dev/null || echo "Not available")

echo "🌐 Network Information:"
echo "   Hostname: $HOSTNAME"
echo "   Local IP: $LOCAL_IP"
if [ "$TAILSCALE_IP" != "Not available" ]; then
    echo "   Tailscale IP: $TAILSCALE_IP"
fi

echo "🔗 Connection URLs:"
echo "   Local: http://localhost:8081"
echo "   Network: http://$LOCAL_IP:8081"
if [ "$TAILSCALE_IP" != "Not available" ]; then
    echo "   Tailscale: http://$TAILSCALE_IP:8081"
fi

echo "🛠️  API Endpoints:"
echo "   Health: http://$LOCAL_IP:8081/health"
echo "   Discovery: http://$LOCAL_IP:8081/discovery"

echo "📞 Press Ctrl+C to stop the server"
echo ""

# Build if needed
if [ ! -f "target/release/rust-bridge-server" ] || [ "src/" -nt "target/release/rust-bridge-server" ]; then
    echo "🔨 Building bridge server..."
    cargo build --release
    echo "✅ Build complete"
fi

echo "🔧 Environment Configuration:"
# Set environment variables to connect to Mac Pro via Tailscale
export RUST_CLI_PORT=8081
export MAC_PRO_HOST=100.118.223.57  # Tailscale IP of Mac Pro
export BRIDGE_SECRET=kota-bridge-secret-2025  # Secret for incoming MCP requests
export MAC_PRO_SECRET=default-secret-change-me  # Secret for outgoing requests to Mac Pro
export RUST_LOG=info
export ENABLE_QUEUE_POLLING=false

echo "   RUST_CLI_PORT: $RUST_CLI_PORT"
echo "   MAC_PRO_HOST: $MAC_PRO_HOST"
echo "   BRIDGE_SECRET: $BRIDGE_SECRET"
echo "   MAC_PRO_SECRET: $MAC_PRO_SECRET"
echo "   QUEUE_POLLING: $ENABLE_QUEUE_POLLING"
echo ""

echo "🚀 Starting Rust Bridge Server..."
echo "🔌 Connecting to Mac Pro at: http://$MAC_PRO_HOST:8080"
echo ""

# Run the server
exec ./target/release/rust-bridge-server