mod aof;
mod app;
mod command;
mod config;
mod database;
mod executor;
mod line_protocol;
mod line_session;
mod logging;
mod output;
mod resp;
mod resp_session;
mod server;
mod snapshot;
mod storage;

pub use app::{
    run, run_with_aof, run_with_aof_config, run_with_snapshot, run_with_snapshot_config,
};
pub use config::MemoryConfig;
pub use logging::{LogLevel, set_log_level};
#[cfg(feature = "profiling")]
pub use server::{
    LockProfile, ProfilePhase, is_server_thread, lock_profile, profiling_phase, reset_lock_profile,
};
pub use server::{
    Shutdown, run_server, run_server_on_listener, run_server_until, run_server_until_with_aof,
    run_server_until_with_aof_config, run_server_until_with_snapshot,
    run_server_until_with_snapshot_config,
};

pub const DEFAULT_SNAPSHOT_PATH: &str = "rustydb.snapshot";
