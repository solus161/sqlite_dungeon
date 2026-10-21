use std::collections::HashMap;
use std::rc::Rc;

use crate::exceptions::CustomErr;

// Table, created from AST
#[derive(Debug)]
pub struct Table {
    name: String,
    offset: u64, // offset bytes on page 1
    page_ptr: u64,
    columns_ord: Vec<Rc<str>>,
    columns: HashMap<Rc<str>, u64>, // column - order
    indexes: HashMap<Rc<str>, Index>,
}

impl Table {
    pub fn new(
        name: String,
        offset: u64,
        page_ptr: u64,
        columns_ord: Vec<Rc<str>>,
        columns: HashMap<Rc<str>, u64>,
        indexes: HashMap<Rc<str>, Index>,
    ) -> Self {
        Self {
            name,
            offset,
            page_ptr,
            columns_ord,
            columns,
            indexes,
        }
    }

    get_attr_str!(name);
    get_attr_u64!(offset);
    get_attr_u64!(page_ptr);

    pub fn index(&self, column_name: &str) -> Option<&Index> {
        self.indexes.get(&Rc::from(column_name))
    }

    pub fn column_index(&self, column_name: &str) -> Option<&u64> {
        self.columns.get(&Rc::from(column_name))
    }

    pub fn columns_ord(&self) -> &[Rc<str>] {
        self.columns_ord.as_ref()
    }
}

// Table Builder
pub struct TableBuilder {
    name: Option<String>,
    offset: Option<u64>,
    page_ptr: Option<u64>,
    columns_ord: Option<Vec<Rc<str>>>,
    columns: Option<HashMap<Rc<str>, u64>>,
    indexes: Option<HashMap<Rc<str>, Index>>,
}

impl TableBuilder {
    pub fn new() -> Self {
        Self {
            name: None,
            offset: None,
            page_ptr: None,
            columns_ord: None,
            columns: None,
            indexes: None,
        }
    }

    pub fn with_name(&mut self, name: &str) -> Result<&mut Self, CustomErr> {
        self.name = Some(name.to_string());
        Ok(self)
    }

    pub fn with_offset(&mut self, offset: u64) -> Result<&mut Self, CustomErr> {
        self.offset = Some(offset);
        Ok(self)
    }

    pub fn with_page_ptr(&mut self, page_ptr: u64) -> Result<&mut Self, CustomErr> {
        self.page_ptr = Some(page_ptr);
        Ok(self)
    }

    pub fn with_column(&mut self, column_name: &str, ord: u64) -> Result<&mut Self, CustomErr> {
        let column_rc: Rc<str> = Rc::from(column_name);
        self.columns_ord
            .get_or_insert_default()
            .push(column_rc.clone());
        let _ = self.columns.get_or_insert_default().insert(column_rc, ord);
        Ok(self)
    }

    pub fn with_index(&mut self, column_name: &str, index: Index) -> Result<&mut Self, CustomErr> {
        let column_rc: Rc<str> = Rc::from(column_name);
        let Some(set) = &mut self.columns else {
            return Err(CustomErr::ValidateAST(format!(
                "Column named {} does not exist",
                column_name
            )));
        };
        let Some((k, _)) = set.get_key_value(&column_rc) else {
            return Err(CustomErr::ValidateAST(format!(
                "Column named {} does not exist",
                column_name
            )));
        };
        let _ = self
            .indexes
            .get_or_insert_default()
            .insert(k.clone(), index);
        Ok(self)
    }

    pub fn build(self) -> Result<Table, CustomErr> {
        Ok(Table::new(
            self.name.ok_or(CustomErr::ValidateAST(
                "Table name not provided".to_string(),
            ))?,
            self.offset.ok_or(CustomErr::ValidateAST(
                "Cell offset not provided".to_string(),
            ))?,
            self.page_ptr.ok_or(CustomErr::ValidateAST(
                "Page pointer not provided".to_string(),
            ))?,
            self.columns_ord
                .ok_or(CustomErr::ValidateAST("Columns not provided".to_string()))?,
            self.columns
                .ok_or(CustomErr::ValidateAST("Columns not provided".to_string()))?,
            self.indexes.unwrap_or_default(),
        ))
    }
}

#[derive(Debug)]
pub struct Index {
    name: String,
    page_ptr: u64,
}

impl Index {
    pub fn new(name: String, page_ptr: u64) -> Self {
        Self { name, page_ptr }
    }

    get_attr_str!(name);
    get_attr_u64!(page_ptr);
}

pub struct IndexBuilder {
    name: Option<String>,
    table: Option<String>,
    column: Option<String>,
    page_ptr: Option<u64>,
}

impl IndexBuilder {
    pub fn new() -> Self {
        Self {
            name: None,
            table: None,
            column: None,
            page_ptr: None,
        }
    }

    build_attr_String!(name);
    build_attr_String!(table);
    build_attr_String!(column);
    build_attr_u64!(page_ptr);

    pub fn build(self) -> Result<(Index, String, String), CustomErr> {
        // Return Index, table name, column name
        let index = Index::new(
            self.name.ok_or(CustomErr::ValidateAST(
                "Index name not provided".to_string(),
            ))?,
            self.page_ptr.ok_or(CustomErr::ValidateAST(
                "Page pointer not provided".to_string(),
            ))?,
        );
        let Some(table) = self.table else {
            return Err(CustomErr::ValidateAST(
                "Indexed table not provided".to_string(),
            ));
        };
        let Some(column) = self.column else {
            return Err(CustomErr::ValidateAST(
                "Indexed column not provided".to_string(),
            ));
        };
        Ok((index, table, column))
    }
}
