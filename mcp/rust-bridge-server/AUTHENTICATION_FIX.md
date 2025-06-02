# Authentication Fix Summary

## Problem
The rust-bridge-server was using the same authentication token (`BRIDGE_SECRET`) for both:
1. Authenticating incoming requests from the KOTA MCP server
2. Authenticating outgoing requests to the Mac Pro bridge server

This caused authentication failures when talking to the Mac Pro, which expects "default-secret-change-me".

## Solution
Separated the authentication tokens into two distinct environment variables:

1. **`BRIDGE_SECRET`** (default: "kota-bridge-secret-2025")
   - Used for authenticating incoming requests from KOTA MCP server
   - Applied in `server.rs` auth middleware

2. **`MAC_PRO_SECRET`** (default: "default-secret-change-me")
   - Used for authenticating outgoing requests to Mac Pro bridge server
   - Applied in `mac_pro_client.rs` when making API calls

## Changes Made

### 1. Updated `src/main.rs`
- Added `mac_pro_secret` field to Configuration struct
- Modified `load_configuration()` to read `MAC_PRO_SECRET` env var
- Pass `mac_pro_secret` to MacProClient instead of `shared_secret`

### 2. Updated `README.md`
- Documented the two separate authentication tokens
- Added authentication section explaining the purpose of each token
- Updated example code to show correct token usage

### 3. Created `.env.example`
- Added template configuration file with both auth tokens
- Includes helpful comments explaining each token's purpose

## Usage

To run the rust-bridge-server with proper authentication:

```bash
# Set environment variables (or create .env file)
export BRIDGE_SECRET="kota-bridge-secret-2025"    # For incoming from MCP
export MAC_PRO_SECRET="default-secret-change-me"  # For outgoing to Mac Pro

# Run the server
cargo run
```

The server will now:
- Accept requests from MCP with "kota-bridge-secret-2025"
- Send requests to Mac Pro with "default-secret-change-me"