use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;
use std::fs::File;
use std::os::unix::fs::FileExt;

use super::lru_cache::LRUCache;
use super::page_source::PageSource;
use crate::exceptions::CustomErr;

pub struct FileSource {
    src: File,
    cache: LRUCache,
}

impl FileSource {
    pub fn new(file_path: &str) -> Result<Self, CustomErr> {
        let file_obj = File::open(file_path).map_err(|_| CustomErr::IOError)?;

        Ok(Self {
            src: file_obj,
            cache: LRUCache::new(1000),
        })
    }
}

impl Debug for FileSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileSource")
            .field("src", &self.src)
            .finish()
    }
}

impl PageSource for FileSource {
    /// This must mirror `std::io::unix::fx:FileExt::read_exact_at` behavior
    /// It never read more bytes than needed to fill buf,
    /// but error if read pass EoF
    fn read_exact_at(&mut self, buf: &mut [u8], offset: u64) -> Result<(), CustomErr> {
        self.src
            .read_exact_at(buf, offset)
            .map_err(|_| CustomErr::IOError)?;
        Ok(())
    }

    fn from_cache(&self, page_number: u64) -> Option<&[u8]> {
        None
    }

    fn to_cache(&mut self, buf: &[u8]) {}
}
