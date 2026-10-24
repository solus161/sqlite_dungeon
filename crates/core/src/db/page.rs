use std::collections::HashMap;
use std::fmt::Display;

use super::page_cell::PageCell;
use crate::exceptions::CustomErr;

#[derive(Debug, PartialEq)]
pub enum PageType {
    TblInterior,
    IdxInterior,
    TblLeaf,
    IdxLeaf,
}

impl Eq for PageType {}

impl TryFrom<u8> for PageType {
    type Error = CustomErr;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x02 => Ok(Self::IdxInterior),
            0x05 => Ok(Self::TblInterior),
            0x0a => Ok(Self::IdxLeaf),
            0x0d => Ok(Self::TblLeaf),
            _ => Err(CustomErr::ParsePage("Invalid page type".to_string())),
        }
    }
}

pub struct PageBuilder {
    // All offset values is from beginning of page
    // so they never exceed 65536
    page_ptr: u64,
    page_type: Option<PageType>,
    freeblock_offset: Option<u64>,
    cell_count: Option<u64>,
    cell_area_offset: Option<u64>,
    fragmented_freebyte_count: Option<u64>,
    right_ptr: Option<u64>, // only in interior page
    cells_order: Vec<u64>,
    cells: HashMap<u64, PageCell>,
}

impl PageBuilder {
    pub fn new(page_ptr: u64) -> Self {
        Self {
            page_ptr,
            page_type: None,
            freeblock_offset: None,
            cell_count: None,
            cell_area_offset: None,
            fragmented_freebyte_count: None,
            right_ptr: None,
            cells_order: Vec::new(),
            cells: HashMap::new(),
        }
    }

    pub fn with_page_type(&mut self, page_type: PageType) -> &mut Self {
        self.page_type = Some(page_type);
        self
    }

    build_attr_u64!(freeblock_offset);
    build_attr_u64!(cell_count);
    build_attr_u64!(cell_area_offset);
    build_attr_u64!(fragmented_freebyte_count);
    build_attr_u64!(right_ptr);

    pub fn with_cell(&mut self, offset: u64, cell: PageCell) -> &mut Self {
        self.cells_order.push(offset);
        self.cells.insert(offset, cell);
        self
    }

    pub fn build(self) -> Result<Page, CustomErr> {
        Ok(Page {
            page_ptr: self.page_ptr,
            page_type: self
                .page_type
                .ok_or(CustomErr::ParsePage("Page type".to_string()))?,
            freeblock_offset: self
                .freeblock_offset
                .ok_or(CustomErr::ParsePage("Free block".to_string()))?,
            cell_count: self
                .cell_count
                .ok_or(CustomErr::ParsePage("Cell count".to_string()))?,
            cell_area_offset: self
                .cell_area_offset
                .ok_or(CustomErr::ParsePage("Cell area offset".to_string()))?,
            fragmented_freebyte_count: self.fragmented_freebyte_count.ok_or(
                CustomErr::ParsePage("Fragmented freebyte count".to_string()),
            )?,
            right_ptr: self.right_ptr,
            cells_order: self.cells_order,
            cells: self.cells,
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct Page {
    page_ptr: u64,
    page_type: PageType,
    freeblock_offset: u64,
    cell_count: u64,
    cell_area_offset: u64,
    fragmented_freebyte_count: u64,
    right_ptr: Option<u64>,
    cells_order: Vec<u64>,
    cells: HashMap<u64, PageCell>,
}

impl Eq for Page {}

impl Page {
    get_attr_u64!(page_ptr);
    get_attr_u64!(freeblock_offset);
    get_attr_u64!(cell_count);
    get_attr_u64!(cell_area_offset);
    get_attr_u64!(fragmented_freebyte_count);

    pub fn page_type(&self) -> &PageType {
        &self.page_type
    }

    pub fn right_ptr(&self) -> Option<&u64> {
        self.right_ptr.as_ref()
    }

    pub fn cells_order(&self) -> &[u64] {
        &self.cells_order
    }

    pub fn cells(&self) -> &HashMap<u64, PageCell> {
        &self.cells
    }

    pub fn cell(&self, offset: &u64) -> Result<&PageCell, CustomErr> {
        self.cells
            .get(offset)
            .ok_or(CustomErr::Execution("Invalid cell offset".to_string()))
    }
}

impl Display for Page {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Page pointer: {}", self.page_ptr)?;
        writeln!(f, "Page type: {:?}", self.page_type)?;
        writeln!(f, "Freeblock offset: {}", self.freeblock_offset)?;
        writeln!(f, "Cell count: {}", self.cell_count)?;
        writeln!(f, "Cell area offset: {}", self.cell_area_offset)?;
        writeln!(
            f,
            "Fragmented free byte count: {}",
            self.fragmented_freebyte_count
        )?;
        writeln!(f, "Right pointer: {:?}", self.right_ptr)?;
        writeln!(f, "Cells___________________________________________")?;
        self.cells_order.iter().try_for_each(|i| {
            if let Some(cell) = self.cells.get(i) {
                writeln!(f, "offset {} {:?}", i, cell)
            } else {
                writeln!(f, "none")
            }
        })
    }
}
