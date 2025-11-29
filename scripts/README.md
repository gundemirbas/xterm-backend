# Test scripts

This folder contains a single consolidated test runner for the xterm-backend project.

`all_tests.py` replaces the older shell and Python scripts and provides the following tests:

- `handshake_raw` - sends a raw HTTP WebSocket upgrade and prints the server response
- `handshake_timeout` - same as above but with a short read timeout
- `ws_client_test` - performs a masked-frame WebSocket client test (sends `ls\n`)
- `stress` - opens multiple WebSocket connections and keeps them alive briefly
- `reclaim` - opens `MAX_WORKERS` connections, closes them, then verifies a new connection succeeds
- `graceful` - starts (or reuses) the server, runs a client, sends SIGTERM and reports logs

Usage:

Run all tests sequentially:

```bash
python3 scripts/all_tests.py all
```

Run a single test:

```bash
python3 scripts/all_tests.py ws_client_test
```

Notes:

- The runner will automatically build and start `target/x86_64-unknown-linux-gnu/release/xterm-backend` when needed.
- The `graceful` test will reuse an existing listening server if found, otherwise it will start and stop the server.
- Tests write `server.log` in the repository root when the runner starts the server.
