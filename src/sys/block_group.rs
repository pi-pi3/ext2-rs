use core::mem;

use alloc::Vec;

use error::Error;
use buffer::Buffer;

/// The Block Group Descriptor Table contains a descriptor for each block group
/// within the file system. The number of block groups within the file system,
/// and correspondingly, the number of entries in the Block Group Descriptor
/// Table, is described above. Each descriptor contains information regarding
/// where important data structures for that group are located.
///
/// The (`BlockGroupDescriptor`) table is located in the block immediately
/// following the Superblock. So if the block size (determined from a field in
/// the superblock) is 1024 bytes per block, the Block Group Descriptor Table
/// will begin at block 2. For any other block size, it will begin at block 1.
/// Remember that blocks are numbered starting at 0, and that block numbers
/// don't usually correspond to physical block addresses.
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct BlockGroupDescriptor {
    /// Block address of block usage bitmap
    pub block_usage_addr: u32,
    /// Block address of inode usage bitmap
    pub inode_usage_addr: u32,
    /// Starting block address of inode table
    pub inode_table_block: u32,
    /// Number of unallocated blocks in group
    pub free_blocks_count: u16,
    /// Number of unallocated inodes in group
    pub free_inodes_count: u16,
    /// Number of directories in group
    pub dirs_count: u16,
    #[doc(hidden)]
    _reserved: [u8; 14],
}

impl BlockGroupDescriptor {
    pub unsafe fn find_descriptor<'a, E>(
        haystack: &'a Buffer<u8, Error = E>,
        offset: usize,
    ) -> Result<(BlockGroupDescriptor, usize), Error>
    where
        Error: From<E>,
    {
        let end = offset + mem::size_of::<BlockGroupDescriptor>();
        if haystack.len() < end {
            return Err(Error::OutOfBounds(end));
        }

        let descr = haystack
            .slice_unchecked(offset..end)
            .dynamic_cast::<BlockGroupDescriptor>();

        Ok(descr)
    }

    pub unsafe fn find_descriptor_table<'a, E>(
        haystack: &'a Buffer<u8, Error = E>,
        offset: usize,
        count: usize,
    ) -> Result<(Vec<BlockGroupDescriptor>, usize), Error>
    where
        Error: From<E>,
    {
        let end = offset + count * mem::size_of::<BlockGroupDescriptor>();
        if haystack.len() < end {
            return Err(Error::OutOfBounds(end));
        }

        let mut vec = Vec::with_capacity(count);
        for i in 0..count {
            let offset = offset + i * mem::size_of::<BlockGroupDescriptor>();
            vec.push({
                BlockGroupDescriptor::find_descriptor(haystack, offset)?.0
            });
        }

        Ok((vec, offset))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find() {
        let buffer = vec![0_u8; 4096];
        let table = unsafe {
            BlockGroupDescriptor::find_descriptor_table(&buffer, 2048, 8)
        };
        assert!(
            table.is_ok(),
            "Err({:?})",
            table.err().unwrap_or_else(|| unreachable!()),
        );
        let table = table.unwrap_or_else(|_| unreachable!());
        assert_eq!(table.0.len(), 8);
    }
}
