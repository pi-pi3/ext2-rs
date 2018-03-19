use error::Error;
use buffer::Buffer;
use sys::superblock::Superblock;

/// Safe wrapper for raw sys structs
pub struct Ext2<B: Buffer<u8>> {
    buffer: B,
    superblock: Option<(Superblock, usize)>,
}

impl<B: Buffer<u8>> Ext2<B>
where
    Error: From<B::Error>,
{
    pub fn new(buffer: B) -> Ext2<B> {
        Ext2 {
            buffer,
            superblock: None,
        }
    }

    pub fn init(&mut self) -> Result<(), Error> {
        let superblock = Superblock::find(&self.buffer);
        match superblock {
            Ok(sb) => Ok(self.superblock = Some(sb)),
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::cell::RefCell;

    use super::Ext2;

    #[test]
    fn file() {
        let file = RefCell::new(File::open("ext2.bin").unwrap());
        let mut fs = Ext2::new(file);
        assert!(fs.init().is_ok());
    }
}
