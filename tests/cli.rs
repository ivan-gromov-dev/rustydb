use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run_cli(input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rustydb"))
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
        "LPUSH tasks second item\nLPUSH tasks first item\nRPUSH tasks third item\nLLEN tasks\nEXIT\n",
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
