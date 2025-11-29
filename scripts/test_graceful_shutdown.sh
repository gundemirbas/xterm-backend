#!/usr/bin/env bash
set -euo pipefail

# Start server in background
cd "$(dirname "$0")/.."
nohup target/x86_64-unknown-linux-gnu/release/xterm-backend > server.log 2>&1 &
SERVER_PID=$!
sleep 0.5

# Determine the actual server parent PID that is listening on port 8000.
# Prefer the process that has a listening socket on 8000 to avoid races with
# short-lived helper processes. Fall back to the directly started PID if
# detection fails.
LISTEN_PID=""
if ss -ltnp 2>/dev/null | grep -q ':8000'; then
	LISTEN_PID=$(ss -ltnp 2>/dev/null | awk '/:8000/ { if (match($0,/pid=[0-9]+,/)) { p=substr($0,RSTART+4,RLENGTH-5); print p } }' | head -n1 || true)
fi
if [ -z "$LISTEN_PID" ] && command -v lsof >/dev/null 2>&1; then
	LISTEN_PID=$(lsof -tiTCP:8000 -sTCP:LISTEN | head -n1 || true)
fi
if [ -z "$LISTEN_PID" ]; then
	echo "No listening PID detected for port 8000, using started PID $SERVER_PID"
	LISTEN_PID=$SERVER_PID
fi
echo "server pid: $LISTEN_PID"

# Start a websocket client (python script) in background to keep a session
python3 scripts/test_ws_client.py > client.out 2>&1 &
CLIENT_PID=$!
echo "client pid: $CLIENT_PID"
sleep 0.5

echo "--- server.log (head) ---"
head -n 50 server.log || true

echo "Killing server with SIGTERM"
kill -15 $SERVER_PID || true
sleep 1

echo "--- server.log (tail) ---"
tail -n 100 server.log || true

echo "Processes after SIGTERM:"
ps aux | egrep '/bin/sh|xterm-backend' | sed -n '1,200p' || true

echo "Cleaning up"
kill -9 $CLIENT_PID || true
rm -f server.pid
