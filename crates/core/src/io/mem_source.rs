use crate::exceptions::CustomErr;
use std::fmt::Debug;

use super::page_source::PageSource;

/// A wrapper to read file when served through browser
pub struct MemSource {
    src: Vec<u8>,
}

impl MemSource {
    /// TODO: src may be st else, `Vec<u8>` is just a placeholder
    pub fn new(src: Vec<u8>) -> Result<Self, CustomErr> {
        Ok(Self { src })
    }
}

impl Debug for MemSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemSource").field("src", &self.src).finish()
    }
}

impl PageSource for MemSource {
    /// TODO: this is just a placeholder
    fn read_exact_at(&mut self, buf: &mut [u8], offset: u64) -> Result<(), CustomErr> {
        let dst_len = buf.len();
        let copied_len = self.src.len() - offset as usize;
        if dst_len <= copied_len {
            buf.copy_from_slice(&self.src[..dst_len]);
            Ok(())
        } else {
            Err(CustomErr::IOError)
        }
    }

    fn from_cache(&self, page_number: u64) -> Option<&[u8]> {
        None
    }

    fn to_cache(&mut self, buf: &[u8]) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mem_source_read_exact_at() {
        let src = vec![1u8; 8];
        let mut mem = MemSource::new(src).unwrap();

        // Dest len > src len
        let mut buf1 = [0u8; 4];
        let offset: usize = 5;
        let o = mem.read_exact_at(buf1.as_mut_slice(), offset as u64);
        assert!(o.is_err());

        // Dest len <= src len
        let mut buf2 = [0u8; 4];
        let offset: usize = 2;
        mem.read_exact_at(buf2.as_mut_slice(), offset as u64)
            .unwrap();
        buf2.iter().for_each(|x| assert_eq!(*x, 1u8));
    }
}
