use crate::exceptions::CustomErr;

/// An interface for file served through browser or through on-disk read
pub trait PageSource {
    /// Read into a buf starting from an offset
    /// TODO: handling error
    fn read_exact_at(&mut self, buf: &mut [u8], offset: u64) -> Result<(), CustomErr>;

    /// Returns a page, page number starts from 1
    /// Default impl on top of read_exact_at, so sources only implement that
    fn read_page(&mut self, page_number: u64, page_size: u64) -> Result<Vec<u8>, CustomErr> {
        // Page 0 does not exist, also guards page_number - 1 from underflow
        if page_number == 0 {
            return Err(CustomErr::ReadPage("Page number starts from 1".to_string()));
        }
        if let Some(page) = self.from_cache(page_number) {
            Ok(page.into())
        } else {
            let offset = (page_number - 1) * page_size;
            let mut buf = vec![0u8; page_size as usize];
            self.read_exact_at(&mut buf, offset)?;
            Ok(buf)
        }
    }

    /// Return a byte page from cache if any
    fn from_cache(&self, page_number: u64) -> Option<&[u8]>;

    /// Save a byte page to cache
    fn to_cache(&mut self, buf: &[u8]);
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal in-memory source, only for testing the default read_page
    struct VecSource(Vec<u8>);

    impl PageSource for VecSource {
        fn read_exact_at(&mut self, buf: &mut [u8], offset: u64) -> Result<(), CustomErr> {
            let start = offset as usize;
            let src = self
                .0
                .get(start..start + buf.len())
                .ok_or(CustomErr::IOError)?;
            buf.copy_from_slice(src);
            Ok(())
        }

        fn from_cache(&self, _page_number: u64) -> Option<&[u8]> {
            None
        }

        fn to_cache(&mut self, _buf: &[u8]) {}
    }

    #[test]
    fn test_read_page() {
        // 2 pages of 4 bytes each
        let mut src = VecSource(vec![1, 1, 1, 1, 2, 2, 2, 2]);
        assert_eq!(src.read_page(1, 4).unwrap(), vec![1, 1, 1, 1]);
        assert_eq!(src.read_page(2, 4).unwrap(), vec![2, 2, 2, 2]);

        // Page 0 does not exist
        assert!(matches!(src.read_page(0, 4), Err(CustomErr::ReadPage(_))));

        // Past the end of source
        assert!(src.read_page(3, 4).is_err());
    }
}
