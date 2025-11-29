````instructions
# Repo guide for AI coding agents — xterm-backend

Concise guide to get productive quickly. Focus on what is special about this repo (no_std runtime, raw syscalls, fork-per-connection server) and where to look for common tasks.

## Big picture
- Runtime: `no_std` with a custom entry in `src/runtime/mod.rs` (`_start`) — stack is aligned in asm and `exit_now` performs SYS_exit. Do NOT remove `#![no_main]` from `src/main.rs`.
- Server model: parent process uses epoll + signalfd to accept connections and forks one worker per WebSocket connection. Worker lifecycle: accept → upgrade → spawn PTY → bridge (WebSocket ↔ PTY) → cleanup.
- Module layout:
  - `src/runtime/` — all unsafe, allocator, syscall shims, panic handlers (isolate unsafe here).
  - `src/sys/` — syscall-based wrappers: `net`, `fs`, `epoll`, `pty`, `mmap`, `signal`.
  - `src/net/` — HTTP and WebSocket logic (`http.rs`, `ws/*`).
  - `src/pty/` — PTY spawn & helpers.
  - `src/server/` — server helpers and `bridge.rs` (bridge loop lives here).

## Key files to open first
- `src/runtime/mod.rs` — custom `_start`, runtime exports, and reasons for `no_std`/`no_main`.
- `src/runtime/allocator.rs` — `page_alloc` / `page_free` semantics and mmap fallback.
- `src/sys/` — how raw syscalls are wrapped; used everywhere.
- `src/server/mod.rs` and `src/server/bridge.rs` — accept loop helpers (used by `main.rs`) and the epoll bridge between WS fd and PTY master.
- `src/net/ws/{handshake.rs,frame.rs,crypto.rs}` — handshake, masking rules, and in-tree SHA1/base64.
- `scripts/all_tests.py` — canonical test runner (builds/releases and runs integration tests).

## Project-specific conventions (do not deviate)
- Safety: ALL `unsafe` must live in `src/runtime/`. Changes outside that folder should avoid `unsafe`.
- no_std/no_main: Keep `#![no_std]` and `#![no_main]` in `src/main.rs`. The runtime provides `_start` and configures ABI/stack.
- Error types: prefer `Result<T, &'static str>` with short literal messages (e.g., `Err("fork")`).
- Logging: use `crate::server::log(b"...")` and `itoa::Buffer` for integers—no format macros.
- WebSocket: client frames MUST be masked. Parser returns `Err("client not masked")` for unmasked frames; server writes unmasked binary frames.
- Memory: prefer `page_alloc` for large buffers; functions operate on slices (`&[u8]` / `&mut [u8]`).

## Build, test, debug (the fast path)
- Format & lint: `cargo fmt` and `cargo clippy -- -D warnings`.
- Build (release): `cargo build --release` → produced binary: `target/x86_64-unknown-linux-gnu/release/xterm-backend`.
- Run server (background):
  - `nohup target/x86_64-unknown-linux-gnu/release/xterm-backend > server.log 2>&1 &`
  - `tail -f server.log`
- Run the integration test suite (recommended): `python3 scripts/all_tests.py all` (it builds, starts the server, runs tests, then stops it).
- Quick protocol check: `python3 ./scripts/test_ws_client.py`.

## Debugging notes
- Use `server.log` output for fork/accept/reap messages; tests write this file.
- For protocol bugs: review `src/net/ws/handshake.rs` and `src/net/ws/frame.rs` (masking, payload length handling).
- For low-level crashes or stack issues: verify `_start` stack alignment in `src/runtime/mod.rs` (inline asm) and allocator correctness.
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
