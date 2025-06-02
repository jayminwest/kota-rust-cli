#!/bin/bash
# Simple message sender for KOTA bridge
# Usage: ./send.sh <to> <type> <message>
#
# Examples:
#   ./send.sh mac-pro knowledge "Found optimization opportunity"
#   ./send.sh claude context "System resources: CPU 45%, Memory 8GB free"
#   ./send.sh mac-pro insight "Consider running heavy tasks at night"

# Configuration
BRIDGE_DIR="${KOTA_BRIDGE_DIR:-/tmp/kota-bridge}"
AGENT_NAME="${AGENT_NAME:-claude}"

# Arguments
TO=$1
TYPE=$2
MESSAGE=$3

# Validate arguments
if [ -z "$TO" ] || [ -z "$TYPE" ] || [ -z "$MESSAGE" ]; then
    echo "Usage: $0 <to> <type> <message>"
    echo "Types: knowledge, context, insight, command, query"
    exit 1
fi

# Generate message ID
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
ID=$(head -c 16 /dev/urandom | xxd -p | cut -c1-8)
FILENAME="${TIMESTAMP}_${TYPE}_${ID}.msg"

# Ensure directory exists
mkdir -p "${BRIDGE_DIR}/outbox/${AGENT_NAME}"

# Create message file
cat > "${BRIDGE_DIR}/outbox/${AGENT_NAME}/${FILENAME}" << EOF
TYPE: ${TYPE}
FROM: ${AGENT_NAME}
TO: ${TO}
TIME: $(date -u +%Y-%m-%dT%H:%M:%SZ)
ID: ${ID}

${MESSAGE}
EOF

echo "✅ Message sent: ${FILENAME}"
echo "   Type: ${TYPE}"
echo "   To: ${TO}"
echo "   Location: ${BRIDGE_DIR}/outbox/${AGENT_NAME}/"