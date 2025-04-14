#![allow(missing_docs)]
//! Utilities for parsing events emitted by axelar-solana programs
use axelar_message_primitives::U256;
use solana_program::pubkey::Pubkey;

pub use base64;
pub use event_macros::*;

type Disc = [u8; 16];
pub trait Event {
    const DISC: &'static Disc;

    /// Emits the event data using `sol_log_data`
    fn emit(&self);

    /// Tries to parses an event of this type from a log message string.
    fn try_from_log(log: &str) -> Result<Self, EventParseError>
    where
        Self: Sized;

    /// Parses an event of this type from combined, decoded log data bytes.
    /// Assumes the discriminant has *already been checked* by the caller.
    fn deserialize<I: Iterator<Item = Vec<u8>>>(data: I) -> Result<Self, EventParseError>
    where
        Self: Sized;
}

/// Errors that may occur while parsing a `MessageEvent`.
#[derive(Debug, thiserror::Error)]
pub enum EventParseError {
    /// Occurs when a required field is missing in the event data.
    #[error("Missing data: {0}")]
    MissingData(&'static str),

    /// The data is there but it's not of valid format
    #[error("Invalid data: {0}")]
    InvalidData(&'static str),

    /// Occurs when the length of a field does not match the expected length.
    #[error("Invalid length for {field}: expected {expected}, got {actual}")]
    InvalidLength {
        /// the field that we're trying to parse
        field: &'static str,
        /// the desired length
        expected: usize,
        /// the actual length
        actual: usize,
    },

    /// Occurs when a field contains invalid UTF-8 data.
    #[error("Invalid UTF-8 in {field}: {source}")]
    InvalidUtf8 {
        /// the field we're trying to parse
        field: &'static str,
        /// underlying error
        #[source]
        source: std::string::FromUtf8Error,
    },

    /// Generic error for any other parsing issues.
    #[error("Other error: {0}")]
    Other(&'static str),
}

/// Tries to read a fixed-size array from the provided data slice.
pub fn read_array<const N: usize>(
    field: &'static str,
    data: &[u8],
) -> Result<[u8; N], EventParseError> {
    if data.len() != N {
        return Err(EventParseError::InvalidLength {
            field,
            expected: N,
            actual: data.len(),
        });
    }
    let array = data
        .try_into()
        .map_err(|_err| EventParseError::InvalidLength {
            field,
            expected: N,
            actual: data.len(),
        })?;
    Ok(array)
}

/// Tries to read a string from the provided data vector.
pub fn read_string(field: &'static str, data: Vec<u8>) -> Result<String, EventParseError> {
    String::from_utf8(data).map_err(|err| EventParseError::InvalidUtf8 { field, source: err })
}

pub fn read_u8(field: &'static str, data: &[u8]) -> Result<u8, EventParseError> {
    if data.len() != 1 {
        return Err(EventParseError::InvalidData(field));
    }
    Ok(data[0])
}

pub fn read_u16(field: &'static str, data: &[u8]) -> Result<u16, EventParseError> {
    if data.len() != 2 {
        return Err(EventParseError::InvalidData(field));
    }
    Ok(u16::from_le_bytes(data.try_into().expect("length checked")))
}

pub fn read_u32(field: &'static str, data: &[u8]) -> Result<u32, EventParseError> {
    if data.len() != 4 {
        return Err(EventParseError::InvalidData(field));
    }
    Ok(u32::from_le_bytes(data.try_into().expect("length checked")))
}

/// Tries to read a u64 from the provided data slice.
#[allow(clippy::little_endian_bytes)]
pub fn read_u64(field: &'static str, data: &[u8]) -> Result<u64, EventParseError> {
    if data.len() != 8 {
        return Err(EventParseError::InvalidData(field));
    }

    Ok(u64::from_le_bytes(data.try_into().expect("length checked")))
}

pub fn read_u128(field: &'static str, data: &[u8]) -> Result<u128, EventParseError> {
    if data.len() != 16 {
        return Err(EventParseError::InvalidData(field));
    }

    Ok(u128::from_le_bytes(
        data.try_into().expect("length checked"),
    ))
}

pub fn read_i8(field: &'static str, data: &[u8]) -> Result<i8, EventParseError> {
    if data.len() != 1 {
        return Err(EventParseError::InvalidData(field));
    }
    Ok(i8::from_le_bytes(data.try_into().expect("length checked")))
}

pub fn read_i16(field: &'static str, data: &[u8]) -> Result<i16, EventParseError> {
    if data.len() != 2 {
        return Err(EventParseError::InvalidData(field));
    }
    Ok(i16::from_le_bytes(data.try_into().expect("length checked")))
}

pub fn read_i32(field: &'static str, data: &[u8]) -> Result<i32, EventParseError> {
    if data.len() != 4 {
        return Err(EventParseError::InvalidData(field));
    }
    Ok(i32::from_le_bytes(data.try_into().expect("length checked")))
}

pub fn read_i64(field: &'static str, data: &[u8]) -> Result<i64, EventParseError> {
    if data.len() != 8 {
        return Err(EventParseError::InvalidData(field));
    }
    Ok(i64::from_le_bytes(data.try_into().expect("length checked")))
}

pub fn read_i128(field: &'static str, data: &[u8]) -> Result<i128, EventParseError> {
    if data.len() != 16 {
        return Err(EventParseError::InvalidData(field));
    }
    Ok(i128::from_le_bytes(
        data.try_into().expect("length checked"),
    ))
}

pub fn read_pubkey(field: &'static str, data: &[u8]) -> Result<Pubkey, EventParseError> {
    if data.len() != 32 {
        return Err(EventParseError::InvalidData(field));
    }
    Ok(Pubkey::new_from_array(
        data.try_into().expect("length checked"),
    ))
}

pub fn read_bool(field: &'static str, data: &[u8]) -> Result<bool, EventParseError> {
    if data.len() != 1 {
        return Err(EventParseError::InvalidData(field));
    }
    Ok(data[0] != 0)
}

pub fn read_f32(field: &'static str, data: &[u8]) -> Result<f32, EventParseError> {
    if data.len() != 4 {
        return Err(EventParseError::InvalidData(field));
    }
    Ok(f32::from_le_bytes(data.try_into().expect("length checked")))
}

pub fn read_f64(field: &'static str, data: &[u8]) -> Result<f64, EventParseError> {
    if data.len() != 8 {
        return Err(EventParseError::InvalidData(field));
    }
    Ok(f64::from_le_bytes(data.try_into().expect("length checked")))
}

pub fn read_u256(field: &'static str, data: &[u8]) -> Result<U256, EventParseError> {
    if data.len() != 32 {
        return Err(EventParseError::InvalidData(field));
    }
    Ok(U256::from_le_bytes(
        data.try_into().expect("length checked"),
    ))
}
