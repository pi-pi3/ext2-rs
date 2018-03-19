use alloc::Vec;

use error::Error;
use buffer::{Buffer, BufferSlice};
use sys::superblock::Superblock;
use sys::block_group::BlockGroupDescriptor;

struct Struct<T> {
    pub inner: T,
    pub offset: usize,
}

impl<T> From<(T, usize)> for Struct<T> {
    #[inline]
    fn from((inner, offset): (T, usize)) -> Struct<T> {
        Struct { inner, offset }
    }
}

/// Safe wrapper for raw sys structs
pub struct Ext2<B: Buffer<u8>> {
    buffer: B,
    superblock: Struct<Superblock>,
    block_groups: Struct<Vec<BlockGroupDescriptor>>,
}

impl<B: Buffer<u8>> Ext2<B>
where
    Error: From<B::Error>,
{
    pub fn new(buffer: B) -> Result<Ext2<B>, Error> {
        let superblock = Struct::from(Superblock::find(&buffer)?);
        let block_size = superblock.inner.block_size();
        let block_groups_offset =
            (superblock.inner.first_data_block as usize + 1) * block_size;
        let block_groups_count = superblock
            .inner
            .block_group_count()
            .map(|count| count as usize)
            .map_err(|(a, b)| Error::BadBlockGroupCount(a, b))?;
        let block_groups = BlockGroupDescriptor::find_descriptor_table(
            &buffer,
            block_groups_offset,
            block_groups_count,
        )?;
        let block_groups = Struct::from(block_groups);
        Ok(Ext2 {
            buffer,
            superblock,
            block_groups,
        })
    }

    pub fn update(&mut self) -> Result<(), Error> {
        let slice = BufferSlice::from_cast(
            &self.superblock.inner,
            self.superblock.offset,
        );
        let commit = slice.commit();
        self.buffer.commit(commit).map_err(|err| Error::from(err))
    }

    fn superblock(&self) -> &Superblock {
        &self.superblock.inner
    }

    fn superblock_mut(&mut self) -> &mut Superblock {
        &mut self.superblock.inner
    }

    pub fn block_group_count(&self) -> Result<usize, Error> {
        self.superblock()
            .block_group_count()
            .map(|count| count as usize)
            .map_err(|(a, b)| Error::BadBlockGroupCount(a, b))
    }

    pub fn total_block_count(&self) -> usize {
        self.superblock().blocks_count as _
    }

    pub fn free_block_count(&self) -> usize {
        self.superblock().free_blocks_count as _
    }

    pub fn block_size(&self) -> usize {
        self.superblock().block_size()
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::cell::RefCell;

    use buffer::Buffer;

    use super::Ext2;

    #[test]
    fn file_len() {
        let file = RefCell::new(File::open("ext2.bin").unwrap());
        assert_eq!(unsafe { file.slice_unchecked(1024..2048).len() }, 1024);
    }

    #[test]
    fn file() {
        let file = RefCell::new(File::open("ext2.bin").unwrap());
        let fs = Ext2::new(file);
        assert_eq!(Ok(()), fs.map(|_| ()));
    }
}
