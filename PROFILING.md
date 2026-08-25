# RustyDB profiling

Profiling uses the workloads defined in [BENCHMARKS.md](BENCHMARKS.md). Keep the
workload parameters identical when comparing profiles.

## Allocations and lock contention

Build the benchmark with the opt-in `profiling` feature:

```console
cargo run --release --features profiling --bin rustydb-benchmark -- --workload mixed --operations 100000 --value-size 64 --concurrency 4
```

In addition to the normal benchmark fields, this reports:

- `allocation_events`: allocation and reallocation calls during the timed run;
- `allocated_bytes`: requested bytes across those events;
- `server_allocation_events` and `server_allocated_bytes`: the subset attributed
  to RESP server worker threads;
- `client_runner_allocation_events` and `client_runner_allocated_bytes`: the
  subset attributed to benchmark clients and coordination;
- `decode_*`, `command_*`, `execute_*`, and `response_*`: server allocations
  attributed to RESP decoding, typed-command construction, execution/storage,
  and response conversion/writing respectively;
- `server_other_*`: server-worker allocations outside those four phases;
- `lock_acquisitions`: timed shared-database mutex acquisitions;
- `lock_wait_nanoseconds`: total time spent waiting to acquire that mutex;
- `lock_max_wait_nanoseconds`: longest observed acquisition wait.

Setup, connection creation, and worker teardown are outside the counters and
timer. Allocation totals cover the whole benchmark process and do not represent
live memory or detect leaks. Thread-role attribution separates RESP worker
allocations from the built-in clients; small server-listener allocations remain
in the client/runner bucket. The counting allocator and lock timers add overhead,
so compare profiling runs only with other profiling runs. Normal benchmarks
compile out all of this instrumentation.

Use phase counters to identify where allocations occur, then confirm a proposed
change with repeated profiling runs using identical parameters. Allocation
counts are deterministic evidence more often than elapsed time, but they still
measure requested allocation volume rather than retained memory. Use CPU stacks
or narrower experiments before attributing a phase-level result to a specific
function.

## CPU sampling

Build the normal release runner first so allocation instrumentation does not
distort the CPU profile:

```console
cargo build --release --bin rustydb-benchmark
```

On Windows, Windows Performance Recorder can capture sampled CPU stacks. The
commands may require an elevated terminal:

```console
wpr -start CPU -filemode
target\release\rustydb-benchmark.exe --workload mixed --operations 1000000 --value-size 64 --concurrency 4
wpr -stop rustydb-cpu.etl
```

Open the ETL file in Windows Performance Analyzer and filter CPU Usage (Sampled)
to `rustydb-benchmark.exe`. On Linux, the equivalent workflow is:

```console
perf record -g -- target/release/rustydb-benchmark --workload mixed --operations 1000000 --value-size 64 --concurrency 4
perf report
```

Profiles are evidence, not performance gates. Record the raw benchmark line,
toolchain, profiler command, and the hottest stacks before proposing an
optimization. Performance changes must then use repeated uninstrumented
before-and-after benchmark runs.
