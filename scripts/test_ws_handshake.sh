#!/usr/bin/env bash
# Simple WebSocket handshake test against localhost:8000
set -euo pipefail

REQ=$(cat <<'EOF'
GET /term HTTP/1.1
Host: localhost
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==
Sec-WebSocket-Version: 13

EOF
)

echo "Sending handshake request to localhost:8000..."
printf "%b" "$REQ" | nc localhost 8000 -w 3 || true

echo "Done."
