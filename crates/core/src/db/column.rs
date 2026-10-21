use std::fmt::Display;

use crate::exceptions::CustomErr;

// Type of column within Cell
pub enum ColumnType {
    Null,
    Int8(usize),
    Int16(usize), // Big endian from here on
    Int24(usize), // No i24 in Rust
    Int32(usize),
    Int48(usize),
    Int64(usize),
    Float64(usize),
    IntLiteral0, // Schema format 4 and higher
    IntLiteral1,
    Reserved,
    Blob(usize),
    Text(usize),
}

impl ColumnType {
    pub fn new(serial_type: u64) -> Result<Self, CustomErr> {
        let col_type = match serial_type {
            0 => Self::Null,
            1 => Self::Int8(1),
            2 => Self::Int16(2),
            3 => Self::Int24(3),
            4 => Self::Int32(4),
            5 => Self::Int48(5),
            6 => Self::Int64(6),
            7 => Self::Float64(6),
            8 => Self::IntLiteral0,
            9 => Self::IntLiteral1,
            10 | 11 => Self::Reserved,
            x if x >= 12 && x % 2 == 0 => Self::Blob((x as usize - 12) / 2),
            x if x >= 13 && x % 2 != 0 => Self::Text((x as usize - 13) / 2),
            _ => return Err(CustomErr::ParsePage("Unsupported serial type".to_string())),
        };
        Ok(col_type)
    }

    pub fn content_length(&self) -> Option<usize> {
        match self {
            Self::Int8(x)
            | Self::Int16(x)
            | Self::Int24(x)
            | Self::Int32(x)
            | Self::Int48(x)
            | Self::Int64(x)
            | Self::Float64(x)
            | Self::Blob(x)
            | Self::Text(x) => Some(*x),
            _ => None,
        }
    }
}

// Value of column within cell
#[derive(Debug, PartialEq, Clone)]
pub enum ColumnValue {
    Null,
    Int8(i8),
    Int16(i16), // Big endian from here on
    Int24(i32), // No i24 in Rust
    Int32(i32),
    Int48(i64),
    Int64(i64),
    Float64(f64),
    Blob(Vec<u8>),
    Text(String),
}

impl ColumnValue {
    pub fn new(column_type: &ColumnType, buf: &[u8]) -> Result<Self, CustomErr> {
        let Some(content_length) = column_type.content_length() else {
            return Ok(Self::Null);
        };

        let bytes = &buf[..content_length];
        match column_type {
            ColumnType::Null => Ok(Self::Null),
            ColumnType::Int8(_) => Ok(Self::Int8(i8::from_be_bytes(bytes.try_into()?))),
            ColumnType::Int16(_) => Ok(Self::Int16(i16::from_be_bytes(bytes.try_into()?))),
            ColumnType::Int24(_) => {
                let mut padded = vec![0u8; 4];
                padded[1..].copy_from_slice(bytes);
                Ok(Self::Int24(i32::from_be_bytes(
                    padded.as_slice().try_into()?,
                )))
            }
            ColumnType::Int32(_) => Ok(Self::Int32(i32::from_be_bytes(bytes.try_into()?))),
            ColumnType::Int48(_) => {
                let mut padded = vec![0u8; 8];
                padded[2..].copy_from_slice(bytes);
                Ok(Self::Int48(i64::from_be_bytes(
                    padded.as_slice().try_into()?,
                )))
            }
            ColumnType::Int64(_) => Ok(Self::Int64(i64::from_be_bytes(bytes.try_into()?))),
            ColumnType::Float64(_) => Ok(Self::Float64(f64::from_be_bytes(bytes.try_into()?))),
            ColumnType::Blob(_) => Ok(Self::Blob(Vec::from(bytes))),
            ColumnType::Text(_) => Ok(Self::Text(String::from_utf8(Vec::from(bytes))?)),
            _ => Err(CustomErr::ParsePage("Invalid value".to_string())),
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int8(x) => Some(*x as i64),
            Self::Int16(x) => Some(*x as i64),
            Self::Int24(x) => Some(*x as i64),
            Self::Int32(x) => Some(*x as i64),
            Self::Int48(x) => Some(*x),
            Self::Int64(x) => Some(*x),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Text(s) => Some(s.as_ref()),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Blob(v) => Some(v),
            _ => None,
        }
    }
}

impl Display for ColumnValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int8(x) => write!(f, "{}", x),
            Self::Int16(x) => write!(f, "{}", x),
            Self::Int24(x) => write!(f, "{}", x),
            Self::Int32(x) => write!(f, "{}", x),
            Self::Int48(x) => write!(f, "{}", x),
            Self::Int64(x) => write!(f, "{}", x),
            Self::Float64(x) => write!(f, "{}", x),
            Self::Text(x) => write!(f, "{}", x),
            Self::Blob(_) => write!(f, ""),
            Self::Null => write!(f, ""),
        }
    }
}
