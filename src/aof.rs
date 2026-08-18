use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use std::time::SystemTime;
use std::{fmt, mem};

use crate::command::Command;

const MAGIC: &[u8; 8] = b"RUSTAOF\0";
const VERSION: u16 = 1;
const MAX_RECORD_LENGTH: usize = 512 * 1024 * 1024;
const MAX_ARGUMENTS: usize = 2_000_001;
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Debug)]
pub(crate) enum AofError {
    Io(io::Error),
    InvalidMagic,
    UnsupportedVersion(u16),
    Truncated,
    ChecksumMismatch,
    InvalidRecord,
    InvalidCommand(String),
    LimitExceeded,
    TimeOutOfRange,
}

impl fmt::Display for AofError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "AOF I/O error: {error}"),
            Self::InvalidMagic => write!(formatter, "invalid AOF magic"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported AOF version: {version}")
            }
            Self::Truncated => write!(formatter, "AOF is truncated"),
            Self::ChecksumMismatch => write!(formatter, "AOF record checksum does not match"),
            Self::InvalidRecord => write!(formatter, "AOF contains an invalid record"),
            Self::InvalidCommand(error) => {
                write!(formatter, "AOF contains an invalid command: {error}")
            }
            Self::LimitExceeded => write!(formatter, "AOF record exceeds the supported limit"),
            Self::TimeOutOfRange => {
                write!(formatter, "system time is outside the supported AOF range")
            }
        }
    }
}

impl std::error::Error for AofError {}

impl From<io::Error> for AofError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

pub(crate) struct Aof {
    file: File,
}

impl Aof {
    pub(crate) fn open(path: &Path) -> Result<(Self, Vec<Command>), AofError> {
        let mut file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(path)?;
        let length = file.metadata()?.len();
        let commands = if length == 0 {
            file.write_all(MAGIC)?;
            file.write_all(&VERSION.to_le_bytes())?;
            file.sync_all()?;
            Vec::new()
        } else {
            read_commands(&mut file, SystemTime::now())?
        };
        Ok((Self { file }, commands))
    }

    pub(crate) fn append(&mut self, arguments: &[Vec<u8>]) -> Result<(), AofError> {
        let timestamp = unix_millis(SystemTime::now())?;
        let payload = encode_payload(timestamp, arguments)?;
        let length = u64::try_from(payload.len()).map_err(|_| AofError::LimitExceeded)?;
        self.file.write_all(&length.to_le_bytes())?;
        self.file.write_all(&payload)?;
        self.file.write_all(&checksum(&payload).to_le_bytes())?;
        self.file.sync_all()?;
        Ok(())
    }
}

fn read_commands(reader: &mut impl Read, now: SystemTime) -> Result<Vec<Command>, AofError> {
    let mut magic = [0; MAGIC.len()];
    read_exact(reader, &mut magic)?;
    if &magic != MAGIC {
        return Err(AofError::InvalidMagic);
    }
    let version = read_u16(reader)?;
    if version != VERSION {
        return Err(AofError::UnsupportedVersion(version));
    }

    let now_millis = unix_millis(now)?;
    let mut commands = Vec::new();
    while let Some(length) = read_optional_u64(reader)? {
        let length = usize::try_from(length).map_err(|_| AofError::LimitExceeded)?;
        if length > MAX_RECORD_LENGTH {
            return Err(AofError::LimitExceeded);
        }
        let mut payload = vec![0; length];
        read_exact(reader, &mut payload)?;
        let expected = read_u64(reader)?;
        if checksum(&payload) != expected {
            return Err(AofError::ChecksumMismatch);
        }
        commands.push(decode_payload(&payload, now_millis)?);
    }
    Ok(commands)
}

fn encode_payload(timestamp: u64, arguments: &[Vec<u8>]) -> Result<Vec<u8>, AofError> {
    if arguments.is_empty() || arguments.len() > MAX_ARGUMENTS {
        return Err(AofError::LimitExceeded);
    }
    let mut payload = Vec::new();
    payload.extend_from_slice(&timestamp.to_le_bytes());
    payload.extend_from_slice(
        &u64::try_from(arguments.len())
            .map_err(|_| AofError::LimitExceeded)?
            .to_le_bytes(),
    );
    for argument in arguments {
        payload.extend_from_slice(
            &u64::try_from(argument.len())
                .map_err(|_| AofError::LimitExceeded)?
                .to_le_bytes(),
        );
        payload.extend_from_slice(argument);
        if payload.len() > MAX_RECORD_LENGTH {
            return Err(AofError::LimitExceeded);
        }
    }
    Ok(payload)
}

fn decode_payload(payload: &[u8], now_millis: u64) -> Result<Command, AofError> {
    let mut cursor = payload;
    let timestamp = take_u64(&mut cursor)?;
    let count = usize::try_from(take_u64(&mut cursor)?).map_err(|_| AofError::LimitExceeded)?;
    if count == 0 || count > MAX_ARGUMENTS {
        return Err(AofError::LimitExceeded);
    }
    let mut arguments = Vec::new();
    arguments
        .try_reserve_exact(count)
        .map_err(|_| AofError::LimitExceeded)?;
    for _ in 0..count {
        let length =
            usize::try_from(take_u64(&mut cursor)?).map_err(|_| AofError::LimitExceeded)?;
        if length > cursor.len() {
            return Err(AofError::InvalidRecord);
        }
        let (argument, remaining) = cursor.split_at(length);
        arguments.push(argument);
        cursor = remaining;
    }
    if !cursor.is_empty() {
        return Err(AofError::InvalidRecord);
    }
    let mut command = Command::from_bytes(&arguments)
        .map_err(|error| AofError::InvalidCommand(error.to_string()))?;
    adjust_expiration(&mut command, now_millis.saturating_sub(timestamp));
    Ok(command)
}

fn adjust_expiration(command: &mut Command, elapsed_millis: u64) {
    match command {
        Command::Expire { key, seconds } => {
            let remaining = seconds.saturating_mul(1_000).saturating_sub(elapsed_millis);
            *command = Command::PExpire {
                key: mem::take(key),
                milliseconds: remaining,
            };
        }
        Command::PExpire { milliseconds, .. } => {
            *milliseconds = milliseconds.saturating_sub(elapsed_millis)
        }
        _ => {}
    }
}

fn unix_millis(time: SystemTime) -> Result<u64, AofError> {
    let duration = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|_| AofError::TimeOutOfRange)?;
    u64::try_from(duration.as_millis()).map_err(|_| AofError::TimeOutOfRange)
}

fn checksum(bytes: &[u8]) -> u64 {
    let mut value = FNV_OFFSET_BASIS;
    for byte in bytes {
        value ^= u64::from(*byte);
        value = value.wrapping_mul(FNV_PRIME);
    }
    value
}

fn read_optional_u64(reader: &mut impl Read) -> Result<Option<u64>, AofError> {
    let mut bytes = [0; size_of::<u64>()];
    let mut read = 0;
    while read < bytes.len() {
        match reader.read(&mut bytes[read..])? {
            0 if read == 0 => return Ok(None),
            0 => return Err(AofError::Truncated),
            count => read += count,
        }
    }
    Ok(Some(u64::from_le_bytes(bytes)))
}

fn read_u16(reader: &mut impl Read) -> Result<u16, AofError> {
    let mut bytes = [0; size_of::<u16>()];
    read_exact(reader, &mut bytes)?;
    Ok(u16::from_le_bytes(bytes))
}
fn read_u64(reader: &mut impl Read) -> Result<u64, AofError> {
    let mut bytes = [0; size_of::<u64>()];
    read_exact(reader, &mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}
fn take_u64(bytes: &mut &[u8]) -> Result<u64, AofError> {
    if bytes.len() < size_of::<u64>() {
        return Err(AofError::InvalidRecord);
    }
    let (value, remaining) = bytes.split_at(size_of::<u64>());
    *bytes = remaining;
    Ok(u64::from_le_bytes(
        value.try_into().map_err(|_| AofError::InvalidRecord)?,
    ))
}
fn read_exact(reader: &mut impl Read, bytes: &mut [u8]) -> Result<(), AofError> {
    reader.read_exact(bytes).map_err(|error| {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            AofError::Truncated
        } else {
            AofError::Io(error)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_round_trip_preserves_binary_arguments() {
        let payload = encode_payload(
            1_000,
            &[b"SET".to_vec(), b"k\0".to_vec(), b"v\xff".to_vec()],
        )
        .unwrap();
        assert_eq!(
            decode_payload(&payload, 1_000).unwrap(),
            Command::Set {
                key: b"k\0".to_vec(),
                value: b"v\xff".to_vec()
            }
        );
    }

    #[test]
    fn replay_reduces_expiration_by_downtime() {
        let payload = encode_payload(
            1_000,
            &[b"PEXPIRE".to_vec(), b"key".to_vec(), b"500".to_vec()],
        )
        .unwrap();
        assert_eq!(
            decode_payload(&payload, 1_200).unwrap(),
            Command::PExpire {
                key: b"key".to_vec(),
                milliseconds: 300
            }
        );
    }
}
