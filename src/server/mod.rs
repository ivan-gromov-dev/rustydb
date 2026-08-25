mod listener;
#[cfg(feature = "profiling")]
pub(crate) mod profiling;
mod shutdown;

pub use listener::{
    run_server, run_server_on_listener, run_server_until, run_server_until_with_aof,
    run_server_until_with_aof_config, run_server_until_with_snapshot,
    run_server_until_with_snapshot_config,
};
pub use shutdown::Shutdown;

#[cfg(test)]
pub(crate) use listener::{SharedDatabase, execute_shared, serve_incoming};
#[cfg(feature = "profiling")]
pub use profiling::{
    LockProfile, ProfilePhase, is_server_thread, lock_profile, profiling_phase, reset_lock_profile,
};

#[cfg(test)]
mod tests;
