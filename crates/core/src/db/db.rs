use std::collections::HashMap;
use std::fmt::Debug;

use crate::db::page::Page;
use crate::{exceptions::CustomErr, page_source::PageSource};
use crate::db::page::{PageType, PageBuilder};
use crate::db::page_cell::PageCell;
use crate::parser::elements::{Table, TableBuilder, IndexBuilder};
use crate::parser::{Ast, ast_from_str};

pub struct DbHeader {
    pub page_count: u64,
    pub page_size: u64,
}

impl DbHeader {
    pub fn parse(buf: &[u8; 100]) -> Result<Self, CustomErr> {
        let raw_page_size = u16::from_be_bytes(
            buf[16..16 + 2]
            .try_into()
            .unwrap()
            ) as u64;
        let page_size = if raw_page_size == 1 {
            65536
        } else {
            raw_page_size
        };

        // Page count
        let page_count: u64 = u32::from_be_bytes(
            buf[16 + 12..16 + 12 + 4]
            .try_into().unwrap()
            ).into();

        Ok(DbHeader { page_count, page_size })
    }
}

impl Debug for DbHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DbHeader")
            .field("page_size", &self.page_size)
            .field("page_count", &self.page_count)
            .finish()
    }
}

pub struct Schema{
    pub tables: HashMap<String, Table>
}

impl Schema {
    pub fn new(page_01: &Page) -> Result<Self, CustomErr> {
        let mut builders_tbl: HashMap<String, TableBuilder> = HashMap::new();
        let mut builders_idx: Vec<IndexBuilder> = Vec::new();

        let cells = page_01.cells();
        for (offset, v) in cells.iter() {
            let Some(payload) = v.payload() else { continue };

            // sqlite_schema having columns in the following order
            // type, name, tbl_name, rootpage, sql
            // Let's building metadata

            let msg_col_index = "Invalid column index";
            let msg_type = "Wrong column type";

            // Type
            let type_col = payload
                .column(0)
                .ok_or(CustomErr::ValidateAST(msg_col_index.to_string()))?;
            let type_value = type_col
                .as_str()
                .ok_or(CustomErr::ValidateAST(msg_type.to_string()))?;

            // Page ptr
            let ptr_col = payload
                .column(3)
                .ok_or(CustomErr::ValidateAST(msg_col_index.to_string()))?;
            let ptr_value = ptr_col
                .as_int()
                .ok_or(CustomErr::ValidateAST(msg_type.to_string()))?;

            // Sql
            let sql_col = payload
                .column(4)
                .ok_or(CustomErr::ValidateAST(msg_col_index.to_string()))?;
            let sql_value = sql_col
                .as_str()
                .ok_or(CustomErr::ValidateAST(msg_type.to_string()))?;

            let Some(ast) = ast_from_str(sql_value)? else {
                continue;
            };
            match type_value {
                "table" => {
                    let mut builder = TableBuilder::new();
                    let Ast::CreateTable(stmt) = ast else {
                        return Err(CustomErr::ValidateAST("Wrong stmt type".to_string()));
                    };

                    let _ = builder.with_name(stmt.name())?;
                    let _ = builder.with_page_ptr(ptr_value as u64)?;
                    let _ = builder.with_offset(*offset)?;

                    let columns = stmt.columns();
                    for (i, c) in columns.iter().enumerate() {
                        let _ = builder.with_column(c.name(), i as u64)?;
                    }

                    builders_tbl.insert(stmt.name().to_string(), builder);
                }
                "index" => {
                    let mut builder = IndexBuilder::new();
                    let Ast::CreateIndex(stmt) = ast else {
                        return Err(CustomErr::ValidateAST("Wrong stmt type".to_string()));
                    };

                    builder.with_name(stmt.name());
                    builder.with_table(stmt.table());
                    builder.with_column(stmt.column());
                    builder.with_page_ptr(ptr_value as u64);
                    builders_idx.push(builder);
                }
                _ => continue,
            };
        }

        // Resolving index to table
        for b_idx in builders_idx.drain(..) {
            let (index, table, column) = b_idx.build()?;
            let Some(b_tbl) = builders_tbl.get_mut(&table) else {
                continue;
            };
            let _a = b_tbl.with_index(column.as_ref(), index);
        }

        // Builds tables
        let mut tables: HashMap<String, Table> = HashMap::new();
        for (k, v) in builders_tbl.drain() {
            let table = v.build()?;
            tables.insert(k, table);
        }
        Ok(Self { tables })
    }

    pub fn table(&self, name: &str) -> Option<&Table> {
        self.tables.get(name)
    }
}

/// Wrapper for source, header, and schema
/// Schema could be None
pub struct Db<S: PageSource> {
    pub src: S,
    pub header: DbHeader,
    pub schema: Option<Schema>,
}

impl<S: PageSource> Db<S> {
    pub fn open(mut src: S) -> Result<Self, CustomErr> {
        let mut buf = [0u8; 100];
        src.read_exact_at(&mut buf, 0)?;
        let header = DbHeader::parse(&buf)?;
        let mut db = Self { src, header, schema: None };
        let page_01 = db.parse_page(1)?;
        let schema = Schema::new(&page_01)?;
        db.schema = Some(schema);
        Ok(db)
        // Ok(Self { src, header, schema: () })
    }

    pub fn table_count(&self) -> usize {
        if let Some(schema) = &self.schema {
            schema.tables.len()
        } else {
            0
        }
    }

    pub fn table(&self, name: &str) -> Option<&Table> {
        if let Some(schema) = &self.schema {
            schema.table(name)
        } else {
            None
        }
    }

    /// Return Vec into &Table, immutable borrowing
    /// Insert/remove from schema must happen after this Vec is dropped
    pub fn tables(&self) -> Vec<&Table> {
        if let Some(schema) = &self.schema {
            schema.tables.values().into_iter().collect()
        } else {
            Vec::new()
        }
    }

    pub fn parse_page(&mut self, page_no: u64) -> Result<Page, CustomErr>
    where S: PageSource
    {
        let buf = self.src.read_page(page_no, self.header.page_size)?;
        let mut builder = PageBuilder::new(page_no);

        // First page, skip first 100 bytes
        let mut offset: usize = if page_no == 1 { 100 } else { 0 };

        // Parse the first page

        let page_type = PageType::try_from(get_offset(&buf, offset, 1)[0])?;
        offset += 1;

        // Start of freeblock
        let freeblock_offset = u16::from_be_bytes(get_offset(&buf, offset, 2).try_into()?) as u64;
        offset += 2;

        // Number of cell on page
        let cell_count = u16::from_be_bytes(get_offset(&buf, offset, 2).try_into()?) as u64;
        offset += 2;

        // Start of cell content area
        let cell_area_offset = u16::from_be_bytes(get_offset(&buf, offset, 2).try_into()?) as u64;
        offset += 2;

        // Number of fragmented free bytes
        let fragmented_freebyte_count =
            u8::from_be_bytes(get_offset(&buf, offset, 1).try_into()?) as u64;
        offset += 1;

        // Righ pointer, if any
        match &page_type {
            PageType::TblInterior | PageType::IdxInterior => {
                let right_ptr = u32::from_be_bytes(get_offset(&buf, 8, 4).try_into()?) as u64;
                offset += 4;
                builder.with_right_ptr(right_ptr);
            }
            _ => {}
        };

        // Cells offset, in increasing key order
        // physical position of cell does not matter
        for _ in 0..cell_count {
            let cell_offset = u16::from_be_bytes(get_offset(&buf, offset, 2).try_into()?) as u64;
            let cell = PageCell::new(
                &page_type, 
                &buf[cell_offset as usize..],
                &mut self.src,
                self.header.page_size
                )?;
            offset += 2;
            builder.with_cell(cell_offset, cell);
        }

        builder
            .with_page_type(page_type)
            .with_freeblock_offset(freeblock_offset)
            .with_cell_count(cell_count)
            .with_cell_area_offset(cell_area_offset)
            .with_fragmented_freebyte_count(fragmented_freebyte_count);

        let page = builder.build()?;
        Ok(page)
    }
}

fn get_offset(buf: &[u8], offset: usize, length: usize) -> &[u8] {
    &buf[offset..offset + length]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_source::FileSource;
    use crate::TEST_DB;

    #[test]
    fn test_parse_page_01() {
        let src = FileSource::new(TEST_DB);
        let mut db = Db::open(src).unwrap();
        let page_01 = db.parse_page(1);
        println!("Page 01: {:?}", page_01);
        assert!(page_01.is_ok());

        // Page 2, table companies
        let page_02 = db.parse_page(2);
        println!("Page 02: {:?}", page_02);
        assert!(page_02.is_ok());

        // Parse metadata
        println!("Parse meta result {:?}", db.header);

        // Index on country
        let page_04 = db.parse_page(4);
        println!("Page 04: {:?}", page_04);
    }
}
