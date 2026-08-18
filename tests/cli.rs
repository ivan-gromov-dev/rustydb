use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "rustydb-cli-test-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn snapshot(&self) -> PathBuf {
        self.0.join("database.snapshot")
    }

    fn aof(&self) -> PathBuf {
        self.0.join("database.aof")
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run_cli(input: &str) -> Output {
    let directory = TestDirectory::new();
    run_cli_with_snapshot(&directory.snapshot(), &[], input)
}

fn run_cli_with_args(arguments: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rustydb"))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to start rustydb binary");

    child
        .stdin
        .take()
        .expect("child stdin must be piped")
        .write_all(input.as_bytes())
        .expect("failed to write command script");

    child
        .wait_with_output()
        .expect("failed to wait for rustydb")
}

fn run_cli_with_snapshot(snapshot: &Path, arguments: &[&str], input: &str) -> Output {
    let snapshot = snapshot.to_str().unwrap();
    let mut all_arguments = Vec::with_capacity(arguments.len() + 2);
    all_arguments.extend_from_slice(arguments);
    all_arguments.extend_from_slice(&["--snapshot", snapshot]);
    run_cli_with_args(&all_arguments, input)
}

fn run_cli_with_aof(aof: &Path, input: &str) -> Output {
    run_cli_with_args(&["--aof", aof.to_str().unwrap()], input)
}

#[test]
fn aof_replays_successful_mutations_without_recording_failed_ones() {
    let directory = TestDirectory::new();
    let aof = directory.aof();

    let first = run_cli_with_aof(
        &aof,
        "SET greeting hello\nLPUSH items first\nAPPEND items invalid\nEXIT\n",
    );
    assert!(first.status.success());
    assert!(
        String::from_utf8(first.stdout)
            .unwrap()
            .contains("ERR operation against a key holding the wrong kind of value")
    );
    let length_before_replay = fs::metadata(&aof).unwrap().len();

    let second = run_cli_with_aof(&aof, "GET greeting\nLRANGE items 0 -1\nEXIT\n");
    assert!(second.status.success());
    let stdout = String::from_utf8(second.stdout).unwrap();
    assert!(stdout.contains("db> hello\n"));
    assert!(stdout.contains("db> first\n"));
    assert_eq!(fs::metadata(&aof).unwrap().len(), length_before_replay);
}

#[test]
fn executes_a_script_and_exits_successfully() {
    let output = run_cli("SET greeting Привет мир\nGET greeting\nEXIT\n");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stderr).unwrap(), "");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        concat!(
            "Rusty DB\n",
            "Type HELP to see available commands.\n",
            "db> OK\n",
            "db> Привет мир\n",
            "db> Bye!\n",
        )
    );
}

#[test]
fn executes_list_commands() {
    let output = run_cli(
        "LPUSH tasks second item\nLPUSH tasks first item\nRPUSH tasks third item\nLLEN tasks\nLRANGE tasks 0 -1\nLPOP tasks\nRPOP tasks\nLPOP missing\nEXIT\n",
    );

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stderr).unwrap(), "");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        concat!(
            "Rusty DB\n",
            "Type HELP to see available commands.\n",
            "db> 1\n",
            "db> 2\n",
            "db> 3\n",
            "db> 3\n",
            "db> first item\n",
            "second item\n",
            "third item\n",
            "db> first item\n",
            "db> third item\n",
            "db> (nil)\n",
            "db> Bye!\n",
        )
    );
}

#[test]
fn executes_set_collection_commands() {
    let output = run_cli(
        "SADD tags zeta\nSADD tags alpha value\nSADD tags alpha value\nSISMEMBER tags alpha value\nSCARD tags\nSMEMBERS tags\nSREM tags alpha value\nSCARD tags\nEXIT\n",
    );

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stderr).unwrap(), "");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        concat!(
            "Rusty DB\n",
            "Type HELP to see available commands.\n",
            "db> 1\n",
            "db> 1\n",
            "db> 0\n",
            "db> 1\n",
            "db> 2\n",
            "db> alpha value\n",
            "zeta\n",
            "db> 1\n",
            "db> 1\n",
            "db> Bye!\n",
        )
    );
}

#[test]
fn reports_invalid_input_and_continues() {
    let output = run_cli("GET\nUNKNOWN\nEXIT\n");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stderr).unwrap(), "");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        concat!(
            "Rusty DB\n",
            "Type HELP to see available commands.\n",
            "db> ERR usage: GET key\n",
            "db> ERR unknown command: UNKNOWN\n",
            "db> Bye!\n",
        )
    );
}

#[test]
fn end_of_input_is_a_successful_shutdown() {
    let output = run_cli("");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stderr).unwrap(), "");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "Rusty DB\nType HELP to see available commands.\ndb> \n"
    );
}

#[test]
fn unknown_mode_reports_usage_and_returns_exit_code_two() {
    let output = run_cli_with_args(&["unknown"], "");

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "");
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        concat!(
            "Usage:\n",
            "  rustydb [--snapshot path] [--save-on-shutdown] [--aof path]\n",
            "  rustydb server [bind-address] [--snapshot path] [--save-on-shutdown] [--aof path]\n",
        )
    );
}

#[test]
fn extra_server_arguments_report_usage_and_return_exit_code_two() {
    let output = run_cli_with_args(&["server", "127.0.0.1:6379", "extra"], "");

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "");
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        concat!(
            "Usage:\n",
            "  rustydb [--snapshot path] [--save-on-shutdown] [--aof path]\n",
            "  rustydb server [bind-address] [--snapshot path] [--save-on-shutdown] [--aof path]\n",
        )
    );
}

#[test]
fn invalid_bind_address_returns_exit_code_one() {
    let output = run_cli_with_args(&["server", "not a valid socket address"], "");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "");
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .starts_with("Error: ")
    );
}

#[test]
fn save_persists_values_types_and_ttl_across_restarts() {
    let directory = TestDirectory::new();
    let snapshot = directory.snapshot();
    let first = run_cli_with_snapshot(
        &snapshot,
        &[],
        concat!(
            "SET greeting hello\n",
            "RPUSH tasks first\n",
            "RPUSH tasks second\n",
            "SADD tags zeta\n",
            "SADD tags alpha\n",
            "PEXPIRE greeting 600000\n",
            "SAVE\n",
            "EXIT\n",
        ),
    );

    assert!(first.status.success());
    assert!(snapshot.is_file());
    assert!(
        String::from_utf8(first.stdout)
            .unwrap()
            .contains("db> OK\ndb> Bye!\n")
    );

    let second = run_cli_with_snapshot(
        &snapshot,
        &[],
        "GET greeting\nLRANGE tasks 0 -1\nSMEMBERS tags\nPTTL greeting\nEXIT\n",
    );
    let output = String::from_utf8(second.stdout).unwrap();

    assert!(second.status.success());
    assert_eq!(String::from_utf8(second.stderr).unwrap(), "");
    assert!(output.contains("db> hello\n"));
    assert!(output.contains("db> first\nsecond\n"));
    assert!(output.contains("db> alpha\nzeta\n"));
    let ttl = output
        .lines()
        .find_map(|line| line.strip_prefix("db> ")?.parse::<i64>().ok())
        .unwrap();
    assert!((1..=600_000).contains(&ttl));
}

#[test]
fn save_on_shutdown_persists_without_save_command() {
    let directory = TestDirectory::new();
    let snapshot = directory.snapshot();
    let first = run_cli_with_snapshot(&snapshot, &["--save-on-shutdown"], "SET key value\nEXIT\n");

    assert!(first.status.success());
    assert!(snapshot.is_file());

    let second = run_cli_with_snapshot(&snapshot, &[], "GET key\nEXIT\n");
    assert!(second.status.success());
    assert!(
        String::from_utf8(second.stdout)
            .unwrap()
            .contains("db> value\n")
    );
}

#[test]
fn corrupt_snapshot_stops_startup_with_a_clear_error() {
    let directory = TestDirectory::new();
    let snapshot = directory.snapshot();
    fs::write(&snapshot, b"bad").unwrap();

    let output = run_cli_with_snapshot(&snapshot, &[], "");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "");
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "Error: snapshot is truncated\n"
    );
}
