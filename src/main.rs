use std::io;
use std::path::PathBuf;

use rustydb::{DEFAULT_SNAPSHOT_PATH, Shutdown, run_server_until_with_snapshot, run_with_snapshot};

const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1:6379";
const USAGE: &str = concat!(
    "Usage:\n",
    "  rustydb [--snapshot path] [--save-on-shutdown]\n",
    "  rustydb server [bind-address] [--snapshot path] [--save-on-shutdown]"
);

struct Configuration {
    mode: Mode,
    snapshot_path: PathBuf,
    save_on_shutdown: bool,
}

enum Mode {
    Interactive,
    Server(String),
}

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let configuration = match parse_arguments(&arguments) {
        Ok(configuration) => configuration,
        Err(()) => {
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    };

    let result = match configuration.mode {
        Mode::Interactive => {
            run_with_snapshot(configuration.snapshot_path, configuration.save_on_shutdown)
        }
        Mode::Server(bind_address) => run_server_with_ctrl_c(
            &bind_address,
            configuration.snapshot_path,
            configuration.save_on_shutdown,
        ),
    };

    if let Err(err) = result {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn parse_arguments(arguments: &[String]) -> Result<Configuration, ()> {
    let mut index = 0;
    let mode = if arguments
        .first()
        .is_some_and(|argument| argument == "server")
    {
        index += 1;
        let bind_address = match arguments.get(index) {
            Some(argument) if !argument.starts_with("--") => {
                index += 1;
                argument.clone()
            }
            _ => DEFAULT_BIND_ADDRESS.to_owned(),
        };
        Mode::Server(bind_address)
    } else {
        Mode::Interactive
    };

    let mut snapshot_path = PathBuf::from(DEFAULT_SNAPSHOT_PATH);
    let mut save_on_shutdown = false;

    while let Some(argument) = arguments.get(index) {
        match argument.as_str() {
            "--snapshot" => {
                index += 1;
                let path = arguments
                    .get(index)
                    .filter(|path| !path.is_empty())
                    .ok_or(())?;
                snapshot_path = PathBuf::from(path);
            }
            "--save-on-shutdown" => save_on_shutdown = true,
            _ => return Err(()),
        }
        index += 1;
    }

    Ok(Configuration {
        mode,
        snapshot_path,
        save_on_shutdown,
    })
}

fn run_server_with_ctrl_c(
    bind_address: &str,
    snapshot_path: PathBuf,
    save_on_shutdown: bool,
) -> io::Result<()> {
    let shutdown = Shutdown::default();
    let signal_shutdown = shutdown.clone();

    ctrlc::set_handler(move || signal_shutdown.request()).map_err(io::Error::other)?;

    run_server_until_with_snapshot(bind_address, shutdown, snapshot_path, save_on_shutdown)
}
