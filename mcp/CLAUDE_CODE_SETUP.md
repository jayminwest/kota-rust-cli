# KOTA MCP Server Setup for Claude Code

This directory contains the MCP (Model Context Protocol) integration for KOTA with Claude Code.

## Quick Start

### 1. Start the Bridge Server

In one terminal:
```bash
cd mcp/rust-bridge-server
./run-bridge-server.sh
```

The bridge server will start on `http://localhost:8080` and provide the API endpoints that the KOTA MCP server expects.

### 2. Use Claude Code with MCP Tools

In another terminal:
```bash
claude
```

You now have access to 8 KOTA MCP tools:

- **send_to_mac_pro** - Send data/insights to Mac Pro system
- **query_mac_pro_data** - Query Mac Pro MCP servers (calendar, finance, etc.)
- **get_mac_pro_status** - Get system status from Mac Pro
- **get_bridge_logs** - View communication logs
- **analyze_kota_context** - Analyze current development context
- **send_proactive_insight** - Send insights to Mac Pro
- **sync_project_status** - Sync project progress
- **request_mac_pro_assistance** - Request assistance from Mac Pro

### 3. Test the Setup

```bash
cd mcp
./test-mcp-setup.sh
```

## Architecture

```
Claude Code → KOTA MCP Server → Bridge Server (localhost:8080) → [Future: Mac Pro]
                    ↓                     ↓
              8 MCP Tools         Knowledge/Context Storage
```

## Configuration

The KOTA MCP server is configured in Claude Code with:
- **Command**: `/path/to/kota-mcp-server`
- **Environment**:
  - `BRIDGE_HOST=localhost`
  - `BRIDGE_PORT=8080`
  - `BRIDGE_SECRET=kota-bridge-secret-2025`

## Bridge Server Endpoints

The bridge server provides these API endpoints:

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/health` | GET | Health check |
| `/api/send-knowledge` | POST | Store knowledge from MCP |
| `/api/send-context-update` | POST | Update context information |
| `/api/query-mcp-data` | POST | Query external MCP servers |
| `/api/system-status` | GET | Get system status |
| `/api/receive-insight` | POST | Receive insights |

## Troubleshooting

### Bridge Server Not Starting
```bash
# Check if port 8080 is available
lsof -i :8080

# View bridge server logs
cd mcp/rust-bridge-server
./run-bridge-server.sh
```

### MCP Server Not Working in Claude Code
```bash
# Check MCP configuration
claude mcp list

# Rebuild MCP server
cd mcp/kota-mcp-server
cargo build --release

# Test MCP server directly
./target/release/kota-mcp-server
```

### Cannot Connect to Bridge
1. Ensure bridge server is running on port 8080
2. Check firewall settings
3. Verify environment variables are set correctly

## Development

### Adding New MCP Tools

1. Add tool definition to `kota-mcp-server/src/tools.rs`
2. Implement tool handler in `kota-mcp-server/src/main.rs`
3. Add corresponding API endpoint in `rust-bridge-server/src/server.rs`
4. Rebuild both servers

### Adding New Bridge Endpoints

1. Add route in `rust-bridge-server/src/server.rs`
2. Implement handler function
3. Update API documentation

## Future Enhancements

- WebSocket support for real-time communication
- Integration with actual Mac Pro MCP servers
- Enhanced authentication and security
- Distributed knowledge synchronization
- Advanced pattern analysis and insights