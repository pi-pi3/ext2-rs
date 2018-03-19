use core::mem;
use alloc::Vec;

use error::Error;
use buffer::{Buffer, BufferSlice};
use sys::superblock::Superblock;
use sys::block_group::BlockGroupDescriptor;
use sys::inode::Inode;

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
        let superblock = unsafe { Struct::from(Superblock::find(&buffer)?) };
        let block_size = superblock.inner.block_size();
        let block_groups_offset =
            (superblock.inner.first_data_block as usize + 1) * block_size;
        let block_groups_count = superblock
            .inner
            .block_group_count()
            .map(|count| count as usize)
            .map_err(|(a, b)| Error::BadBlockGroupCount(a, b))?;
        let block_groups = unsafe {
            BlockGroupDescriptor::find_descriptor_table(
                &buffer,
                block_groups_offset,
                block_groups_count,
            )?
        };
        let block_groups = Struct::from(block_groups);
        Ok(Ext2 {
            buffer,
            superblock,
            block_groups,
        })
    }

    #[allow(dead_code)]
    fn update_global(&mut self) -> Result<(), Error> {
        // superblock
        {
            let slice = BufferSlice::from_cast(
                &self.superblock.inner,
                self.superblock.offset,
            );
            let commit = slice.commit();
            self.buffer.commit(commit).map_err(|err| Error::from(err))?;
        }

        // block group descriptors
        let mut offset = self.block_groups.offset;
        for descr in &self.block_groups.inner {
            let slice = BufferSlice::from_cast(descr, offset);
            let commit = slice.commit();
            self.buffer.commit(commit).map_err(|err| Error::from(err))?;
            offset += mem::size_of::<BlockGroupDescriptor>();
        }

        Ok(())
    }

    #[allow(dead_code)]
    fn update_inode(
        &mut self,
        &(ref inode, offset): &(Inode, usize),
    ) -> Result<(), Error> {
        let slice = BufferSlice::from_cast(inode, offset);
        let commit = slice.commit();
        self.buffer.commit(commit).map_err(|err| Error::from(err))
    }

    pub fn root_inode(&self) -> (Inode, usize) {
        self.inode_nth(2).unwrap()
    }

    pub fn inode_nth(&self, index: usize) -> Option<(Inode, usize)> {
        self.inodes_nth(index).next()
    }

    pub fn inodes<'a>(&'a self) -> Inodes<'a, B> {
        self.inodes_nth(1)
    }

    pub fn inodes_nth<'a>(&'a self, index: usize) -> Inodes<'a, B> {
        assert!(index > 0, "inodes are 1-indexed");
        Inodes {
            buffer: &self.buffer,
            block_groups: &self.block_groups.inner,
            block_size: self.block_size(),
            inode_size: self.inode_size(),
            inodes_per_group: self.inodes_count(),
            inodes_count: self.total_inodes_count(),
            index,
        }
    }

    fn superblock(&self) -> &Superblock {
        &self.superblock.inner
    }

    #[allow(dead_code)]
    fn superblock_mut(&mut self) -> &mut Superblock {
        &mut self.superblock.inner
    }

    pub fn version(&self) -> (u32, u16) {
        (self.superblock().rev_major, self.superblock().rev_minor)
    }

    pub fn inode_size(&self) -> usize {
        if self.version().0 == 0 {
            mem::size_of::<Inode>()
        } else {
            // note: inodes bigger than 128 are not supported
            self.superblock().inode_size as usize
        }
    }

    pub fn inodes_count(&self) -> usize {
        self.superblock().inodes_per_group as _
    }

    pub fn total_inodes_count(&self) -> usize {
        self.superblock().inodes_count as _
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

pub struct Inodes<'a, B: 'a + Buffer<u8>> {
    buffer: &'a B,
    block_groups: &'a [BlockGroupDescriptor],
    block_size: usize,
    inode_size: usize,
    inodes_per_group: usize,
    inodes_count: usize,
    index: usize,
}

impl<'a, B: 'a + Buffer<u8>> Iterator for Inodes<'a, B>
where
    Error: From<B::Error>,
{
    type Item = (Inode, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.inodes_count {
            let block_group = (self.index - 1) / self.inodes_per_group;
            let index = (self.index - 1) % self.inodes_per_group;
            self.index += 1;

            let inodes_block =
                self.block_groups[block_group].inode_table_block as usize;

            let offset =
                inodes_block * self.block_size + index * self.inode_size;
            unsafe {
                Inode::find_inode(self.buffer, offset, self.inode_size).ok()
            }
        } else {
            None
        }
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

        assert!(
            fs.is_ok(),
            "Err({:?})",
            fs.err().unwrap_or_else(|| unreachable!()),
        );

        let fs = fs.unwrap();

        let vers = fs.version();
        println!("version: {}.{}", vers.0, vers.1);
        assert_eq!(128, fs.inode_size());
    }

    #[test]
    fn inodes() {
        let file = RefCell::new(File::open("ext2.bin").unwrap());
        let fs = Ext2::new(file);

        assert!(
            fs.is_ok(),
            "Err({:?})",
            fs.err().unwrap_or_else(|| unreachable!()),
        );

        let fs = fs.unwrap();

        let inodes = fs.inodes().filter(|inode| inode.0.in_use());
        for inode in inodes {
            println!("{:?}", inode);
        }
    }
}
