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

### Initial instrumentation sample

Measured on 2026-08-25 using the same Windows x86-64, 12-logical-CPU environment
as the initial benchmark baseline. These are single 20,000-operation mixed
samples with 64-byte values, retained as a sanity check rather than an
optimization claim.

| Concurrency | Server events | Server bytes | Client/runner events | Client/runner bytes | Average lock wait | Maximum lock wait |
| ----------: | ------------: | -----------: | -------------------: | ------------------: | ----------------: | ----------------: |
|           1 |       164,000 |    5,392,000 |              184,000 |           4,576,000 |          41.41 ns |          1,100 ns |
|           4 |       164,000 |    5,392,000 |              184,000 |           4,576,000 |          84.89 ns |         35,700 ns |

The allocation totals are unchanged because the workload and operation count
are identical. The longer lock waits at concurrency 4 establish contention as
measurable, but repeated samples and CPU stacks are required before attributing
it to a specific bottleneck.

Single-client command profiles with the same operation count and value size
show where the server-side volume is concentrated:

| Workload    | Server events/operation | Server bytes/operation |
| ----------- | ----------------------: | ---------------------: |
| GET         |                     7.0 |                  220.0 |
| SET         |                    13.0 |                  468.0 |
| Mixed 80/20 |                     8.2 |                  269.6 |

SET therefore allocates almost twice as often as GET in the current end-to-end
path. Phase attribution narrows that difference further:

| Workload | Decode events/bytes per op | Command events/bytes per op | Execute events/bytes per op | Response events/bytes per op |
| --- | ---: | ---: | ---: | ---: |
| GET | 3 / 78 | 3 / 78 | 1 / 64 | 0 / 0 |
| SET | 4 / 174 | 4 / 142 | 4 / 150 | 1 / 2 |

The extra SET allocation volume is distributed across the whole request path,
not isolated to storage: decoding, command construction, and execution each add
three or four allocation events per operation. Allocation stacks or narrower
microbenchmarks are still needed before choosing a safe optimization target.

### First measured optimization

The command phase showed that every request allocated an uppercase copy of its
command name. `GET` and `SET` now use allocation-free ASCII case-insensitive
matching before the general parser. With otherwise identical 20,000-operation,
64-byte, single-client profiling runs, the command phase changed as follows:

| Workload | Before events/bytes per op | After events/bytes per op |
| --- | ---: | ---: |
| GET | 3 / 78 | 2 / 75 |
| SET | 4 / 142 | 3 / 139 |

This removes exactly one allocation and three requested bytes per operation
without changing case-insensitive command behavior. The end-to-end allocation
count falls from 7 to 6 server events per GET and from 13 to 12 per SET.

Five ordinary release runs before and after the change produced these raw
operations/second samples:

| Workload | Before | After | Median before | Median after |
| --- | --- | --- | ---: | ---: |
| GET | 13099.42, 10760.34, 13161.00, 12883.63, 8650.01 | 12985.74, 13045.72, 12610.40, 12487.07, 12934.34 | 12883.63 | 12934.34 |
| SET | 16176.62, 16521.38, 14532.51, 16383.19, 16206.50 | 17510.98, 16654.37, 17877.41, 17956.37, 17600.80 | 16206.50 | 17600.80 |

The GET throughput change is within run-to-run noise. The SET median is higher
in this short sample, but the runs were not interleaved and are insufficient to
claim a stable throughput improvement. The allocation reduction is the direct,
repeatable result.

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
