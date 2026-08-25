use std::sync::atomic::{AtomicU8, Ordering};
use std::{fmt::Write as _, io::Write as _};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LogLevel {
    #[default]
    Off = 0,
    Error = 1,
    Info = 2,
    Debug = 3,
}

static LOG_LEVEL: AtomicU8 = AtomicU8::new(LogLevel::Off as u8);

pub fn set_log_level(level: LogLevel) {
    LOG_LEVEL.store(level as u8, Ordering::Relaxed);
}

pub(crate) fn enabled(level: LogLevel) -> bool {
    LOG_LEVEL.load(Ordering::Relaxed) >= level as u8
}

pub(crate) fn event(level: LogLevel, event: &str, fields: &[(&str, &str)]) {
    if !enabled(level) {
        return;
    }

    let mut record = format!("level={} event={event}", level.name());
    for (name, value) in fields {
        let _ = write!(record, " {name}={value}");
    }
    let _ = writeln!(std::io::stderr().lock(), "{record}");
}

impl LogLevel {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "off" => Some(Self::Off),
            "error" => Some(Self::Error),
            "info" => Some(Self::Info),
            "debug" => Some(Self::Debug),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Error => "error",
            Self::Info => "info",
            Self::Debug => "debug",
        }
    }
}
