use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::{fmt, mem, process};

use crate::command::Command;
use crate::storage::{SnapshotEntry, SnapshotValue};

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
    InvalidPath,
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
            Self::InvalidPath => write!(formatter, "AOF path must name a file"),
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
    file: Option<File>,
    path: PathBuf,
}

impl Aof {
    pub(crate) fn open(path: &Path) -> Result<(Self, Vec<Command>), AofError> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        let length = file.metadata()?.len();
        let commands = if length == 0 {
            file.write_all(MAGIC)?;
            file.write_all(&VERSION.to_le_bytes())?;
            file.sync_all()?;
            Vec::new()
        } else {
            file.seek(SeekFrom::Start(0))?;
            let replay = read_commands(&mut file, SystemTime::now())?;
            if replay.truncated_tail {
                file.set_len(replay.valid_length)?;
                file.sync_all()?;
            }
            replay.commands
        };
        file.seek(SeekFrom::End(0))?;
        Ok((
            Self {
                file: Some(file),
                path: path.to_owned(),
            },
            commands,
        ))
    }

    pub(crate) fn append(&mut self, arguments: &[Vec<u8>]) -> Result<(), AofError> {
        let timestamp = unix_millis(SystemTime::now())?;
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| AofError::Io(io::Error::other("AOF file is temporarily unavailable")))?;
        write_record(file, timestamp, arguments)?;
        file.sync_all()?;
        Ok(())
    }

    pub(crate) fn rewrite(
        &mut self,
        entries: &[SnapshotEntry],
        wall_now: SystemTime,
    ) -> Result<(), AofError> {
        let timestamp = unix_millis(wall_now)?;
        let (temporary_path, mut temporary) = create_temporary_file(&self.path)?;
        let result = (|| {
            temporary.write_all(MAGIC)?;
            temporary.write_all(&VERSION.to_le_bytes())?;
            for entry in entries {
                write_entry(&mut temporary, entry, timestamp)?;
            }
            temporary.sync_all()?;
            drop(temporary);

            drop(self.file.take());
            if let Err(error) = fs::rename(&temporary_path, &self.path) {
                self.file = Some(open_existing(&self.path)?);
                return Err(AofError::Io(error));
            }
            sync_parent(&self.path)?;
            self.file = Some(open_existing(&self.path)?);
            Ok(())
        })();

        if result.is_err() {
            let _ = fs::remove_file(temporary_path);
            if self.file.is_none() {
                self.file = open_existing(&self.path).ok();
            }
        }
        result
    }
}

fn write_record(
    writer: &mut impl Write,
    timestamp: u64,
    arguments: &[Vec<u8>],
) -> Result<(), AofError> {
    let payload = encode_payload(timestamp, arguments)?;
    let length = u64::try_from(payload.len()).map_err(|_| AofError::LimitExceeded)?;
    writer.write_all(&length.to_le_bytes())?;
    writer.write_all(&payload)?;
    writer.write_all(&checksum(&payload).to_le_bytes())?;
    Ok(())
}

fn write_entry(
    writer: &mut impl Write,
    entry: &SnapshotEntry,
    timestamp: u64,
) -> Result<(), AofError> {
    match &entry.value {
        SnapshotValue::String(value) => write_record(
            writer,
            timestamp,
            &[b"SET".to_vec(), entry.key.clone(), value.clone()],
        )?,
        SnapshotValue::List(values) => {
            for value in values {
                write_record(
                    writer,
                    timestamp,
                    &[b"RPUSH".to_vec(), entry.key.clone(), value.clone()],
                )?;
            }
        }
        SnapshotValue::Set(values) => {
            for value in values {
                write_record(
                    writer,
                    timestamp,
                    &[b"SADD".to_vec(), entry.key.clone(), value.clone()],
                )?;
            }
        }
    }

    if let Some(expires_at) = entry.expires_at_unix_millis {
        let remaining = expires_at.saturating_sub(timestamp);
        write_record(
            writer,
            timestamp,
            &[
                b"PEXPIRE".to_vec(),
                entry.key.clone(),
                remaining.to_string().into_bytes(),
            ],
        )?;
    }
    Ok(())
}

fn create_temporary_file(path: &Path) -> Result<(PathBuf, File), AofError> {
    let Some(file_name) = path.file_name() else {
        return Err(AofError::InvalidPath);
    };
    let parent = path
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    for attempt in 0..1_000_u16 {
        let mut name = file_name.to_os_string();
        name.push(format!(".rewrite.{}.{attempt}", process::id()));
        let temporary_path = parent.join(name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
        {
            Ok(file) => return Ok((temporary_path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(AofError::Io(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a temporary AOF file",
    )))
}

fn open_existing(path: &Path) -> Result<File, AofError> {
    let mut file = OpenOptions::new().read(true).write(true).open(path)?;
    file.seek(SeekFrom::End(0))?;
    Ok(file)
}

#[cfg(unix)]
fn sync_parent(path: &Path) -> Result<(), AofError> {
    let parent = path
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    File::open(parent)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_parent(_path: &Path) -> Result<(), AofError> {
    Ok(())
}

struct Replay {
    commands: Vec<Command>,
    valid_length: u64,
    truncated_tail: bool,
}

fn read_commands(reader: &mut (impl Read + Seek), now: SystemTime) -> Result<Replay, AofError> {
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
    loop {
        let record_start = reader.stream_position()?;
        let Some(length) = read_record_length(reader)? else {
            let end = reader.stream_position()?;
            return Ok(Replay {
                commands,
                valid_length: record_start,
                truncated_tail: end != record_start,
            });
        };
        let length = usize::try_from(length).map_err(|_| AofError::LimitExceeded)?;
        if length > MAX_RECORD_LENGTH {
            return Err(AofError::LimitExceeded);
        }
        let mut payload = vec![0; length];
        if !read_record_part(reader, &mut payload)? {
            return Ok(Replay {
                commands,
                valid_length: record_start,
                truncated_tail: true,
            });
        }
        let mut checksum_bytes = [0; size_of::<u64>()];
        if !read_record_part(reader, &mut checksum_bytes)? {
            return Ok(Replay {
                commands,
                valid_length: record_start,
                truncated_tail: true,
            });
        }
        let expected = u64::from_le_bytes(checksum_bytes);
        if checksum(&payload) != expected {
            return Err(AofError::ChecksumMismatch);
        }
        commands.push(decode_payload(&payload, now_millis)?);
    }
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
        Command::SetAdvanced { expiration, .. } => match expiration {
            Some(crate::command::SetExpiration::Seconds(seconds)) => {
                *expiration = Some(crate::command::SetExpiration::Milliseconds(
                    seconds.saturating_mul(1_000).saturating_sub(elapsed_millis),
                ));
            }
            Some(crate::command::SetExpiration::Milliseconds(milliseconds)) => {
                *milliseconds = milliseconds.saturating_sub(elapsed_millis);
            }
            _ => {}
        },
        Command::GetEx { expiration, .. } => match expiration {
            Some(crate::command::GetExExpiration::Seconds(seconds)) => {
                *expiration = Some(crate::command::GetExExpiration::Milliseconds(
                    seconds.saturating_mul(1_000).saturating_sub(elapsed_millis),
                ));
            }
            Some(crate::command::GetExExpiration::Milliseconds(milliseconds)) => {
                *milliseconds = milliseconds.saturating_sub(elapsed_millis);
            }
            _ => {}
        },
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

fn read_record_length(reader: &mut impl Read) -> Result<Option<u64>, AofError> {
    let mut bytes = [0; size_of::<u64>()];
    let mut read = 0;
    while read < bytes.len() {
        match reader.read(&mut bytes[read..])? {
            0 if read == 0 => return Ok(None),
            0 => return Ok(None),
            count => read += count,
        }
    }
    Ok(Some(u64::from_le_bytes(bytes)))
}

fn read_record_part(reader: &mut impl Read, bytes: &mut [u8]) -> Result<bool, AofError> {
    match reader.read_exact(bytes) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => Ok(false),
        Err(error) => Err(AofError::Io(error)),
    }
}

fn read_u16(reader: &mut impl Read) -> Result<u16, AofError> {
    let mut bytes = [0; size_of::<u16>()];
    read_exact(reader, &mut bytes)?;
    Ok(u16::from_le_bytes(bytes))
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
    use std::io::Cursor;

    use super::*;

    fn record(timestamp: u64, arguments: &[Vec<u8>]) -> Vec<u8> {
        let payload = encode_payload(timestamp, arguments).unwrap();
        raw_record(&payload)
    }

    fn raw_record(payload: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(payload.len() as u64).to_le_bytes());
        bytes.extend_from_slice(payload);
        bytes.extend_from_slice(&checksum(payload).to_le_bytes());
        bytes
    }

    fn log(records: &[Vec<u8>]) -> Vec<u8> {
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(&VERSION.to_le_bytes());
        for record in records {
            bytes.extend_from_slice(record);
        }
        bytes
    }

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

    #[test]
    fn replay_reduces_relative_set_expiration_by_downtime() {
        let payload = encode_payload(
            1_000,
            &[
                b"SET".to_vec(),
                b"key".to_vec(),
                b"value".to_vec(),
                b"PX".to_vec(),
                b"500".to_vec(),
            ],
        )
        .unwrap();
        assert_eq!(
            decode_payload(&payload, 1_200).unwrap(),
            Command::SetAdvanced {
                key: b"key".to_vec(),
                value: b"value".to_vec(),
                condition: None,
                return_old: false,
                expiration: Some(crate::command::SetExpiration::Milliseconds(300)),
            }
        );
    }

    #[test]
    fn replay_reduces_relative_getex_expiration_by_downtime() {
        let payload = encode_payload(
            1_000,
            &[
                b"GETEX".to_vec(),
                b"key".to_vec(),
                b"EX".to_vec(),
                b"2".to_vec(),
            ],
        )
        .unwrap();
        assert_eq!(
            decode_payload(&payload, 1_500).unwrap(),
            Command::GetEx {
                key: b"key".to_vec(),
                expiration: Some(crate::command::GetExExpiration::Milliseconds(1_500)),
            }
        );
    }

    #[test]
    fn every_partial_final_record_is_ignored_at_the_previous_boundary() {
        let first = record(1_000, &[b"SET".to_vec(), b"one".to_vec(), b"1".to_vec()]);
        let second = record(1_000, &[b"SET".to_vec(), b"two".to_vec(), b"2".to_vec()]);
        let complete = log(&[first.clone(), second]);
        let second_start = MAGIC.len() + size_of::<u16>() + first.len();

        for cut in second_start + 1..complete.len() {
            let mut cursor = Cursor::new(&complete[..cut]);
            let replay = read_commands(&mut cursor, SystemTime::UNIX_EPOCH).unwrap();
            assert!(replay.truncated_tail, "cut at byte {cut}");
            assert_eq!(replay.valid_length, second_start as u64);
            assert_eq!(replay.commands.len(), 1);
        }
    }

    #[test]
    fn checksum_corruption_is_not_treated_as_a_truncated_tail() {
        let mut bytes = log(&[record(
            1_000,
            &[b"SET".to_vec(), b"key".to_vec(), b"value".to_vec()],
        )]);
        let last = bytes.len() - 1;
        bytes[last] ^= 1;

        assert!(matches!(
            read_commands(&mut Cursor::new(bytes), SystemTime::UNIX_EPOCH),
            Err(AofError::ChecksumMismatch)
        ));
    }

    #[test]
    fn rejects_invalid_and_truncated_headers() {
        let mut invalid_magic = log(&[]);
        invalid_magic[0] ^= 1;
        assert!(matches!(
            read_commands(&mut Cursor::new(invalid_magic), SystemTime::UNIX_EPOCH),
            Err(AofError::InvalidMagic)
        ));

        let mut unsupported = log(&[]);
        unsupported[MAGIC.len()..MAGIC.len() + size_of::<u16>()]
            .copy_from_slice(&2_u16.to_le_bytes());
        assert!(matches!(
            read_commands(&mut Cursor::new(unsupported), SystemTime::UNIX_EPOCH),
            Err(AofError::UnsupportedVersion(2))
        ));

        assert!(matches!(
            read_commands(
                &mut Cursor::new(&MAGIC[..MAGIC.len() - 1]),
                SystemTime::UNIX_EPOCH
            ),
            Err(AofError::Truncated)
        ));
    }

    #[test]
    fn rejects_complete_malformed_and_unknown_command_records() {
        let mut payload = 1_000_u64.to_le_bytes().to_vec();
        payload.extend_from_slice(&0_u64.to_le_bytes());
        let malformed = log(&[raw_record(&payload)]);
        assert!(matches!(
            read_commands(&mut Cursor::new(malformed), SystemTime::UNIX_EPOCH),
            Err(AofError::LimitExceeded)
        ));

        let unknown = log(&[record(1_000, &[b"UNKNOWN".to_vec()])]);
        assert!(matches!(
            read_commands(&mut Cursor::new(unknown), SystemTime::UNIX_EPOCH),
            Err(AofError::InvalidCommand(_))
        ));
    }
}
