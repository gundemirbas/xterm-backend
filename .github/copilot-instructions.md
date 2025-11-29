# xterm-backend — AI Agent Guide

A `no_std` Rust WebSocket terminal server using raw Linux syscalls. No libc. Custom runtime with `_start` entry point.

## Architecture Overview

**Process Model**: Fork-per-connection. Parent accepts on port 8000, forks a child per WebSocket upgrade, child spawns PTY shell and bridges I/O until disconnect.

**Key Constraint**: ALL `unsafe` code isolated in `src/runtime/*`. Rest of codebase uses safe wrappers.

**Module Layout**:
- `src/main.rs` — Bootstrap with accept loop (epoll + signalfd), enforces MAX_WORKERS=15
- `src/server/mod.rs` — Helper functions: `setup_listener()`, `handle_signal_event()`, `handle_listener_event()`
- `src/server/bridge.rs` — Worker's epoll loop bridging WebSocket fd ↔ PTY master fd
- `src/runtime/*` — Custom allocator (16 MiB bump arena), syscall wrappers, panic/shim, `_start` entry
- `src/sys/*` — Safe syscall facades (net, fs, epoll, pty, mmap, signal)
- `src/net/*` — HTTP parser, WebSocket handshake/framing, SHA-1/base64 crypto
- `src/pty/pty.rs` — PTY spawn with prctl(PR_SET_PDEATHSIG) and setsid

**Data Flow**: Accept → fork → child closes parent fds → WebSocket handshake → spawn PTY (`/bin/sh`) → bridge loop (epoll on ws_fd + pty_fd) → detect Ctrl-C (0x03) → kill shell → exit child

## Essential Commands

```bash
# Format and lint (MUST pass before commit)
cargo fmt && cargo clippy -- -D warnings

# Release build (optimized, stripped)
cargo build --release

# Run server
nohup target/x86_64-unknown-linux-gnu/release/xterm-backend > server.log 2>&1 &
tail -f server.log

# Run full test suite (canonical integration tests)
python3 scripts/all_tests.py all

# Kill server
pkill -f xterm-backend
```

**Test suite** (`scripts/all_tests.py`) runs: handshake_raw, handshake_timeout, ws_client_test, stress (16 concurrent), reclaim (worker reaping), graceful (SIGTERM shutdown).

## Critical Conventions

**Safety**: `unsafe` ONLY in `src/runtime/*`. If adding syscall, put raw wrapper in `runtime/syscall.rs` and safe facade in `src/sys/*.rs`.

**Error Handling**: `Result<T, &'static str>` with terse literals: `Err("fork")`, `Err("ws read")`. No allocations in error paths.

**Memory**: 
- Large buffers: `runtime::allocator::page_alloc(len)` / `page_free(ptr, len)` (returns `*mut u8`)
- Hot path functions take `&[u8]` / `&mut [u8]` slices
- Bridge uses 64 KiB mmap'd buffers for WebSocket/PTY I/O

**Logging**: `crate::server::log(b"...")` writes to fd 1. Use `itoa::Buffer` for numbers (no format macros).

**WebSocket**: Clients MUST send masked frames (RFC 6455). Parser rejects unmasked: `Err("client not masked")`. Server sends unmasked binary frames.

## Common Tasks

**Add WebSocket feature**: Edit `src/net/ws/frame.rs` (parser) or `handshake.rs` (upgrade). Keep crypto in-tree.

**Change bridge behavior**: Edit `src/server/bridge.rs` epoll loop. Buffer allocation via `page_alloc`/`page_free`.

**Modify accept/reap logic**: Edit `src/server/mod.rs` helpers. Parent uses `wait_any_nohang()` on SIGCHLD.

**Add syscall**: Raw wrapper in `src/runtime/syscall.rs`, safe facade in `src/sys/*.rs` (e.g., `sys/pty.rs`).

**Debug protocol issue**: Reproduce with `python3 scripts/test_ws_client.py` before browser. Check handshake in `upgrade_to_websocket()` or frame parsing in `parse_and_unmask_frames()`.

## Key Files to Scan

- `src/runtime/mod.rs` — Custom `_start` with stack alignment (16-byte ABI), `exit_now` wrapper
- `src/runtime/allocator.rs` — Global allocator: atomic bump arena (16 MiB) + mmap fallback
- `src/server/bridge.rs` — Epoll loop, Ctrl-C detection (0x03 → SIGINT to child)
- `src/net/ws/frame.rs` — Frame parser (enforces client masking), binary frame writer
- `src/sys/pty.rs` — `fork()`, `execve()`, `prctl_set_pdeathsig()`, `tcsetpgrp()`

## Debugging Tips

- **Stack corruption**: Verify `_start` aligns stack in `src/runtime/mod.rs`
- **Handshake fails**: Check SHA-1/base64 in `src/net/ws/crypto.rs`
- **Frame parsing**: Clients must mask; verify mask bit in `frame.rs`
- **LLDB attach**: `sudo sysctl -w kernel.yama.ptrace_scope=0`, then attach to PID

## CI/Build Notes

- `Cargo.lock` is gitignored (not tracked per `.gitignore`)
- Profile: `panic = "abort"`, `lto = true`, `opt-level = "z"`, `strip = true`
- Only external dep: `itoa` (no_std int-to-string)
- CI should run: `cargo fmt -- --check`, `cargo clippy -- -D warnings`, `cargo build --release`, `python3 scripts/all_tests.py all`
