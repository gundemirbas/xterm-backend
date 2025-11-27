#!/usr/bin/env bash
# WebSocket handshake test using /dev/tcp (no nc required)
set -euo pipefail

REQ=$'GET /term HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n'

echo "Connecting to localhost:8000..."
exec 3<>/dev/tcp/localhost/8000
printf "%s" "$REQ" >&3
 # read response with timeout (requires coreutils `timeout`)
 timeout 2 cat <&3 || true
exec 3>&-

echo "Done."
