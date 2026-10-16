#[macro_use]
mod utils;
pub mod btree;
mod exceptions;
pub mod pager;
mod parser;
pub mod processor;

pub const TEST_DB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/companies.db");
