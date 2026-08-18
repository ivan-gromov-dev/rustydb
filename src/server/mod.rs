mod listener;
mod shutdown;

pub use listener::{
    run_server, run_server_on_listener, run_server_until, run_server_until_with_aof,
    run_server_until_with_snapshot,
};
pub use shutdown::Shutdown;

#[cfg(test)]
pub(crate) use listener::{SharedDatabase, execute_shared, serve_incoming};

#[cfg(test)]
mod tests;
