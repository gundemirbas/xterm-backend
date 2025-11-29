````instructions
# Repo guide for AI coding agents — xterm-backend

Concise guide to get productive quickly. Focus on what is special about this repo (no_std runtime, raw syscalls, fork-per-connection server) and where to look for common tasks.

````instructions
# xterm-backend — concise AI agent guide

This repo is a no_std Rust backend using raw syscalls. Key constraints: custom `_start` entry, all `unsafe` code lives under `src/runtime/`, and the server uses a fork-per-connection model.

Quick facts
- Entry/runtime: `src/runtime/mod.rs` defines `_start` and stack alignment; keep `#![no_std]` + `#![no_main]` in `src/main.rs`.
- Server: parent uses epoll + signalfd and forks one worker per WS connection. Bridge lives in `src/server/bridge.rs`.

Files to scan first
- `src/runtime/*` — allocator, syscall shims, panic handlers (unsafe only here).
- `src/server/mod.rs`, `src/server/bridge.rs` — accept loop helpers and PTY↔WS bridge.
- `src/net/http.rs`, `src/net/ws/{handshake.rs,frame.rs,crypto.rs}` — HTTP parsing, WS handshake, masking rules.
- `scripts/all_tests.py` — canonical integration test runner (builds, starts server, runs tests).

Essential commands
- Format & lint: `cargo fmt` && `cargo clippy -- -D warnings`
- Build (release): `cargo build --release`
- Run server: `nohup target/x86_64-unknown-linux-gnu/release/xterm-backend > server.log 2>&1 &`
- Run tests: `python3 scripts/all_tests.py all` (recommended CI target)
- Quick WS check: `python3 scripts/test_ws_client.py`

Conventions (must follow)
- Keep all `unsafe` in `src/runtime/` only.
- Logging: use `crate::server::log(b"...")` (no format macros). Use `itoa::Buffer` for numbers.
- Error style: `Result<T, &'static str>` with short literal messages (e.g., `Err("fork")`).
- Memory: prefer `page_alloc`/`page_free` for large buffers; functions operate on slices.
- WebSocket: client frames MUST be masked (parser rejects unmasked). Server sends unmasked binary frames.

Where to change things
- Protocol logic: `src/net/ws/*` (handshake, frame parsing).
- Bridge/IO: `src/server/bridge.rs` (epoll loop, buffer allocation).
- Accept/reap logic: `src/server/mod.rs`.
- Low-level/syscall changes: `src/runtime/*` and `src/sys/*` only.

CI suggestion (minimal): run `cargo fmt -- --check`, `cargo clippy -- -D warnings`, `cargo build --release`, then `python3 scripts/all_tests.py all` on push/PR.

If you want, I can add the GitHub Actions YAML implementing the CI steps above.
````
- LLDB/dev attach: `sudo sysctl -w kernel.yama.ptrace_scope=0` then attach to the relevant PID.

## Tests & CI guidance
- The repo contains `scripts/all_tests.py` which is the canonical runner used in development and CI. CI should:
  - Run `cargo fmt -- --check`
  - Run `cargo clippy -- -D warnings`
  - Build `cargo build --release`
  - Run `python3 scripts/all_tests.py all`
- Do not add `Cargo.lock` to git (this project ignores it). See `.gitignore`.

## Common refactors and where to change things
- Add protocol features: `src/net/ws/*` (keep parsing deterministic and in-tree crypto).
- Change bridge behavior or buffers: `src/server/bridge.rs` (epoll loop and buffer allocation via `page_alloc`).
- Change accept/reap logic: `src/server/mod.rs` (helpers used by `main.rs`).
- Unsafe/syscall changes: `src/runtime/*` and `src/sys/*` only.

## Example idioms (copyable)
- Read a signalfd and reap children (see `handle_signal_event` in `src/server/mod.rs`).
- Upgrade handshake: call `net::ws::upgrade_to_websocket(fd, req)` then use `bridge::run_bridge(ws.fd, p.master_fd, p.child_pid)`.

If any of this is unclear or you want the CI workflow I outlined committed to `.github/workflows/`, tell me which runner steps to include and I’ll add the YAML.
````
# Repo guide for AI coding agents

This is a `no_std` Rust xterm backend using raw syscalls for HTTP server, WebSocket protocol, PTY management, and epoll-based I/O. Runs on Linux/x86_64.

## Architecture & Components

**Entry & Runtime** (all `unsafe` isolated here)
- `src/runtime/mod.rs`: Custom `_start` entry point with inline asm to align stack (16-byte ABI requirement), `exit_now` syscall wrapper
- `src/runtime/syscall.rs`: Raw syscall wrappers with safe checked variants (moved from sys/)
- `src/runtime/allocator.rs`: Global allocator with atomic bump arena (moved from mem/)
- `src/runtime/util.rs`: Pointer-to-slice conversion helpers (moved from sys/)
- `src/runtime/panic.rs`, `src/runtime/shims.rs`: no_std panic handler and compiler builtins
- `src/main.rs`: Bootstrap only—calls `server::server_main()` then exits

**Server & Process Model**
- `src/server.rs`: Main accept loop using epoll + signalfd. Forks a worker child per WebSocket connection (fork-per-connection model). Parent reaps children via SIGCHLD and enforces `MAX_WORKERS=15` limit
- Worker lifecycle: accept → upgrade to WebSocket → spawn PTY shell → run bridge → cleanup & exit

**Memory Management**
- `src/runtime/allocator.rs`: Custom `#[global_allocator]` using atomic bump arena (16 MiB) with mmap fallback. Provides `page_alloc`/`page_free` wrappers for raw buffer allocation
- `src/sys/mmap.rs`: Safe mmap/munmap wrappers using runtime syscalls

**Network & Protocol**
- `src/net/http.rs`: Minimal HTTP header parser, detects WebSocket upgrade and `/term` path
- `src/net/ws/handshake.rs`: Computes Sec-WebSocket-Accept (SHA-1 + base64) and sends 101 response
- `src/net/ws/frame.rs`: WebSocket frame parser (requires masked client frames) and binary frame writer (unmasked server frames)
- `src/net/ws/crypto.rs`: In-tree SHA-1 and base64 implementations

**I/O Bridge**
- `src/server/bridge.rs`: Epoll loop bridging WebSocket fd ↔ PTY master fd. Handles Ctrl-C detection (0x03 byte) by sending SIGINT to shell child

**System Calls**
- `src/sys/*`: Safe syscall wrappers (net, fs, epoll, pty, mmap, signal) using runtime syscalls. No libc dependency

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
- `all_tests.py`: Consolidated Python test runner that builds/starts the server and runs handshake, ws client, stress, reclaim and graceful tests.

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

**Safety & Code Quality (CRITICAL)**
- `unsafe` code is ONLY allowed in `src/runtime/` directory—all low-level operations (syscalls, allocator, stack alignment) are isolated there
- Code MUST pass `cargo fmt` without changes and `cargo clippy` without warnings or errors
- Rest of codebase uses safe abstractions provided by runtime module

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
**Fix memory issues**: `src/runtime/allocator.rs`, `src/runtime/mod.rs`  
**System call changes**: `src/sys/{net,fs,epoll,pty,mmap}.rs` or `src/runtime/syscall.rs`  
**Worker limits/reaping**: `src/server.rs` (see `MAX_WORKERS`, SIGCHLD handler)
