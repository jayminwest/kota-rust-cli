# KOTA Bridge Authentication

## Overview

The KOTA bridge system uses Bearer token authentication to secure communication between components:

```
Claude Code (MCP) → rust-bridge-server → Mac Pro Bridge
```

## Authentication Flow

1. **MCP → rust-bridge-server**: Uses `Bearer kota-bridge-secret-2025`
2. **rust-bridge-server → Mac Pro**: Uses `Bearer default-secret-change-me`

## Configuration

### rust-bridge-server

The rust-bridge-server uses the following environment variables:
- `BRIDGE_SECRET`: Authentication token for incoming requests (default: `kota-bridge-secret-2025`)
- `MAC_PRO_HOST`: Mac Pro bridge server host (default: `100.118.223.57`)
- `RUST_CLI_PORT`: Port to listen on (default: `8081`)

### kota-mcp-server

The MCP server for Claude Code uses:
- `BRIDGE_HOST`: rust-bridge-server host (default: `localhost`)
- `BRIDGE_PORT`: rust-bridge-server port (default: `8081`)
- `BRIDGE_SECRET`: Authentication token (default: `kota-bridge-secret-2025`)

## Security Implementation

The rust-bridge-server implements authentication middleware that:
1. Allows public access to `/health` and `/discovery` endpoints
2. Requires `Bearer <token>` authentication for all `/api/*` endpoints
3. Returns 401 Unauthorized for invalid or missing tokens

## Testing Authentication

### Test with valid token:
```bash
curl -H "Authorization: Bearer kota-bridge-secret-2025" \
     http://localhost:8081/api/system-status
```

### Test without token (should fail):
```bash
curl http://localhost:8081/api/system-status
# Returns: 401 Unauthorized
```

## Troubleshooting

If you see authentication errors in Claude Code:
1. Check the rust-bridge-server logs for "Authentication failed" messages
2. Verify the BRIDGE_SECRET matches between kota-mcp-server and rust-bridge-server
3. Ensure the Authorization header format is exactly: `Bearer <token>`
4. Check that the rust-bridge-server is running on the expected port (8081)

## Default Tokens

- **MCP → rust-bridge-server**: `kota-bridge-secret-2025`
- **rust-bridge-server → Mac Pro**: `default-secret-change-me`

**Important**: Change these default tokens in production environments!