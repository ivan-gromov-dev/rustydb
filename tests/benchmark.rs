use std::process::Command;

#[test]
fn benchmark_runner_records_workload_and_environment() {
    let output = Command::new(env!("CARGO_BIN_EXE_rustydb-benchmark"))
        .args([
            "--workload",
            "mixed",
            "--operations",
            "20",
            "--value-size",
            "8",
            "--concurrency",
            "2",
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    for field in [
        "workload=mixed",
        "operations=20",
        "value_size_bytes=8",
        "concurrency=2",
        "duration_seconds=",
        "operations_per_second=",
        "os=",
        "arch=",
        "logical_cpus=",
        "package_version=",
    ] {
        assert!(stdout.contains(field), "missing {field:?} in {stdout:?}");
    }
    #[cfg(feature = "profiling")]
    for field in [
        "allocation_events=",
        "allocated_bytes=",
        "server_allocation_events=",
        "server_allocated_bytes=",
        "client_runner_allocation_events=",
        "client_runner_allocated_bytes=",
        "server_other_allocation_events=",
        "server_other_allocated_bytes=",
        "decode_allocation_events=",
        "decode_allocated_bytes=",
        "command_allocation_events=",
        "command_allocated_bytes=",
        "execute_allocation_events=",
        "execute_allocated_bytes=",
        "response_allocation_events=",
        "response_allocated_bytes=",
        "lock_acquisitions=20",
        "lock_wait_nanoseconds=",
        "lock_max_wait_nanoseconds=",
    ] {
        assert!(stdout.contains(field), "missing {field:?} in {stdout:?}");
    }
}

#[test]
fn benchmark_runner_rejects_invalid_configuration() {
    let output = Command::new(env!("CARGO_BIN_EXE_rustydb-benchmark"))
        .args(["--operations", "0"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("operations must be a positive integer")
    );
}
