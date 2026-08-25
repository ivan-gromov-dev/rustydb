# RustyDB benchmarks

RustyDB includes a dependency-free end-to-end benchmark runner. It starts the
real TCP server on loopback, creates the requested number of RESP2 clients, and
reports one space-separated `key=value` record suitable for saving or parsing.

Build and run benchmarks in release mode:

```console
cargo run --release --bin rustydb-benchmark -- --workload mixed --operations 100000 --value-size 64 --concurrency 4
```

The available workloads are:

- `get`: repeated `GET` requests against one preloaded key per client;
- `set`: repeated `SET` overwrites against one key per client;
- `mixed`: deterministic 80% `GET` and 20% `SET` traffic.

Every operation is one request/response round trip. Client connection and key
setup happen before timing starts. The total operation count is divided across
clients, including any remainder. Each client owns a separate key, all values
have exactly `--value-size` bytes, TCP_NODELAY is enabled, and persistence,
logging, and the key limit remain disabled. The measurement therefore includes
RESP encoding/decoding, loopback TCP, server scheduling, shared-database lock
contention, command execution, and the benchmark client's response handling.
It is not a storage-only microbenchmark.

For comparisons, use an otherwise idle machine, keep the command and toolchain
fixed, run each case several times, and report the distribution rather than
only the best result. Record changes to power mode, virtualization, and CPU
availability because they can materially alter results.

## Initial baseline

Measured on 2026-08-25 using Windows x86-64, 12 logical CPUs, Rust/Cargo 1.97.1,
and RustyDB 0.8.0. Each row is one short 20,000-operation sample and is retained
only as an initial sanity baseline; the samples are too short for optimization
claims.

| Workload | Value bytes | Concurrency | Operations/second |
| --- | ---: | ---: | ---: |
| GET | 64 | 1 | 12,883.25 |
| GET | 64 | 4 | 37,102.81 |
| SET | 64 | 1 | 17,716.99 |
| SET | 64 | 4 | 50,751.58 |
| Mixed 80/20 | 64 | 1 | 13,875.02 |
| Mixed 80/20 | 64 | 4 | 26,484.25 |
| Mixed 80/20 | 1,024 | 4 | 37,911.17 |

Do not interpret differences between these single samples as regressions or
improvements. Any performance change must include repeated before-and-after
measurements with the raw runner output and identical parameters.

CI runs a 100-operation mixed workload against the release binary as a smoke
test. It verifies that the optimized runner and end-to-end server path work, but
does not enforce a throughput threshold because shared runners are noisy.
