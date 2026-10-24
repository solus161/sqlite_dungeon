use crate::data_types::ValueExpr;
use crate::exceptions::CustomErr;
use crate::io::page_source::PageSource;

use super::page::PageType;
use super::payload::Payload;

struct CellBuilder {
    left_child_ptr: Option<u64>,
    payload_length: Option<u64>,
    row_id: Option<u64>,
    payload: Option<PageCell>,
    overflow_ptr: Option<u64>,
}

impl CellBuilder {
    pub fn new() -> Self {
        Self {
            left_child_ptr: None,
            payload_length: None,
            row_id: None,
            payload: None,
            overflow_ptr: None,
        }
    }

    pub fn with_payload(&mut self, payload: PageCell) -> &mut Self {
        self.payload = Some(payload);
        self
    }

    build_attr_u64!(left_child_ptr);
    build_attr_u64!(payload_length);
    build_attr_u64!(row_id);
    build_attr_u64!(overflow_ptr);
}

fn get_offset(buf: &[u8], offset: usize, length: usize) -> &[u8] {
    &buf[offset..offset + length]
}

fn parse_varint(buf: &[u8]) -> (u64, usize) {
    // convert a slice of bytes to u64, big endian
    let mut output = 0u64;

    // Up to 9 bytes could be use
    for (i, _b) in buf.iter().enumerate().take(9) {
        let b = buf[i];
        if i == 8 {
            output = (output << 8) | b as u64;
            return (output, 9);
        };

        output = (output << 7) | (b & 0x7f) as u64;
        // Check bit at 0
        if b & 0x80 == 0 {
            return (output, i + 1);
        };
    }

    (output, 0)
}

#[derive(Debug, PartialEq)]
pub enum PageCell {
    TblInterior {
        left_child_ptr: u64,
        row_id: u64,
    },
    IdxInterior {
        left_child_ptr: u64,
        payload_length: u64,
        payload: Payload,
    },
    TblLeaf {
        payload_length: u64,
        row_id: u64,
        payload: Payload,
    },
    IdxLeaf {
        payload_length: u64,
        payload: Payload,
    },
}

impl PageCell {
    pub fn new<T>(
        page_type: &PageType,
        buf: &[u8],
        src: &mut T,
        page_size: u64,
    ) -> Result<Self, CustomErr>
    where
        T: PageSource,
    {
        match page_type {
            PageType::TblInterior => {
                let left_child_ptr = u32::from_be_bytes(get_offset(buf, 0, 4).try_into()?) as u64;
                let (row_id, _row_bytes) = parse_varint(&buf[4..]);
                Ok(Self::TblInterior {
                    left_child_ptr,
                    row_id,
                })
            }
            PageType::IdxInterior => {
                let mut offset: usize = 0;
                let left_child_ptr = u32::from_be_bytes(get_offset(buf, 0, 4).try_into()?) as u64;
                offset += 4;
                let (payload_length, payload_bytes) = parse_varint(&buf[offset..]);
                offset += payload_bytes;

                // Handle overflow page
                let payload: Payload = if offset + payload_length as usize > buf.len() {
                    let payload_buf =
                        Self::get_overflow_payload(src, page_size, &buf[offset..], payload_length)?;
                    Payload::try_from(&payload_buf[..])?
                } else {
                    Payload::try_from(&buf[offset..])?
                };
                Ok(Self::IdxInterior {
                    left_child_ptr,
                    payload_length,
                    payload,
                })
            }
            PageType::TblLeaf => {
                let mut offset: usize = 0;
                let (payload_length, payload_bytes) = parse_varint(buf);
                offset += payload_bytes;
                let (row_id, row_id_bytes) = parse_varint(&buf[offset..]);
                offset += row_id_bytes;

                // There could be overflow page
                let mut payload: Payload = if offset + payload_length as usize > buf.len() {
                    let payload_buf =
                        Self::get_overflow_payload(src, page_size, &buf[offset..], payload_length)?;
                    Payload::try_from(&payload_buf[..])?
                } else {
                    Payload::try_from(&buf[offset..])?
                };
                payload.add_row_id(row_id);
                Ok(Self::TblLeaf {
                    payload_length,
                    row_id,
                    payload,
                })
            }
            PageType::IdxLeaf => {
                let mut offset: usize = 0;
                let (payload_length, payload_bytes) = parse_varint(buf);
                offset += payload_bytes;

                // Overflow page
                let payload: Payload = if offset + payload_length as usize > buf.len() {
                    let payload_buf =
                        Self::get_overflow_payload(src, page_size, &buf[offset..], payload_length)?;
                    Payload::try_from(&payload_buf[..])?
                } else {
                    Payload::try_from(&buf[offset..])?
                };
                Ok(Self::IdxLeaf {
                    payload_length,
                    payload,
                })
            }
        }
    }

    /// A payload may be in two or more pages
    /// this walks the payload till overflow_ptr is none
    fn get_overflow_payload<T>(
        src: &mut T,
        page_size: u64,
        buf: &[u8],
        payload_length: u64,
    ) -> Result<Vec<u8>, CustomErr>
    where
        T: PageSource,
    {
        let overflow_ptr = u32::from_be_bytes(*buf.last_chunk::<4>().ok_or(
            CustomErr::Execution("Corrupted page: overflow payload".to_string()),
        )?) as u64;
        let mut payload_bytes: Vec<u8> = Vec::from(&buf[..buf.len() - 4]);
        let mut next_pointer: u64;

        loop {
            let next_buf = src.read_page(overflow_ptr, page_size)?;
            next_pointer = i32::from_be_bytes(next_buf[..4].try_into()?) as u64;
            payload_bytes.extend_from_slice(&next_buf[4..]);
            if next_pointer == 0 {
                break;
            };
        }
        let (payload_bytes, _) = payload_bytes.split_at(payload_length as usize);
        Ok(Vec::from(payload_bytes))
    }

    pub fn left_ptr(&self) -> Option<u64> {
        match self {
            Self::IdxInterior { left_child_ptr, .. } | Self::TblInterior { left_child_ptr, .. } => {
                Some(*left_child_ptr)
            }
            _ => None,
        }
    }

    pub fn indexed_value(&self) -> Option<ValueExpr> {
        match self {
            Self::IdxInterior { payload, .. } | Self::IdxLeaf { payload, .. } => {
                let col_value = payload.column(0).expect("Must be indexed value");
                ValueExpr::try_from(col_value).ok()
            }
            _ => None,
        }
    }

    pub fn row_id(&self) -> Option<u64> {
        match self {
            Self::TblInterior { row_id, .. } | Self::TblLeaf { row_id, .. } => Some(*row_id),
            Self::IdxInterior { payload, .. } | Self::IdxLeaf { payload, .. } => {
                let cell_value = payload.column(1).expect("Must be row_id");
                let row_id = cell_value.as_int().expect("Must be row_id") as u64;
                Some(row_id)
            }
        }
    }

    pub fn unpack_index(&self) -> Result<(u64, ValueExpr, u64), CustomErr> {
        let left_ptr = self.left_ptr();
        let indexed_value = self.indexed_value();
        let row_id = self.row_id();
        if left_ptr.is_some() && indexed_value.is_some() && row_id.is_some() {
            Ok((
                left_ptr.ok_or(CustomErr::Execution("Must be left_ptr".to_string()))?,
                indexed_value.ok_or(CustomErr::Execution("Must be indexed value".to_string()))?,
                row_id.ok_or(CustomErr::Execution("Must be row id".to_string()))?,
            ))
        } else {
            Err(CustomErr::Execution("Corruped index".to_string()))
        }
    }

    pub fn payload(&self) -> Option<&Payload> {
        match self {
            Self::IdxInterior { payload, .. }
            | Self::IdxLeaf { payload, .. }
            | Self::TblLeaf { payload, .. } => Some(payload),
            _ => None,
        }
    }
}
