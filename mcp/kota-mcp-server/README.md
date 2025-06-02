# KOTA MCP Server for Claude Code

A Model Context Protocol (MCP) server that enables Claude Code to communicate with the KOTA bridge system, providing seamless integration between Claude Code and the Mac Pro (kota_md) system.

## Overview

This MCP server acts as a bridge between Claude Code and the KOTA ecosystem, allowing Claude Code to:
- Send insights and data to the Mac Pro system
- Query Mac Pro MCP servers (calendar, finance, etc.)
- Analyze current development context
- Provide proactive recommendations
- Synchronize project status across systems

## Architecture

```
Claude Code                    KOTA MCP Server                 Bridge Server                Mac Pro
┌─────────────────┐           ┌─────────────────┐             ┌─────────────────┐         ┌──────────────┐
│ MCP Client      │◄─stdio───►│ MCP Server      │◄───HTTP────►│ rust-bridge     │◄─HTTP──►│ kota_md      │
│ (Claude Code)   │           │ Port: stdio     │             │ Port: 8081      │         │ Port: 8080   │
└─────────────────┘           └─────────────────┘             └─────────────────┘         └──────────────┘
```

## Features

### 🔧 Available Tools

1. **send_to_mac_pro**
   - Send data, insights, or commands to Mac Pro
   - Categories: knowledge, context, insight, command, analysis
   - Metadata support for priority, tags, and related files

2. **query_mac_pro_data**
   - Query Mac Pro MCP servers (google-calendar, plaid-finance, gmail, etc.)
   - Access external service data through Mac Pro
   - Pass-through tool arguments

3. **get_mac_pro_status**
   - Get comprehensive system status from Mac Pro
   - Health monitoring and system information

4. **get_bridge_logs**
   - Access communication logs for debugging
   - Configurable log levels and limits

5. **analyze_kota_context**
   - Analyze current KOTA CLI development context
   - Generate insights about files, commands, patterns
   - Focus areas: performance, security, architecture

6. **send_proactive_insight**
   - Send proactive recommendations to Mac Pro
   - Confidence scoring and categorization
   - Urgency levels and actionable flags

7. **sync_project_status**
   - Synchronize project progress with Mac Pro
   - Track blockers and next steps
   - Project status updates

8. **request_mac_pro_assistance**
   - Request specific help from Mac Pro systems
   - Calendar checks, financial data, research
   - Priority-based request handling

## Quick Start

### 1. Prerequisites

Ensure the bridge server is running:
```bash
# From kota-rust-cli root directory
./run_bridge_server.sh
```

### 2. Build the MCP Server

```bash
cd mcp/kota-mcp-server
./run-mcp-server.sh
```

This will:
- Build the MCP server
- Test bridge connectivity
- Display Claude Code configuration

### 3. Configure Claude Code

Add this to your Claude Code MCP settings:

```json
{
  "mcpServers": {
    "kota-bridge": {
      "command": "cargo",
      "args": [
        "run",
        "--manifest-path",
        "/Users/jayminwest/Projects/kota-rust-cli/mcp/kota-mcp-server/Cargo.toml",
        "--bin",
        "kota-mcp-server"
      ],
      "cwd": "/Users/jayminwest/Projects/kota-rust-cli/mcp/kota-mcp-server",
      "env": {
        "BRIDGE_HOST": "localhost",
        "BRIDGE_PORT": "8081",
        "BRIDGE_SECRET": "kota-bridge-secret-2025",
        "RUST_LOG": "info"
      }
    }
  }
}
```

### 4. Alternative: Use Compiled Binary

For better performance, use the compiled binary:

```bash
cd mcp/kota-mcp-server
cargo build --release
```

Then use this configuration:

```json
{
  "mcpServers": {
    "kota-bridge": {
      "command": "/Users/jayminwest/Projects/kota-rust-cli/mcp/kota-mcp-server/target/release/kota-mcp-server",
      "env": {
        "BRIDGE_HOST": "localhost",
        "BRIDGE_PORT": "8081",
        "BRIDGE_SECRET": "kota-bridge-secret-2025",
        "RUST_LOG": "info"
      }
    }
  }
}
```

## Usage Examples

### Send Knowledge to Mac Pro

```
Use the send_to_mac_pro tool to send this insight to Mac Pro:
- Category: code_analysis
- Content: "Identified performance bottleneck in data processing loop. Consider implementing batch processing for 15% improvement."
- Metadata: {"priority": "high", "tags": ["performance", "optimization"]}
```

### Query Calendar Data

```
Use query_mac_pro_data to check my calendar:
- Server: google-calendar
- Tool: list_events
- Arguments: {"maxResults": 5, "timeMin": "2025-06-01T00:00:00Z"}
```

### Analyze Current Context

```
Use analyze_kota_context to analyze the current development session:
- Analysis type: patterns
- Focus area: performance
```

### Send Proactive Insight

```
Use send_proactive_insight to recommend:
- Insight: "Based on recent commit patterns, consider implementing unit tests for the new authentication module"
- Confidence: 0.85
- Category: code_quality
- Urgency: medium
```

### Sync Project Status

```
Use sync_project_status to update:
- Project: KOTA Bridge Implementation
- Status: in_progress
- Progress: "Completed MCP server, testing Claude Code integration"
- Next steps: ["Documentation", "Error handling improvements"]
```

## Configuration

### Environment Variables

```bash
# Bridge server connection
BRIDGE_HOST=localhost
BRIDGE_PORT=8081
BRIDGE_SECRET=kota-bridge-secret-2025

# Logging
RUST_LOG=info

# Feature flags
ENABLE_PROACTIVE_INSIGHTS=true
ENABLE_CONTEXT_SYNC=true
ENABLE_AUTO_DISCOVERY=true
```

### Bridge Server Dependency

The MCP server requires the `rust-bridge-server` to be running. The bridge server handles:
- Communication with Mac Pro (kota_md)
- HTTP to MCP protocol translation
- Communication logging
- Health monitoring

## Development

### Building

```bash
cargo build --release
```

### Testing

```bash
# Test MCP server standalone
cargo run --release -- --standalone

# Test bridge connectivity
curl http://localhost:8081/health
```

### Debugging

```bash
# Enable debug logging
RUST_LOG=debug cargo run
```

## Integration with KOTA CLI

The MCP server integrates with the broader KOTA ecosystem:

1. **Context Awareness**: Analyzes current KOTA CLI context and files
2. **Proactive Insights**: Generates recommendations based on development patterns
3. **Cross-System Communication**: Enables Claude Code to access Mac Pro data
4. **Project Synchronization**: Keeps all systems aware of development progress

## Troubleshooting

### MCP Server Not Connecting

1. **Check Bridge Server**: Ensure `rust-bridge-server` is running on port 8081
2. **Verify Configuration**: Check environment variables in `.env`
3. **Test Connectivity**: Use `curl http://localhost:8081/health`
4. **Check Logs**: Look at stderr output for error messages

### Claude Code Integration Issues

1. **Restart Claude Code**: After adding MCP configuration
2. **Check Paths**: Verify absolute paths in configuration
3. **Environment Variables**: Ensure all required env vars are set
4. **Permissions**: Check file permissions for the binary

### Bridge Communication Issues

1. **Mac Pro Connectivity**: Verify Mac Pro bridge server is running
2. **Network Configuration**: Check IP addresses and ports
3. **Authentication**: Verify `BRIDGE_SECRET` matches across systems
4. **Firewall**: Ensure ports 8080 and 8081 are accessible

## Security

- **Local Network Only**: All communication stays on local network
- **Bearer Token Authentication**: Shared secret authentication
- **No External Access**: MCP server only accessible via stdio
- **Safe Error Handling**: No sensitive data in error messages

## Logs and Monitoring

- **MCP Communication**: Logged to stderr (visible in Claude Code)
- **Bridge Communication**: Logged via bridge server
- **Error Tracking**: Comprehensive error reporting
- **Health Monitoring**: Automatic connection health checks

## Future Enhancements

- **WebSocket Support**: Real-time bidirectional communication
- **Enhanced Discovery**: Automatic Mac Pro discovery
- **Tool Plugins**: Extensible tool system
- **Performance Metrics**: Communication performance tracking
- **Caching**: Local caching for frequently accessed data

---

This MCP server bridges the gap between Claude Code and the KOTA ecosystem, enabling powerful cross-system AI collaboration and context awareness.