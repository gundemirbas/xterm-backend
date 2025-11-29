Title: test: consolidate test scripts into `scripts/all_tests.py` and remove legacy scripts

Summary:

- Add `scripts/all_tests.py`: a consolidated Python test runner that implements the existing test cases (handshake, masked-frame client, stress, reclaim workers, graceful shutdown).
- Remove legacy test scripts (`test_ws_handshake.sh`, `test_ws_handshake_fd.sh`, `test_ws_client.py`, `stress_ws_clients.py`, `test_reclaim_workers.py`, `test_graceful_shutdown.sh`, `run_all_tests.py`) and keep a single canonical runner.
- Make the runner executable and have it auto-start the release server when needed (it will build the release binary if missing).

Why:

- Simplifies test maintenance by keeping a single test runner in Python.
- Improves portability (no dependency on `nc` or `/dev/tcp` scripts) and makes tests easier to integrate into CI.

How to run:

```bash
# Build the release binary once (runner will build if missing):
cargo build --release

# Run everything:
python3 scripts/all_tests.py all

# Run a single test:
python3 scripts/all_tests.py ws_client_test
```

Notes:

- `graceful` test writes `server.log` in repo root; runner will try to reuse an existing listening server if present.
- The tests assume a Linux environment with `ss` or `lsof` available for PID detection.
