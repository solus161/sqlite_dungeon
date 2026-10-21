use crate::exceptions::CustomErr;

use super::column::{ColumnType, ColumnValue};
use super::utils::{get_offset, parse_varint};

#[derive(Debug, PartialEq, Clone)]
pub struct Payload {
    row_id: Option<u64>,
    columns: Vec<ColumnValue>,
}

impl Payload {
    pub fn column(&self, index: usize) -> Option<&ColumnValue> {
        self.columns.get(index)
    }

    pub fn last(&self) -> Result<&ColumnValue, CustomErr> {
        self.columns.last().ok_or(CustomErr::Execution(
            "Payload has not last value".to_string(),
        ))
    }

    pub fn add_row_id(&mut self, row_id: u64) {
        self.row_id = Some(row_id)
    }

    pub fn row_id(&self) -> Option<&u64> {
        self.row_id.as_ref()
    }
}

impl TryFrom<&[u8]> for Payload {
    type Error = CustomErr;

    fn try_from(buf: &[u8]) -> Result<Self, Self::Error> {
        let mut offset: usize = 0;

        // 1 varint is header length
        let (header_length, header_bytes) = parse_varint(buf);
        offset += header_bytes;
        let mut remaining_bytes: usize = header_length as usize - header_bytes;

        let mut column_types: Vec<ColumnType> = Vec::new();
        while remaining_bytes > 0 {
            let (value, value_bytes) = parse_varint(get_offset(buf, offset, remaining_bytes));
            column_types.push(ColumnType::new(value)?);
            offset += value_bytes;
            remaining_bytes -= value_bytes;
        }

        // Onto payload body
        let mut column_values: Vec<ColumnValue> = Vec::new();
        for t in column_types {
            let value = ColumnValue::new(&t, &buf[offset..])?;
            column_values.push(value);
            offset += t.content_length().unwrap_or(0);
        }
        Ok(Self {
            row_id: None,
            columns: column_values,
        })
    }
}
