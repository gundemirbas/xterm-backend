xterm-backend
==============

Tiny Rust no_std xterm backend using raw syscalls. This repo contains a small HTTP server that upgrades `/term` to a WebSocket PTY-connected shell.

Key points
- Concurrency: server forks a worker per accepted WebSocket `/term` connection.
- Max workers: `MAX_WORKERS = 15` (see `src/main.rs`) — additional connections receive `503 Service Unavailable`.
- Child cleanup: workers exit when their PTY/bridge terminates; parent reaps children using `signalfd` + `waitpid`.

Quick commands
- Build (release):

```bash
cargo build --release
```

- Run server in background (logs to `server.log`):

```bash
./target/x86_64-unknown-linux-gnu/release/xterm-backend &> server.log & echo $! > server.pid
```

Test scripts
- `scripts/stress_ws_clients.py`: opens 16 concurrent WebSocket handshakes and sends short keepalive frames.
- `scripts/test_reclaim_workers.py`: opens `MAX_WORKERS` connections, closes them, waits 1s, then tries to open a new connection to verify worker reclamation.

Run tests

```bash
python3 scripts/stress_ws_clients.py
python3 scripts/test_reclaim_workers.py
```

If you want me to commit the changes and push a branch, tell me which commit message to use (or I will use a short default).