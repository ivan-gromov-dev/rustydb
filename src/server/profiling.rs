use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static ACQUISITIONS: AtomicU64 = AtomicU64::new(0);
static WAIT_NANOSECONDS: AtomicU64 = AtomicU64::new(0);
static MAX_WAIT_NANOSECONDS: AtomicU64 = AtomicU64::new(0);

thread_local! {
    static PHASE: Cell<ProfilePhase> = const { Cell::new(ProfilePhase::ClientRunner) };
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProfilePhase {
    #[default]
    ClientRunner,
    ServerOther,
    Decode,
    Command,
    Execute,
    Response,
}

impl ProfilePhase {
    pub const COUNT: usize = 6;

    pub const fn index(self) -> usize {
        self as usize
    }
}

pub(crate) struct ProfileScope(ProfilePhase);

impl Drop for ProfileScope {
    fn drop(&mut self) {
        PHASE.set(self.0);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LockProfile {
    pub acquisitions: u64,
    pub wait_nanoseconds: u64,
    pub max_wait_nanoseconds: u64,
}

pub fn reset_lock_profile() {
    ACQUISITIONS.store(0, Ordering::Relaxed);
    WAIT_NANOSECONDS.store(0, Ordering::Relaxed);
    MAX_WAIT_NANOSECONDS.store(0, Ordering::Relaxed);
}

pub fn lock_profile() -> LockProfile {
    LockProfile {
        acquisitions: ACQUISITIONS.load(Ordering::Relaxed),
        wait_nanoseconds: WAIT_NANOSECONDS.load(Ordering::Relaxed),
        max_wait_nanoseconds: MAX_WAIT_NANOSECONDS.load(Ordering::Relaxed),
    }
}

pub fn is_server_thread() -> bool {
    profiling_phase() != ProfilePhase::ClientRunner
}

pub fn profiling_phase() -> ProfilePhase {
    PHASE.get()
}

pub(crate) fn mark_server_thread() {
    PHASE.set(ProfilePhase::ServerOther);
}

pub(crate) fn profile_scope(phase: ProfilePhase) -> ProfileScope {
    ProfileScope(PHASE.replace(phase))
}

pub(crate) fn record_lock_wait(wait: Duration) {
    let nanoseconds = u64::try_from(wait.as_nanos()).unwrap_or(u64::MAX);
    ACQUISITIONS.fetch_add(1, Ordering::Relaxed);
    WAIT_NANOSECONDS.fetch_add(nanoseconds, Ordering::Relaxed);
    MAX_WAIT_NANOSECONDS.fetch_max(nanoseconds, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_scope_restores_the_previous_phase() {
        assert_eq!(profiling_phase(), ProfilePhase::ClientRunner);
        mark_server_thread();
        assert!(is_server_thread());
        {
            let _decode = profile_scope(ProfilePhase::Decode);
            assert_eq!(profiling_phase(), ProfilePhase::Decode);
            {
                let _execute = profile_scope(ProfilePhase::Execute);
                assert_eq!(profiling_phase(), ProfilePhase::Execute);
            }
            assert_eq!(profiling_phase(), ProfilePhase::Decode);
        }
        assert_eq!(profiling_phase(), ProfilePhase::ServerOther);
    }
}
