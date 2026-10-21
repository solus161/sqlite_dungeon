#[macro_use]
mod utils;
mod exceptions;
pub mod file_source;
mod parser;
pub mod processor;
pub mod page_source;
pub mod db;
mod data_types;

pub const TEST_DB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/companies.db");
