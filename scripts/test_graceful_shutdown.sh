#!/usr/bin/env bash
set -euo pipefail

# Start server in background
cd "$(dirname "$0")/.."
nohup target/x86_64-unknown-linux-gnu/release/xterm-backend > server.log 2>&1 &
SERVER_PID=$!
echo "server pid: $SERVER_PID"
sleep 0.5

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
