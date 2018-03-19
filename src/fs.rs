use error::Error;
use buffer::Buffer;
use sys::superblock::Superblock;

/// Safe wrapper for raw sys structs
pub struct Ext2<B: Buffer<u8>> {
    buffer: B,
    superblock: Option<(Superblock, usize)>,
}

impl<B: Buffer<u8>> Ext2<B> {
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
