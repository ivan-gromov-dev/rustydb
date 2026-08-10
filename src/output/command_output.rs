use std::io::{self, Write};

const HELP_TEXT: &str = concat!(
    "Available commands:\n",
    "  SET key value\n",
    "  MSET key value [key value ...]\n",
    "  SETNX key value\n",
    "  GET key\n",
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
    "  RENAME old_key new_key\n",
    "  EXPIRE key seconds\n",
    "  PEXPIRE key milliseconds\n",
    "  TTL key\n",
    "  PTTL key\n",
    "  PERSIST key\n",
    "  STRLEN key\n",
    "  GETRANGE key start end\n",
    "  SETRANGE key offset value\n",
    "  LPUSH key value\n",
    "  RPUSH key value\n",
    "  LRANGE key start end\n",
    "  LLEN key\n",
    "  LPOP key\n",
    "  RPOP key\n",
    "  KEYS\n",
    "  LEN\n",
    "  CLEAR\n",
    "  HELP\n",
    "  EXIT\n",
);

#[derive(Debug, PartialEq)]
pub(crate) enum CommandOutput {
    Ok,
    Integer(i64),
    Float(f64),
    Value(String),
    OptionalValues(Vec<Option<String>>),
    Nil,
    KeyList(Vec<String>),
    Error(String),
    Help,
    Exit,
}

impl CommandOutput {
    pub(crate) fn write_to(&self, writer: &mut impl Write) -> io::Result<()> {
        match self {
            Self::Ok => writeln!(writer, "OK"),
            Self::Integer(value) => writeln!(writer, "{value}"),
            Self::Float(value) => writeln!(writer, "{value}"),
            Self::Value(value) => writeln!(writer, "{value}"),
            Self::OptionalValues(values) => {
                for value in values {
                    match value {
                        Some(value) => writeln!(writer, "{value}")?,
                        None => writeln!(writer, "(nil)")?,
                    }
                }
                Ok(())
            }
            Self::Nil => writeln!(writer, "(nil)"),
            Self::KeyList(keys) if keys.is_empty() => writeln!(writer, "(nil)"),
            Self::KeyList(keys) => {
                for key in keys {
                    writeln!(writer, "{key}")?;
                }
                Ok(())
            }
            Self::Error(error) => writeln!(writer, "ERR {error}"),
            Self::Help => writer.write_all(HELP_TEXT.as_bytes()),
            Self::Exit => Ok(()),
        }
    }
}
