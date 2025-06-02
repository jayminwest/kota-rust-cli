# Simple File-Based Agent Bridge

A drastically simplified alternative to the complex MCP bridge system, using basic file operations and bash scripts for reliable agent-to-agent communication.

## Why Simpler?

The current MCP bridge system has:
- 2 Rust servers with 1000+ lines of code
- Complex HTTP/JSON-RPC protocols
- Tailscale networking requirements
- Authentication tokens and headers
- Async operations and thread management
- JSON serialization/deserialization overhead

This simple alternative uses:
- Shared directories (local or network mounted)
- Plain text files with simple formats
- Basic bash scripts (~50 lines total)
- No authentication (relies on file permissions)
- Synchronous, predictable operations
- Human-readable message formats

## Architecture

```
Claude Code Agent                    Mac Pro Agent
     |                                    |
     v                                    v
/shared/outbox/claude/  ------>  /shared/inbox/mac/
/shared/inbox/claude/   <------  /shared/outbox/mac/
```

## Message Format

Simple text files with naming convention:
```
TIMESTAMP_TYPE_ID.msg

Example: 20250601_120000_knowledge_abc123.msg
```

Content format:
```
TYPE: knowledge
FROM: claude-code
TO: mac-pro
TIME: 2025-06-01T12:00:00Z
METADATA: category=insight,priority=high

This is the actual message content.
Can be multiple lines.
```

## Implementation

### 1. Directory Structure
```bash
/shared/
├── inbox/
│   ├── claude/    # Messages for Claude to read
│   └── mac/       # Messages for Mac to read
├── outbox/
│   ├── claude/    # Messages Claude wants to send
│   └── mac/       # Messages Mac wants to send
└── archive/       # Processed messages (optional)
```

### 2. Send Script (send.sh)
```bash
#!/bin/bash
# Usage: ./send.sh <to> <type> <message>

TO=$1
TYPE=$2
MESSAGE=$3
FROM=${AGENT_NAME:-claude}
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
ID=$(uuidgen | tr '[:upper:]' '[:lower:]' | cut -c1-8)
FILENAME="${TIMESTAMP}_${TYPE}_${ID}.msg"

# Create message file
cat > "/shared/outbox/${FROM}/${FILENAME}" << EOF
TYPE: ${TYPE}
FROM: ${FROM}
TO: ${TO}
TIME: $(date -u +%Y-%m-%dT%H:%M:%SZ)

${MESSAGE}
EOF

echo "Message sent: ${FILENAME}"
```

### 3. Receive Script (receive.sh)
```bash
#!/bin/bash
# Usage: ./receive.sh [--watch]

AGENT=${AGENT_NAME:-claude}
INBOX="/shared/inbox/${AGENT}"

process_message() {
    local file=$1
    echo "=== New Message ==="
    cat "$file"
    echo "=================="
    
    # Mark as read by moving to archive
    mv "$file" "/shared/archive/"
}

if [[ "$1" == "--watch" ]]; then
    echo "Watching for messages..."
    while true; do
        for msg in ${INBOX}/*.msg 2>/dev/null; do
            [ -e "$msg" ] && process_message "$msg"
        done
        sleep 1
    done
else
    # Process all pending messages
    for msg in ${INBOX}/*.msg 2>/dev/null; do
        [ -e "$msg" ] && process_message "$msg"
    done
fi
```

### 4. Bridge Script (bridge.sh)
```bash
#!/bin/bash
# Moves messages from outbox to appropriate inbox

while true; do
    # Move Claude's outbox to Mac's inbox
    for msg in /shared/outbox/claude/*.msg 2>/dev/null; do
        [ -e "$msg" ] && mv "$msg" /shared/inbox/mac/
    done
    
    # Move Mac's outbox to Claude's inbox
    for msg in /shared/outbox/mac/*.msg 2>/dev/null; do
        [ -e "$msg" ] && mv "$msg" /shared/inbox/claude/
    done
    
    sleep 0.5
done
```

## Usage Examples

### Sending a Message
```bash
# From Claude Code
AGENT_NAME=claude ./send.sh mac-pro knowledge "Discovered optimization in algorithm"

# From Mac Pro
AGENT_NAME=mac ./send.sh claude context "System load is high, consider delaying intensive tasks"
```

### Receiving Messages
```bash
# One-time check
AGENT_NAME=claude ./receive.sh

# Continuous monitoring
AGENT_NAME=claude ./receive.sh --watch
```

## Advantages

1. **Simplicity**: Total implementation is under 100 lines of bash
2. **Reliability**: File operations are atomic and predictable
3. **Debuggability**: Messages are human-readable text files
4. **No Dependencies**: Works with basic Unix tools
5. **Network Agnostic**: Works with local dirs, NFS, SMB, Dropbox, etc.
6. **Easy Integration**: Any language can read/write text files
7. **Audit Trail**: Natural file-based history

## Network Options

### Local Development
```bash
# Use local filesystem
mkdir -p /tmp/kota-bridge/{inbox,outbox,archive}/{claude,mac}
```

### Network File System (NFS)
```bash
# Mount shared NFS volume
mount -t nfs mac-pro:/shared/kota-bridge /shared
```

### Dropbox/Syncthing
```bash
# Use any file sync service
ln -s ~/Dropbox/kota-bridge /shared
```

### SSH/SFTP
```bash
# Use SSH for remote file operations
scp message.msg mac-pro:/shared/inbox/claude/
```

## Security Considerations

- Relies on filesystem permissions
- Can add GPG signing for authenticity
- Can encrypt message content if needed
- Simple allow/deny lists via file patterns

## Migration from Complex System

1. **Tool Wrapper**: Create MCP tool that calls send.sh
2. **Format Converter**: Simple script to convert JSON to text format
3. **Backward Compatibility**: Bridge can read both formats

## Performance

- **Latency**: Sub-second for local filesystem
- **Throughput**: Limited by filesystem, typically 1000s msgs/sec
- **Scalability**: Add more directories for topics/priorities
- **Reliability**: Filesystem guarantees durability

## Monitoring

```bash
# Count pending messages
ls -1 /shared/inbox/*/*.msg 2>/dev/null | wc -l

# Watch message flow
watch -n 1 'ls -la /shared/*/claude/'

# Simple stats
find /shared -name "*.msg" -mtime -1 | wc -l  # Messages in last 24h
```

## Conclusion

This file-based approach eliminates 90% of the complexity while maintaining all essential functionality. It's easier to understand, debug, and maintain, making it ideal for reliable agent-to-agent communication.