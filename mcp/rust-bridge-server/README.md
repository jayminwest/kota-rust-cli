# KOTA Rust Bridge Server

A high-performance Rust implementation of the KOTA inter-project communication system, designed to communicate with the Mac Pro (kota_md) bridge server and provide proactive AI insights.

## Features

- **Bidirectional Communication**: HTTP client/server for Mac Pro communication
- **100% Communication Logging**: JSON and JSONL format logging
- **Proactive Insights**: Time-based and pattern-based insight generation  
- **Network Discovery**: Automatic Mac Pro bridge server discovery
- **Queue Polling**: File-based message queue monitoring (optional)
- **RESTful API**: Full HTTP API for external integration
- **Health Monitoring**: Automatic connection health checks

## Quick Start

1. **Configure environment**:
   ```bash
   cp .env.example .env
   # Edit .env with your Mac Pro IP address
   ```

2. **Build and run**:
   ```bash
   cargo build --release
   cargo run
   ```

3. **Test connection**:
   ```bash
   curl http://localhost:8081/health
   curl http://localhost:8081/discovery
   ```

## Configuration

The server is configured via environment variables (see `.env` file):

- `MAC_PRO_HOST`: IP address of Mac Pro bridge server
- `RUST_CLI_PORT`: Port for this server (default: 8081)
- `BRIDGE_SECRET`: Authentication token for incoming requests from MCP (default: kota-bridge-secret-2025)
- `MAC_PRO_SECRET`: Authentication token for outgoing requests to Mac Pro (default: default-secret-change-me)
- `ENABLE_QUEUE_POLLING`: Enable file queue monitoring
- `RUST_LOG`: Logging level (debug, info, warn, error)

### Authentication

The rust-bridge-server uses two separate authentication tokens:

1. **Incoming Auth** (`BRIDGE_SECRET`): Used to authenticate requests from the KOTA MCP server
   - Default: "kota-bridge-secret-2025"
   - Used for: Requests to this server's API endpoints
   - MCP server must include: `Authorization: Bearer kota-bridge-secret-2025`

2. **Outgoing Auth** (`MAC_PRO_SECRET`): Used when making requests to the Mac Pro bridge server
   - Default: "default-secret-change-me"
   - Used for: Calls to Mac Pro's API endpoints
   - This server sends: `Authorization: Bearer default-secret-change-me`

## API Endpoints

### Health & Discovery
- `GET /health` - Server health check
- `GET /discovery` - Service capabilities and endpoints

### Communication with Mac Pro
- `POST /api/send-to-mac-pro` - Send data to Mac Pro
- `GET /api/communication-logs` - View communication logs
- `GET /api/communication-stats` - Communication statistics

### Data Reception (from Mac Pro)
- `POST /api/receive-knowledge` - Receive knowledge updates
- `POST /api/receive-context-update` - Receive context updates

### Analysis & Insights
- `POST /api/analyze-patterns` - Analyze data patterns
- `POST /api/generate-insights` - Generate proactive insights
- `GET /api/get-rust-status` - Get server status

### Data Retrieval
- `GET /api/knowledge/{category}` - Get knowledge by category
- `GET /api/context/{type}` - Get context by type

## Integration with KOTA CLI

The bridge server can be integrated with the main KOTA CLI application:

```rust
// Example integration
use rust_bridge_server::MacProClient;

// Use the Mac Pro's authentication token
let client = MacProClient::new(
    "192.168.1.100", 
    8080, 
    "default-secret-change-me".to_string()  // Mac Pro's auth token
);

// Send insights to Mac Pro
client.send_insight(
    "productivity",
    "Peak focus time detected",
    0.89,
    "kota-cli-analysis"
).await?;

// Get system status from Mac Pro
let status = client.get_system_status().await?;
```

## Logging

All communications are logged in:
- `logs/communications-YYYY-MM-DD.json` - Daily JSON logs
- `logs/all-communications.jsonl` - JSONL stream for analysis

## Network Discovery

The server can automatically discover the Mac Pro bridge server on your local network:

```rust
let mac_pro_ip = MacProClient::discover().await?;
```

## Proactive Monitoring

The server includes built-in proactive monitoring that:
- Analyzes time-based patterns
- Generates contextual insights
- Sends recommendations to Mac Pro
- Monitors connection health

## Error Handling

- Automatic retry with exponential backoff
- Graceful degradation when Mac Pro is unavailable
- Comprehensive error logging
- Connection health monitoring

## Security

- Bearer token authentication
- Local network only (no external exposure)
- Request validation and rate limiting
- Secure error handling (no sensitive data in logs)

## Development

```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Run tests
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy
```

## Architecture

```
┌─────────────────────┐              ┌─────────────────────┐
│ Mac Pro (kota_md)   │◄────────────►│ MacBook (kota-rust) │
│ Bridge Server       │   HTTP/WS    │ Bridge Server       │
│ Port: 8080          │              │ Port: 8081          │
│ TypeScript/Node.js  │              │ Rust                │
└─────────────────────┘              └─────────────────────┘
```

The Rust bridge server acts as both:
1. **Client** to Mac Pro bridge server (sends insights, queries data)
2. **Server** for receiving data and serving the KOTA CLI application

## Troubleshooting

**Connection Issues**:
- Verify Mac Pro bridge server is running on port 8080
- Check IP address in .env file
- Ensure both machines are on same network
- Check firewall settings

**Discovery Issues**:
- Run manual discovery: `let ip = MacProClient::discover().await?`
- Check mDNS/Bonjour service availability
- Try direct IP connection instead

**Queue Polling Issues**:
- Verify queue directory path exists
- Check file permissions
- Ensure network file sharing is configured
- Use HTTP polling as alternative

## License

Part of the KOTA project - see main project LICENSE file.