use std::io::{self, Write};

use crate::command::{CommandMetadata, ProtocolVersion};

pub(crate) const HELP_TEXT: &str = concat!(
    "Available commands:\n",
    "  SET key value [NX|XX] [GET] [EX seconds|PX milliseconds|EXAT unix-seconds|PXAT unix-milliseconds|KEEPTTL]\n",
    "  MSET key value [key value ...]\n",
    "  MSETNX key value [key value ...]\n",
    "  SETNX key value\n",
    "  GET key\n",
    "  GETEX key [EX seconds|PX milliseconds|EXAT unix-seconds|PXAT unix-milliseconds|PERSIST]\n",
    "  MGET key [key ...]\n",
    "  GETSET key value\n",
    "  GETDEL key\n",
    "  APPEND key value\n",
    "  INCR key\n",
    "  INCRBY key inc_value\n",
    "  DECR key\n",
    "  DECRBY key decr_value\n",
    "  INCRBYFLOAT key amount\n",
    "  EXISTS key [key ...]\n",
    "  DEL key [key ...]\n",
    "  TYPE key\n",
    "  TOUCH key [key ...]\n",
    "  UNLINK key [key ...]\n",
    "  RENAME old_key new_key\n",
    "  EXPIRE key seconds [NX|XX|GT|LT]\n",
    "  PEXPIRE key milliseconds [NX|XX|GT|LT]\n",
    "  EXPIREAT key unix-seconds [NX|XX|GT|LT]\n",
    "  PEXPIREAT key unix-milliseconds [NX|XX|GT|LT]\n",
    "  TTL key\n",
    "  PTTL key\n",
    "  EXPIRETIME key\n",
    "  PEXPIRETIME key\n",
    "  PERSIST key\n",
    "  STRLEN key\n",
    "  GETRANGE key start end\n",
    "  SETRANGE key offset value\n",
    "  LPUSH key value [value ...]\n",
    "  LPUSHX key value [value ...]\n",
    "  RPUSH key value [value ...]\n",
    "  RPUSHX key value [value ...]\n",
    "  LRANGE key start end\n",
    "  LLEN key\n",
    "  LINDEX key index\n",
    "  LSET key index value\n",
    "  LINSERT key BEFORE|AFTER pivot element\n",
    "  LTRIM key start stop\n",
    "  LREM key count element\n",
    "  LPOS key element [RANK rank] [COUNT count] [MAXLEN len]\n",
    "  LMOVE source destination LEFT|RIGHT LEFT|RIGHT\n",
    "  RPOPLPUSH source destination\n",
    "  BLPOP key [key ...] timeout\n",
    "  BRPOP key [key ...] timeout\n",
    "  BLMOVE source destination LEFT|RIGHT LEFT|RIGHT timeout\n",
    "  LPOP key [count]\n",
    "  RPOP key [count]\n",
    "  SADD key member [member ...]\n",
    "  SREM key member [member ...]\n",
    "  SISMEMBER key member\n",
    "  SMISMEMBER key member [member ...]\n",
    "  SPOP key [count]\n",
    "  SRANDMEMBER key [count]\n",
    "  SMOVE source destination member\n",
    "  SDIFF key [key ...]\n",
    "  SINTER key [key ...]\n",
    "  SUNION key [key ...]\n",
    "  SDIFFSTORE destination key [key ...]\n",
    "  SINTERSTORE destination key [key ...]\n",
    "  SUNIONSTORE destination key [key ...]\n",
    "  SSCAN key cursor [MATCH pattern] [COUNT count]\n",
    "  SMEMBERS key\n",
    "  SCARD key\n",
    "  HSET key field value [field value ...]\n",
    "  HSETNX key field value\n",
    "  HGET key field\n",
    "  HMGET key field [field ...]\n",
    "  HGETALL key\n",
    "  HDEL key field [field ...]\n",
    "  HEXISTS key field\n",
    "  HLEN key\n",
    "  HKEYS key\n",
    "  HVALS key\n",
    "  HINCRBY key field increment\n",
    "  HINCRBYFLOAT key field increment\n",
    "  HSCAN key cursor [MATCH pattern] [COUNT count]\n",
    "  PING [message]\n",
    "  ECHO message\n",
    "  HELLO [2|3]\n",
    "  CLIENT ID\n",
    "  CLIENT SETNAME name\n",
    "  CLIENT GETNAME\n",
    "  CLIENT SETINFO LIB-NAME|LIB-VER value\n",
    "  COMMAND [INFO [command ...]|COUNT]\n",
    "  SELECT 0\n",
    "  DBSIZE\n",
    "  FLUSHDB [SYNC|ASYNC]\n",
    "  FLUSHALL [SYNC|ASYNC]\n",
    "  KEYS pattern\n",
    "  SCAN cursor [MATCH pattern] [COUNT count] [TYPE type]\n",
    "  RANDOMKEY\n",
    "  COPY source destination [DB 0] [REPLACE]\n",
    "  LEN\n",
    "  CLEAR\n",
    "  SAVE\n",
    "  AOFREWRITE\n",
    "  INFO\n",
    "  HELP\n",
    "  EXIT\n",
);

#[derive(Debug, PartialEq)]
pub(crate) enum CommandOutput {
    Ok,
    Pong,
    Hello {
        protocol: Option<ProtocolVersion>,
        connection_id: Option<i64>,
    },
    Integer(i64),
    IntegerList(Vec<i64>),
    Float(f64),
    SimpleString(&'static str),
    Value(Vec<u8>),
    OptionalValues(Vec<Option<Vec<u8>>>),
    Nil,
    NullArray,
    KeyList(Vec<Vec<u8>>),
    HashEntries(Vec<(Vec<u8>, Vec<u8>)>),
    HashScan {
        cursor: usize,
        entries: Vec<(Vec<u8>, Vec<u8>)>,
    },
    Scan {
        cursor: usize,
        keys: Vec<Vec<u8>>,
    },
    CommandMetadata(Vec<Option<CommandMetadata>>),
    Error(String),
    Help,
    Exit,
}

impl CommandOutput {
    pub(crate) fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    pub(crate) fn write_to(&self, writer: &mut impl Write) -> io::Result<()> {
        match self {
            Self::Ok => writeln!(writer, "OK"),
            Self::Pong => writeln!(writer, "PONG"),
            Self::Hello {
                protocol,
                connection_id,
            } => {
                let protocol = protocol.unwrap_or(ProtocolVersion::Resp2).number();
                writeln!(writer, "server:rustydb")?;
                writeln!(writer, "version:{}", env!("CARGO_PKG_VERSION"))?;
                writeln!(writer, "proto:{protocol}")?;
                if let Some(connection_id) = connection_id {
                    writeln!(writer, "id:{connection_id}")?;
                }
                writeln!(writer, "mode:standalone")?;
                writeln!(writer, "role:master")?;
                writeln!(writer, "modules:(empty)")
            }
            Self::Integer(value) => writeln!(writer, "{value}"),
            Self::IntegerList(values) if values.is_empty() => writeln!(writer, "(nil)"),
            Self::IntegerList(values) => {
                for value in values {
                    writeln!(writer, "{value}")?;
                }
                Ok(())
            }
            Self::Float(value) => writeln!(writer, "{value}"),
            Self::SimpleString(value) => writeln!(writer, "{value}"),
            Self::Value(value) => {
                writer.write_all(value)?;
                writer.write_all(b"\n")
            }
            Self::OptionalValues(values) => {
                for value in values {
                    match value {
                        Some(value) => {
                            writer.write_all(value)?;
                            writer.write_all(b"\n")?;
                        }
                        None => writeln!(writer, "(nil)")?,
                    }
                }
                Ok(())
            }
            Self::Nil => writeln!(writer, "(nil)"),
            Self::NullArray => writeln!(writer, "(nil)"),
            Self::KeyList(keys) if keys.is_empty() => writeln!(writer, "(nil)"),
            Self::KeyList(keys) => {
                for key in keys {
                    writer.write_all(key)?;
                    writer.write_all(b"\n")?;
                }
                Ok(())
            }
            Self::HashEntries(entries) if entries.is_empty() => writeln!(writer, "(nil)"),
            Self::HashEntries(entries) => {
                for (field, value) in entries {
                    writer.write_all(field)?;
                    writer.write_all(b"\n")?;
                    writer.write_all(value)?;
                    writer.write_all(b"\n")?;
                }
                Ok(())
            }
            Self::HashScan { cursor, entries } => {
                writeln!(writer, "{cursor}")?;
                for (field, value) in entries {
                    writer.write_all(field)?;
                    writer.write_all(b"\n")?;
                    writer.write_all(value)?;
                    writer.write_all(b"\n")?;
                }
                Ok(())
            }
            Self::Scan { cursor, keys } => {
                writeln!(writer, "{cursor}")?;
                for key in keys {
                    writer.write_all(key)?;
                    writer.write_all(b"\n")?;
                }
                Ok(())
            }
            Self::CommandMetadata(entries) => {
                for entry in entries {
                    match entry {
                        Some(metadata) => writeln!(
                            writer,
                            "{} arity:{} flags:{} keys:{}/{}/{}",
                            metadata.name,
                            metadata.arity,
                            metadata.flags.join(","),
                            metadata.first_key,
                            metadata.last_key,
                            metadata.key_step
                        )?,
                        None => writeln!(writer, "(nil)")?,
                    }
                }
                Ok(())
            }
            Self::Error(error) => writeln!(writer, "ERR {error}"),
            Self::Help => writer.write_all(HELP_TEXT.as_bytes()),
            Self::Exit => Ok(()),
        }
    }
}
