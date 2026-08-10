use rustydb::{run, run_server};

const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1:6379";
const USAGE: &str = "Usage:\n  rustydb\n  rustydb server [bind-address]";

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();

    let result = match arguments.as_slice() {
        [] => run(),
        [command] if command == "server" => run_server(DEFAULT_BIND_ADDRESS),
        [command, bind_address] if command == "server" => run_server(bind_address),
        _ => {
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    };

    if let Err(err) = result {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
