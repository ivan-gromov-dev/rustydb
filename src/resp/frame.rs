use std::io::{self, Write};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RespFrame {
    SimpleString(String),
    Error(String),
    Integer(i64),
    BulkString(Vec<u8>),
    Array(Vec<RespFrame>),
    Map(Vec<(RespFrame, RespFrame)>),
    Null,
    NullBulkString,
    NullArray,
}

impl RespFrame {
    pub(crate) fn write_to(&self, writer: &mut impl Write) -> io::Result<()> {
        match self {
            Self::SimpleString(value) => {
                writer.write_all(b"+")?;
                writer.write_all(value.as_bytes())?;
                writer.write_all(b"\r\n")
            }
            Self::Error(value) => {
                writer.write_all(b"-")?;
                writer.write_all(value.as_bytes())?;
                writer.write_all(b"\r\n")
            }
            Self::Integer(value) => write!(writer, ":{value}\r\n"),
            Self::BulkString(value) => {
                write!(writer, "${}\r\n", value.len())?;
                writer.write_all(value)?;
                writer.write_all(b"\r\n")
            }
            Self::Array(values) => {
                write!(writer, "*{}\r\n", values.len())?;

                for value in values {
                    value.write_to(writer)?;
                }

                Ok(())
            }
            Self::Map(entries) => {
                write!(writer, "%{}\r\n", entries.len())?;

                for (key, value) in entries {
                    key.write_to(writer)?;
                    value.write_to(writer)?;
                }

                Ok(())
            }
            Self::Null => writer.write_all(b"_\r\n"),
            Self::NullBulkString => writer.write_all(b"$-1\r\n"),
            Self::NullArray => writer.write_all(b"*-1\r\n"),
        }
    }
}
