#!/bin/bash
# Setup script for KOTA Simple Bridge
# Creates directories and tests the system

echo "🚀 KOTA Simple Bridge Setup"
echo "=========================="
echo

# Configuration
BRIDGE_DIR="${KOTA_BRIDGE_DIR:-/tmp/kota-bridge}"

# Create directory structure
echo "📁 Creating bridge directories in ${BRIDGE_DIR}..."
mkdir -p "${BRIDGE_DIR}"/{inbox,outbox,archive}/{claude,mac,mac-pro}

# Make scripts executable
echo "🔧 Making scripts executable..."
chmod +x send.sh receive.sh bridge.sh

# Test the system
echo
echo "🧪 Testing the bridge system..."
echo

# Start bridge in background
echo "Starting bridge router..."
./bridge.sh &
BRIDGE_PID=$!
sleep 1

# Test 1: Send from Claude to Mac
echo "Test 1: Claude → Mac"
AGENT_NAME=claude ./send.sh mac-pro knowledge "Test message from Claude"
sleep 0.5

# Test 2: Send from Mac to Claude  
echo
echo "Test 2: Mac → Claude"
AGENT_NAME=mac ./send.sh claude context "Test response from Mac"
sleep 0.5

# Test 3: Check Claude's inbox
echo
echo "Test 3: Checking messages"
echo "Claude's messages:"
AGENT_NAME=claude ./receive.sh

echo
echo "Mac's messages:"
AGENT_NAME=mac ./receive.sh

# Clean up
kill $BRIDGE_PID 2>/dev/null

echo
echo "✅ Setup complete!"
echo
echo "📚 Quick Start Guide:"
echo "  1. Start the bridge:     ./bridge.sh"
echo "  2. Send a message:       AGENT_NAME=claude ./send.sh mac-pro knowledge \"Your message\""
echo "  3. Receive messages:     AGENT_NAME=claude ./receive.sh --watch"
echo
echo "🔧 Configuration:"
echo "  - Bridge directory: ${BRIDGE_DIR}"
echo "  - Set KOTA_BRIDGE_DIR to change location"
echo "  - Set AGENT_NAME to identify sender/receiver"
echo
echo "💡 Integration with KOTA CLI:"
echo "  - Add to .bashrc: export PATH=\$PATH:$(pwd)"
echo "  - Or create symlinks in /usr/local/bin/"