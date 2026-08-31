use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::{fmt, process};

use crate::storage::{InMemoryStore, SnapshotDataError, SnapshotEntry, SnapshotValue};

const MAGIC: &[u8; 8] = b"RUSTYDB\0";
const FORMAT_VERSION: u16 = 2;
const MIN_SUPPORTED_VERSION: u16 = 1;
const MAX_ENTRIES: usize = 1_000_000;
const MAX_COLLECTION_VALUES: usize = 1_000_000;
const MAX_BLOB_LENGTH: usize = 512 * 1024 * 1024;
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Debug)]
pub(crate) enum SnapshotError {
    Io(io::Error),
    InvalidMagic,
    UnsupportedVersion(u16),
    Truncated,
    ChecksumMismatch,
    TrailingData,
    InvalidValueType(u8),
    InvalidExpirationTag(u8),
    LimitExceeded(&'static str),
    InvalidPath,
    NotConfigured,
    InvalidData(SnapshotDataError),
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "snapshot I/O error: {error}"),
            Self::InvalidMagic => write!(formatter, "invalid snapshot magic"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported snapshot version: {version}")
            }
            Self::Truncated => write!(formatter, "snapshot is truncated"),
            Self::ChecksumMismatch => write!(formatter, "snapshot checksum does not match"),
            Self::TrailingData => write!(formatter, "snapshot has trailing data"),
            Self::InvalidValueType(value_type) => {
                write!(formatter, "snapshot has invalid value type: {value_type}")
            }
            Self::InvalidExpirationTag(tag) => {
                write!(formatter, "snapshot has invalid expiration tag: {tag}")
            }
            Self::LimitExceeded(limit) => write!(formatter, "snapshot exceeds {limit} limit"),
            Self::InvalidPath => write!(formatter, "snapshot path must name a file"),
            Self::NotConfigured => write!(formatter, "snapshot path is not configured"),
            Self::InvalidData(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for SnapshotError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for SnapshotError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<SnapshotDataError> for SnapshotError {
    fn from(error: SnapshotDataError) -> Self {
        Self::InvalidData(error)
    }
}

pub(crate) fn load(path: &Path, store: &mut InMemoryStore) -> Result<bool, SnapshotError> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };

    read_snapshot(BufReader::new(file), store, SystemTime::now())?;
    Ok(true)
}

pub(crate) fn save(path: &Path, store: &mut InMemoryStore) -> Result<(), SnapshotError> {
    save_with_replace(path, store, SystemTime::now(), |from, to| {
        fs::rename(from, to)
    })
}

fn save_with_replace(
    path: &Path,
    store: &mut InMemoryStore,
    wall_now: SystemTime,
    replace: impl FnOnce(&Path, &Path) -> io::Result<()>,
) -> Result<(), SnapshotError> {
    let entries = store.snapshot_entries(wall_now)?;
    validate_entries(&entries)?;
    let (temporary_path, file) = create_temporary_file(path)?;

    let result = (|| {
        let mut writer = BufWriter::new(file);
        write_snapshot(&mut writer, &entries)?;
        writer.flush()?;
        writer.get_ref().sync_all()?;
        drop(writer);

        replace(&temporary_path, path)?;
        sync_parent(path)?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }

    result
}

fn create_temporary_file(path: &Path) -> Result<(PathBuf, File), SnapshotError> {
    let Some(file_name) = path.file_name() else {
        return Err(SnapshotError::InvalidPath);
    };
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    for attempt in 0..1_000_u16 {
        let mut temporary_name = file_name.to_os_string();
        temporary_name.push(format!(".tmp.{}.{attempt}", process::id()));
        let temporary_path = parent.join(temporary_name);

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

    Err(SnapshotError::Io(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a temporary snapshot file",
    )))
}

#[cfg(unix)]
fn sync_parent(path: &Path) -> Result<(), SnapshotError> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    File::open(parent)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_parent(_path: &Path) -> Result<(), SnapshotError> {
    Ok(())
}

fn write_snapshot(mut writer: impl Write, entries: &[SnapshotEntry]) -> Result<(), SnapshotError> {
    writer.write_all(MAGIC)?;
    write_u16(&mut writer, FORMAT_VERSION)?;

    let checksum = {
        let mut checksummed = ChecksummedWriter::new(&mut writer);
        write_u64(&mut checksummed, length_as_u64(entries.len())?)?;

        for entry in entries {
            write_blob(&mut checksummed, &entry.key)?;
            match &entry.value {
                SnapshotValue::String(value) => {
                    checksummed.write_all(&[0])?;
                    write_blob(&mut checksummed, value)?;
                }
                SnapshotValue::List(values) => {
                    checksummed.write_all(&[1])?;
                    write_collection(&mut checksummed, values)?;
                }
                SnapshotValue::Set(values) => {
                    checksummed.write_all(&[2])?;
                    write_collection(&mut checksummed, values)?;
                }
                SnapshotValue::Hash(values) => {
                    checksummed.write_all(&[3])?;
                    write_u64(&mut checksummed, length_as_u64(values.len())?)?;
                    for (field, value) in values {
                        write_blob(&mut checksummed, field)?;
                        write_blob(&mut checksummed, value)?;
                    }
                }
            }

            match entry.expires_at_unix_millis {
                None => checksummed.write_all(&[0])?,
                Some(milliseconds) => {
                    checksummed.write_all(&[1])?;
                    write_u64(&mut checksummed, milliseconds)?;
                }
            }
        }

        checksummed.checksum()
    };
    write_u64(&mut writer, checksum)?;
    Ok(())
}

fn read_snapshot(
    mut reader: impl Read,
    store: &mut InMemoryStore,
    wall_now: SystemTime,
) -> Result<(), SnapshotError> {
    let mut magic = [0; MAGIC.len()];
    read_exact(&mut reader, &mut magic)?;
    if &magic != MAGIC {
        return Err(SnapshotError::InvalidMagic);
    }

    let version = read_u16(&mut reader)?;
    if !(MIN_SUPPORTED_VERSION..=FORMAT_VERSION).contains(&version) {
        return Err(SnapshotError::UnsupportedVersion(version));
    }

    let (entries, checksum) = {
        let mut checksummed = ChecksummedReader::new(&mut reader);
        let entry_count = read_length(&mut checksummed, MAX_ENTRIES, "entry count")?;
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(entry_count)
            .map_err(|_| SnapshotDataError::AllocationFailed)?;

        for _ in 0..entry_count {
            let key = read_blob(&mut checksummed)?;
            let value_type = read_byte(&mut checksummed)?;
            let value = match value_type {
                0 => SnapshotValue::String(read_blob(&mut checksummed)?),
                1 => SnapshotValue::List(read_collection(&mut checksummed)?),
                2 => SnapshotValue::Set(read_collection(&mut checksummed)?),
                3 if version >= 2 => {
                    let length =
                        read_length(&mut checksummed, MAX_COLLECTION_VALUES, "collection length")?;
                    let mut values = Vec::new();
                    values
                        .try_reserve_exact(length)
                        .map_err(|_| SnapshotDataError::AllocationFailed)?;
                    for _ in 0..length {
                        values.push((read_blob(&mut checksummed)?, read_blob(&mut checksummed)?));
                    }
                    SnapshotValue::Hash(values)
                }
                value_type => return Err(SnapshotError::InvalidValueType(value_type)),
            };

            let expires_at_unix_millis = match read_byte(&mut checksummed)? {
                0 => None,
                1 => Some(read_u64(&mut checksummed)?),
                tag => return Err(SnapshotError::InvalidExpirationTag(tag)),
            };

            entries.push(SnapshotEntry {
                key,
                value,
                expires_at_unix_millis,
            });
        }

        (entries, checksummed.checksum())
    };
    let expected_checksum = read_u64(&mut reader)?;
    if checksum != expected_checksum {
        return Err(SnapshotError::ChecksumMismatch);
    }

    let mut trailing = [0];
    match reader.read(&mut trailing) {
        Ok(0) => {}
        Ok(_) => return Err(SnapshotError::TrailingData),
        Err(error) => return Err(error.into()),
    }

    store.restore_snapshot(entries, wall_now)?;
    Ok(())
}

fn validate_entries(entries: &[SnapshotEntry]) -> Result<(), SnapshotError> {
    ensure_limit(entries.len(), MAX_ENTRIES, "entry count")?;
    for entry in entries {
        ensure_limit(entry.key.len(), MAX_BLOB_LENGTH, "blob length")?;
        match &entry.value {
            SnapshotValue::String(value) => {
                ensure_limit(value.len(), MAX_BLOB_LENGTH, "blob length")?;
            }
            SnapshotValue::List(values) | SnapshotValue::Set(values) => {
                ensure_limit(values.len(), MAX_COLLECTION_VALUES, "collection length")?;
                for value in values {
                    ensure_limit(value.len(), MAX_BLOB_LENGTH, "blob length")?;
                }
            }
            SnapshotValue::Hash(values) => {
                ensure_limit(values.len(), MAX_COLLECTION_VALUES, "collection length")?;
                for (field, value) in values {
                    ensure_limit(field.len(), MAX_BLOB_LENGTH, "blob length")?;
                    ensure_limit(value.len(), MAX_BLOB_LENGTH, "blob length")?;
                }
            }
        }
    }
    Ok(())
}

fn write_collection(writer: &mut impl Write, values: &[Vec<u8>]) -> Result<(), SnapshotError> {
    write_u64(writer, length_as_u64(values.len())?)?;
    for value in values {
        write_blob(writer, value)?;
    }
    Ok(())
}

fn read_collection(reader: &mut impl Read) -> Result<Vec<Vec<u8>>, SnapshotError> {
    let length = read_length(reader, MAX_COLLECTION_VALUES, "collection length")?;
    let mut values = Vec::new();
    values
        .try_reserve_exact(length)
        .map_err(|_| SnapshotDataError::AllocationFailed)?;
    for _ in 0..length {
        values.push(read_blob(reader)?);
    }
    Ok(values)
}

fn write_blob(writer: &mut impl Write, value: &[u8]) -> Result<(), SnapshotError> {
    write_u64(writer, length_as_u64(value.len())?)?;
    writer.write_all(value)?;
    Ok(())
}

fn read_blob(reader: &mut impl Read) -> Result<Vec<u8>, SnapshotError> {
    let length = read_length(reader, MAX_BLOB_LENGTH, "blob length")?;
    let mut value = Vec::new();
    value
        .try_reserve_exact(length)
        .map_err(|_| SnapshotDataError::AllocationFailed)?;
    value.resize(length, 0);
    read_exact(reader, &mut value)?;
    Ok(value)
}

fn read_length(
    reader: &mut impl Read,
    maximum: usize,
    name: &'static str,
) -> Result<usize, SnapshotError> {
    let length =
        usize::try_from(read_u64(reader)?).map_err(|_| SnapshotError::LimitExceeded(name))?;
    ensure_limit(length, maximum, name)?;
    Ok(length)
}

fn ensure_limit(length: usize, maximum: usize, name: &'static str) -> Result<(), SnapshotError> {
    if length > maximum {
        Err(SnapshotError::LimitExceeded(name))
    } else {
        Ok(())
    }
}

fn length_as_u64(length: usize) -> Result<u64, SnapshotError> {
    u64::try_from(length).map_err(|_| SnapshotError::LimitExceeded("platform length"))
}

fn read_byte(reader: &mut impl Read) -> Result<u8, SnapshotError> {
    let mut byte = [0];
    read_exact(reader, &mut byte)?;
    Ok(byte[0])
}

fn write_u16(writer: &mut impl Write, value: u16) -> Result<(), SnapshotError> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

fn read_u16(reader: &mut impl Read) -> Result<u16, SnapshotError> {
    let mut bytes = [0; size_of::<u16>()];
    read_exact(reader, &mut bytes)?;
    Ok(u16::from_le_bytes(bytes))
}

fn write_u64(writer: &mut impl Write, value: u64) -> Result<(), SnapshotError> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

fn read_u64(reader: &mut impl Read) -> Result<u64, SnapshotError> {
    let mut bytes = [0; size_of::<u64>()];
    read_exact(reader, &mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

fn read_exact(reader: &mut impl Read, buffer: &mut [u8]) -> Result<(), SnapshotError> {
    reader.read_exact(buffer).map_err(|error| {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            SnapshotError::Truncated
        } else {
            SnapshotError::Io(error)
        }
    })
}

struct ChecksummedWriter<W> {
    inner: W,
    checksum: u64,
}

impl<W> ChecksummedWriter<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            checksum: FNV_OFFSET_BASIS,
        }
    }

    fn checksum(&self) -> u64 {
        self.checksum
    }
}

impl<W: Write> Write for ChecksummedWriter<W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(buffer)?;
        update_checksum(&mut self.checksum, &buffer[..written]);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

struct ChecksummedReader<R> {
    inner: R,
    checksum: u64,
}

impl<R> ChecksummedReader<R> {
    fn new(inner: R) -> Self {
        Self {
            inner,
            checksum: FNV_OFFSET_BASIS,
        }
    }

    fn checksum(&self) -> u64 {
        self.checksum
    }
}

impl<R: Read> Read for ChecksummedReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(buffer)?;
        update_checksum(&mut self.checksum, &buffer[..read]);
        Ok(read)
    }
}

fn update_checksum(checksum: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *checksum ^= u64::from(*byte);
        *checksum = checksum.wrapping_mul(FNV_PRIME);
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    use super::*;

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "rustydb-snapshot-test-{}-{sequence}",
                process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn encoded(entries: &[SnapshotEntry]) -> Vec<u8> {
        let mut bytes = Vec::new();
        write_snapshot(&mut bytes, entries).unwrap();
        bytes
    }

    fn sample_entries(expires_at_unix_millis: Option<u64>) -> Vec<SnapshotEntry> {
        vec![
            SnapshotEntry {
                key: b"string\0key".to_vec(),
                value: SnapshotValue::String(b"binary\0\xff".to_vec()),
                expires_at_unix_millis,
            },
            SnapshotEntry {
                key: b"list".to_vec(),
                value: SnapshotValue::List(vec![b"first".to_vec(), b"second".to_vec()]),
                expires_at_unix_millis: None,
            },
            SnapshotEntry {
                key: b"set".to_vec(),
                value: SnapshotValue::Set(vec![b"alpha".to_vec(), b"zeta".to_vec()]),
                expires_at_unix_millis: None,
            },
            SnapshotEntry {
                key: b"hash".to_vec(),
                value: SnapshotValue::Hash(vec![
                    (b"alpha".to_vec(), b"one".to_vec()),
                    (b"zeta".to_vec(), b"two".to_vec()),
                ]),
                expires_at_unix_millis: None,
            },
        ]
    }

    #[test]
    fn round_trip_preserves_binary_values_types_and_expiration() {
        let wall_now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        let expires_at = 1_060_000;
        let entries = sample_entries(Some(expires_at));
        let mut store = InMemoryStore::new();

        read_snapshot(Cursor::new(encoded(&entries)), &mut store, wall_now).unwrap();

        let actual = store.snapshot_entries(wall_now).unwrap();
        assert_eq!(actual.len(), 4);
        assert_eq!(actual[0], entries[3]);
        assert_eq!(actual[1], entries[1]);
        assert_eq!(actual[2], entries[2]);
        assert_eq!(actual[3].key, entries[0].key);
        assert_eq!(actual[3].value, entries[0].value);
        assert!(matches!(
            actual[3].expires_at_unix_millis,
            Some(1_059_999..=1_060_000)
        ));
    }

    #[test]
    fn keys_expired_while_stopped_are_not_restored() {
        let entries = sample_entries(Some(10_000));
        let mut store = InMemoryStore::new();
        let load_time = SystemTime::UNIX_EPOCH + Duration::from_secs(11);

        read_snapshot(Cursor::new(encoded(&entries)), &mut store, load_time).unwrap();

        assert_eq!(
            store.keys(),
            vec![b"hash".to_vec(), b"list".to_vec(), b"set".to_vec()]
        );
    }

    #[test]
    fn reports_bad_magic_unsupported_versions_truncation_and_checksum_corruption() {
        let entries = sample_entries(None);
        let valid = encoded(&entries);
        let wall_now = SystemTime::UNIX_EPOCH;

        let mut bad_magic = valid.clone();
        bad_magic[0] ^= 1;
        let mut store = InMemoryStore::new();
        assert!(matches!(
            read_snapshot(Cursor::new(bad_magic), &mut store, wall_now),
            Err(SnapshotError::InvalidMagic)
        ));

        let mut unsupported = valid.clone();
        unsupported[MAGIC.len()..MAGIC.len() + 2].copy_from_slice(&3_u16.to_le_bytes());
        assert!(matches!(
            read_snapshot(Cursor::new(unsupported), &mut store, wall_now),
            Err(SnapshotError::UnsupportedVersion(3))
        ));

        assert!(matches!(
            read_snapshot(Cursor::new(&valid[..valid.len() - 1]), &mut store, wall_now),
            Err(SnapshotError::Truncated)
        ));

        let mut corrupt = valid;
        corrupt[MAGIC.len() + 2 + 8 + 8] ^= 1;
        assert!(matches!(
            read_snapshot(Cursor::new(corrupt), &mut store, wall_now),
            Err(SnapshotError::ChecksumMismatch)
        ));
    }

    #[test]
    fn rejects_trailing_data_and_invalid_tags() {
        let wall_now = SystemTime::UNIX_EPOCH;
        let mut store = InMemoryStore::new();
        let mut trailing = encoded(&sample_entries(None));
        trailing.push(0);
        assert!(matches!(
            read_snapshot(Cursor::new(trailing), &mut store, wall_now),
            Err(SnapshotError::TrailingData)
        ));

        let mut invalid_type = encoded(&[SnapshotEntry {
            key: Vec::new(),
            value: SnapshotValue::String(Vec::new()),
            expires_at_unix_millis: None,
        }]);
        let value_type_offset = MAGIC.len() + 2 + 8 + 8;
        invalid_type[value_type_offset] = 9;
        assert!(matches!(
            read_snapshot(Cursor::new(invalid_type), &mut store, wall_now),
            Err(SnapshotError::InvalidValueType(9))
        ));

        let mut invalid_expiration = encoded(&[SnapshotEntry {
            key: Vec::new(),
            value: SnapshotValue::String(Vec::new()),
            expires_at_unix_millis: None,
        }]);
        let expiration_offset = value_type_offset + 1 + 8;
        invalid_expiration[expiration_offset] = 9;
        assert!(matches!(
            read_snapshot(Cursor::new(invalid_expiration), &mut store, wall_now),
            Err(SnapshotError::InvalidExpirationTag(9))
        ));
    }

    #[test]
    fn failed_replacement_keeps_the_last_valid_snapshot() {
        let directory = TestDirectory::new();
        let path = directory.path("database.snapshot");
        let mut store = InMemoryStore::new();
        store.set(b"key".to_vec(), b"old".to_vec());
        save(&path, &mut store).unwrap();
        let previous = fs::read(&path).unwrap();

        store.set(b"key".to_vec(), b"new".to_vec());
        let error = save_with_replace(
            &path,
            &mut store,
            SystemTime::now(),
            |_temporary, _destination| Err(io::Error::other("replace failed")),
        )
        .unwrap_err();

        assert!(matches!(error, SnapshotError::Io(_)));
        assert_eq!(fs::read(&path).unwrap(), previous);
        assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 1);
    }

    #[test]
    fn save_replaces_an_existing_snapshot() {
        let directory = TestDirectory::new();
        let path = directory.path("database.snapshot");
        let mut store = InMemoryStore::new();
        store.set(b"key".to_vec(), b"old".to_vec());
        save(&path, &mut store).unwrap();

        store.set(b"key".to_vec(), b"new".to_vec());
        save(&path, &mut store).unwrap();

        let mut restored = InMemoryStore::new();
        assert!(load(&path, &mut restored).unwrap());
        assert_eq!(restored.get(b"key"), Ok(Some(b"new".as_slice())));
        assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 1);
    }

    #[test]
    fn load_treats_a_missing_snapshot_as_an_empty_database() {
        let directory = TestDirectory::new();
        let mut store = InMemoryStore::new();
        store.set(b"existing".to_vec(), b"value".to_vec());

        assert!(!load(&directory.path("missing.snapshot"), &mut store).unwrap());
        assert!(store.exists(b"existing"));
    }

    #[test]
    fn invalid_snapshot_data_does_not_replace_existing_state() {
        let duplicate_entries = vec![
            SnapshotEntry {
                key: b"duplicate".to_vec(),
                value: SnapshotValue::String(b"first".to_vec()),
                expires_at_unix_millis: None,
            },
            SnapshotEntry {
                key: b"duplicate".to_vec(),
                value: SnapshotValue::String(b"second".to_vec()),
                expires_at_unix_millis: None,
            },
        ];
        let mut store = InMemoryStore::new();
        store.set(b"existing".to_vec(), b"value".to_vec());

        let error = read_snapshot(
            Cursor::new(encoded(&duplicate_entries)),
            &mut store,
            SystemTime::UNIX_EPOCH,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            SnapshotError::InvalidData(SnapshotDataError::DuplicateKey)
        ));
        assert_eq!(store.get(b"existing"), Ok(Some(b"value".as_slice())));
    }
}
