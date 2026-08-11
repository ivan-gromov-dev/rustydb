use std::fmt;
use std::str;

use super::frame::RespFrame;

const DEFAULT_MAX_FRAME_SIZE: usize = 8 * 1024 * 1024;
const DEFAULT_MAX_ARRAY_LEN: usize = 1024;
const DEFAULT_MAX_DEPTH: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DecodeLimits {
    pub(crate) max_frame_size: usize,
    pub(crate) max_array_len: usize,
    pub(crate) max_depth: usize,
}

impl Default for DecodeLimits {
    fn default() -> Self {
        Self {
            max_frame_size: DEFAULT_MAX_FRAME_SIZE,
            max_array_len: DEFAULT_MAX_ARRAY_LEN,
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DecodeResult {
    Complete { frame: RespFrame, consumed: usize },
    Incomplete,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DecodeError {
    InvalidPrefix(u8),
    InvalidLineEnding,
    InvalidInteger,
    InvalidLength,
    InvalidUtf8,
    FrameTooLarge,
    ArrayTooLong,
    TooDeep,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPrefix(prefix) => write!(formatter, "invalid RESP prefix: {prefix:#04x}"),
            Self::InvalidLineEnding => write!(formatter, "invalid RESP line ending"),
            Self::InvalidInteger => write!(formatter, "invalid RESP integer"),
            Self::InvalidLength => write!(formatter, "invalid RESP length"),
            Self::InvalidUtf8 => write!(formatter, "invalid UTF-8 in RESP text frame"),
            Self::FrameTooLarge => write!(formatter, "RESP frame exceeds size limit"),
            Self::ArrayTooLong => write!(formatter, "RESP array exceeds element limit"),
            Self::TooDeep => write!(formatter, "RESP frame exceeds nesting limit"),
        }
    }
}

pub(crate) fn decode(input: &[u8], limits: DecodeLimits) -> Result<DecodeResult, DecodeError> {
    parse_frame(input, limits, 0)
}

fn parse_frame(
    input: &[u8],
    limits: DecodeLimits,
    depth: usize,
) -> Result<DecodeResult, DecodeError> {
    if depth > limits.max_depth {
        return Err(DecodeError::TooDeep);
    }

    let Some(prefix) = input.first().copied() else {
        return Ok(DecodeResult::Incomplete);
    };

    match prefix {
        b'+' => parse_text(input, limits, RespFrame::SimpleString),
        b'-' => parse_text(input, limits, RespFrame::Error),
        b':' => parse_integer(input, limits),
        b'$' => parse_bulk_string(input, limits),
        b'*' => parse_array(input, limits, depth),
        prefix => Err(DecodeError::InvalidPrefix(prefix)),
    }
}

fn parse_text(
    input: &[u8],
    limits: DecodeLimits,
    make_frame: impl FnOnce(String) -> RespFrame,
) -> Result<DecodeResult, DecodeError> {
    let Some((value, consumed)) = parse_line(&input[1..], limits.max_frame_size)? else {
        return Ok(DecodeResult::Incomplete);
    };
    let consumed = consumed.checked_add(1).ok_or(DecodeError::FrameTooLarge)?;
    ensure_frame_size(consumed, limits)?;

    let value = str::from_utf8(value).map_err(|_| DecodeError::InvalidUtf8)?;

    Ok(DecodeResult::Complete {
        frame: make_frame(value.to_owned()),
        consumed,
    })
}

fn parse_integer(input: &[u8], limits: DecodeLimits) -> Result<DecodeResult, DecodeError> {
    let Some((value, consumed)) = parse_line(&input[1..], limits.max_frame_size)? else {
        return Ok(DecodeResult::Incomplete);
    };
    let consumed = consumed.checked_add(1).ok_or(DecodeError::FrameTooLarge)?;
    ensure_frame_size(consumed, limits)?;

    let value = parse_i64(value).map_err(|_| DecodeError::InvalidInteger)?;

    Ok(DecodeResult::Complete {
        frame: RespFrame::Integer(value),
        consumed,
    })
}

fn parse_bulk_string(input: &[u8], limits: DecodeLimits) -> Result<DecodeResult, DecodeError> {
    let Some((length, header_len)) = parse_line(&input[1..], limits.max_frame_size)? else {
        return Ok(DecodeResult::Incomplete);
    };
    let header_len = header_len
        .checked_add(1)
        .ok_or(DecodeError::FrameTooLarge)?;
    let length = parse_i64(length).map_err(|_| DecodeError::InvalidLength)?;

    if length == -1 {
        ensure_frame_size(header_len, limits)?;
        return Ok(DecodeResult::Complete {
            frame: RespFrame::NullBulkString,
            consumed: header_len,
        });
    }

    let length = usize::try_from(length).map_err(|_| DecodeError::InvalidLength)?;
    let consumed = header_len
        .checked_add(length)
        .and_then(|value| value.checked_add(2))
        .ok_or(DecodeError::FrameTooLarge)?;
    ensure_frame_size(consumed, limits)?;

    if input.len() < consumed {
        return Ok(DecodeResult::Incomplete);
    }

    if input[header_len + length..consumed] != *b"\r\n" {
        return Err(DecodeError::InvalidLineEnding);
    }

    Ok(DecodeResult::Complete {
        frame: RespFrame::BulkString(input[header_len..header_len + length].to_vec()),
        consumed,
    })
}

fn parse_array(
    input: &[u8],
    limits: DecodeLimits,
    depth: usize,
) -> Result<DecodeResult, DecodeError> {
    let Some((length, header_len)) = parse_line(&input[1..], limits.max_frame_size)? else {
        return Ok(DecodeResult::Incomplete);
    };
    let mut consumed = header_len
        .checked_add(1)
        .ok_or(DecodeError::FrameTooLarge)?;
    let length = parse_i64(length).map_err(|_| DecodeError::InvalidLength)?;

    if length == -1 {
        ensure_frame_size(consumed, limits)?;
        return Ok(DecodeResult::Complete {
            frame: RespFrame::NullArray,
            consumed,
        });
    }

    let length = usize::try_from(length).map_err(|_| DecodeError::InvalidLength)?;
    if length > limits.max_array_len {
        return Err(DecodeError::ArrayTooLong);
    }

    let mut frames = Vec::with_capacity(length);
    for _ in 0..length {
        match parse_frame(&input[consumed..], limits, depth + 1)? {
            DecodeResult::Complete {
                frame,
                consumed: frame_len,
            } => {
                consumed = consumed
                    .checked_add(frame_len)
                    .ok_or(DecodeError::FrameTooLarge)?;
                ensure_frame_size(consumed, limits)?;
                frames.push(frame);
            }
            DecodeResult::Incomplete => return Ok(DecodeResult::Incomplete),
        }
    }

    Ok(DecodeResult::Complete {
        frame: RespFrame::Array(frames),
        consumed,
    })
}

fn parse_line(input: &[u8], max_frame_size: usize) -> Result<Option<(&[u8], usize)>, DecodeError> {
    for (index, byte) in input.iter().copied().enumerate() {
        match byte {
            b'\r' if input.get(index + 1) == Some(&b'\n') => {
                return Ok(Some((&input[..index], index + 2)));
            }
            b'\r' if input.get(index + 1).is_none() => return Ok(None),
            b'\r' | b'\n' => return Err(DecodeError::InvalidLineEnding),
            _ => {}
        }
    }

    if input.len() >= max_frame_size {
        Err(DecodeError::FrameTooLarge)
    } else {
        Ok(None)
    }
}

fn parse_i64(input: &[u8]) -> Result<i64, ()> {
    str::from_utf8(input)
        .map_err(|_| ())?
        .parse()
        .map_err(|_| ())
}

fn ensure_frame_size(consumed: usize, limits: DecodeLimits) -> Result<(), DecodeError> {
    if consumed > limits.max_frame_size {
        Err(DecodeError::FrameTooLarge)
    } else {
        Ok(())
    }
}
