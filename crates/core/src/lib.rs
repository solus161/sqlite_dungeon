#[macro_use]
mod utils;
mod data_types;
pub mod db;
pub mod exceptions;
pub mod io;
mod parser;
pub mod processor;

pub const TEST_DB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/companies.db");
