use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run_cli(input: &str) -> Output {
    run_cli_with_args(&[], input)
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
        "Usage:\n  rustydb\n  rustydb server [bind-address]\n"
    );
}

#[test]
fn extra_server_arguments_report_usage_and_return_exit_code_two() {
    let output = run_cli_with_args(&["server", "127.0.0.1:6379", "extra"], "");

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "");
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "Usage:\n  rustydb\n  rustydb server [bind-address]\n"
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
