# Repo guide for AI coding agents

This is a `no_std` Rust xterm backend using raw syscalls for HTTP server, WebSocket protocol, PTY management, and epoll-based I/O. Runs on Linux/x86_64.

## Architecture & Components

**Entry & Runtime**
- `src/runtime/mod.rs`: Custom `_start` entry point with inline asm to align stack (16-byte ABI requirement) before calling `main`
- `src/main.rs`: Bootstrap only—calls `server::server_main()` then exits
- `src/runtime/panic.rs`, `src/runtime/shims.rs`: no_std panic handler and compiler builtins

**Server & Process Model**
- `src/server.rs`: Main accept loop using epoll + signalfd. Forks a worker child per WebSocket connection (fork-per-connection model). Parent reaps children via SIGCHLD and enforces `MAX_WORKERS=15` limit
- Worker lifecycle: accept → upgrade to WebSocket → spawn PTY shell → run bridge → cleanup & exit

**Memory Management**
- `src/mem/allocator.rs`: Custom `#[global_allocator]` using atomic bump arena (16 MiB) with mmap fallback. Provides `page_alloc`/`page_free` wrappers for raw buffer allocation
- `src/sys/mmap.rs`: Raw mmap/munmap syscall wrappers

**Network & Protocol**
- `src/net/http.rs`: Minimal HTTP header parser, detects WebSocket upgrade and `/term` path
- `src/net/ws/handshake.rs`: Computes Sec-WebSocket-Accept (SHA-1 + base64) and sends 101 response
- `src/net/ws/frame.rs`: WebSocket frame parser (requires masked client frames) and binary frame writer (unmasked server frames)
- `src/net/ws/crypto.rs`: In-tree SHA-1 and base64 implementations

**I/O Bridge**
- `src/loop/bridge.rs`: Epoll loop bridging WebSocket fd ↔ PTY master fd. Handles Ctrl-C detection (0x03 byte) by sending SIGINT to shell child

**System Calls**
- `src/sys/*`: Raw syscall wrappers (net, fs, epoll, pty, mmap, signal). No libc dependency

## Build & Run

```bash
# Debug build (unoptimized, with debug symbols)
cargo build
# Binary: target/x86_64-unknown-linux-gnu/debug/xterm-backend

# Release build (optimized, stripped)
cargo build --release
# Binary: target/x86_64-unknown-linux-gnu/release/xterm-backend

# Run server (listens on port 8000)
nohup target/x86_64-unknown-linux-gnu/release/xterm-backend > server.log 2>&1 &
tail -f server.log

# Kill running server
pkill -f xterm-backend
```

## Testing & Debugging

**Test Scripts** (`scripts/`)
- `test_ws_client.py`: Full WebSocket handshake + frame exchange test—use this first to reproduce issues deterministically
- `test_ws_handshake.sh`: Quick handshake validation
- `stress_ws_clients.py`: Concurrent connection stress test
- `test_graceful_shutdown.sh`: SIGTERM/SIGINT handling
- `test_reclaim_workers.py`: Worker reaping validation

**Debugging Patterns**
1. Reproduce with `./scripts/test_ws_client.py` before testing in browser
2. Check `server.log` for fork/accept/error messages
3. For protocol issues: inspect `upgrade_to_websocket` (handshake) and `parse_and_unmask_frames` (frame parsing)
4. LLDB workflow: `sudo sysctl -w kernel.yama.ptrace_scope=0` then attach to parent or child PID

**Common Issues**
- Stack corruption → ensure `_start` aligns stack (see `src/runtime/mod.rs`)
- Handshake fails → verify SHA-1/base64 in `src/net/ws/crypto.rs`
- Frame parsing errors → clients MUST send masked frames; check `frame.rs` mask logic

## Project Conventions

**Error Handling**
- Use `Result<T, &'static str>` with literal error messages: `Err("descriptive msg")`
- Keep errors terse but meaningful (e.g., `"no key"`, `"fork failed"`, `"ws read"`)

**Memory & Buffers**
- Prefer stack buffers or `page_alloc`/`page_free` for large allocations
- Functions take `&[u8]` or `&mut [u8]` slices—no allocations in hot paths
- Bridge uses 64 KiB mmap'd buffers for WebSocket/PTY I/O

**WebSocket Protocol**
- Clients MUST mask frames (RFC 6455 requirement)—parser rejects unmasked: `Err("client not masked")`
- Server sends unmasked binary frames via `write_binary_frame`
- Frame parsing mutates input buffer in-place for efficiency

**Logging**
- Use `server::log(b"...")` for stdout (fd 1) writes
- No format macros—use `itoa::Buffer` for numbers or manual byte arrays

## Integration Points

**Browser Client**
- `assets/terminal.html`: Connects to `ws://<host>:8000/term`
- Sends masked text frames, expects unmasked binary frames from server

**External Dependencies**
- `itoa` crate (int-to-string formatting only, no_std compatible)
- All other functionality (HTTP, WebSocket, crypto, syscalls) is in-tree

## Critical Files for Common Tasks

**Add/modify protocol logic**: `src/net/ws/{handshake,frame}.rs`  
**Change I/O behavior**: `src/loop/bridge.rs`  
**Fix memory issues**: `src/mem/allocator.rs`, `src/runtime/mod.rs`  
**System call changes**: `src/sys/{net,fs,epoll,pty,mmap}.rs`  
**Worker limits/reaping**: `src/server.rs` (see `MAX_WORKERS`, SIGCHLD handler)
