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

## Interpreting results

For comparisons, use an otherwise idle machine and keep the workload,
operation count, value size, concurrency, Rust toolchain, and build profile
fixed. Run each case several times and report the full distribution or a stated
summary such as the median rather than only the best result. Record operating
system, architecture, CPU availability, power mode, and virtualization changes
because they can materially alter results.

Performance changes should include repeated before-and-after measurements made
with identical parameters. Keep raw benchmark output with the change or in its
review discussion; this document intentionally does not preserve historical
results from individual machines.

CI runs a 100-operation mixed workload against the release binary as a smoke
test. It verifies that the optimized runner and end-to-end server path work, but
does not enforce a throughput threshold because shared runners are noisy.

For allocation counts, shared-database lock wait measurements, and CPU sampling
instructions, see [PROFILING.md](PROFILING.md).
