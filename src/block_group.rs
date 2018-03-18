#[cfg(test)]
use std::mem;
#[cfg(test)]
use std::slice;

#[cfg(not(test))]
use core::mem;
#[cfg(not(test))]
use core::slice;

use error::Error;

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
pub struct BlockGroupDescriptor {
    /// Block address of block usage bitmap
    block_usage_addr: u32,
    /// Block address of inode usage bitmap
    inode_usage_addr: u32,
    /// Starting block address of inode table
    inode_table_block: u32,
    /// Number of unallocated blocks in group
    free_blocks_count: u16,
    /// Number of unallocated inodes in group
    free_inodes_count: u16,
    /// Number of directories in group
    dirs_count: u16,
    #[doc(hidden)]
    _reserved: [u8; 14],
}

impl BlockGroupDescriptor {
    pub fn find_descriptor_table<'a>(
        haystack: &'a mut [u8],
        offset: isize,
        count: usize,
    ) -> Result<&'a mut [BlockGroupDescriptor], Error> {
        let offset = (2048 + offset) as usize;
        let end = offset + count * mem::size_of::<BlockGroupDescriptor>();
        if haystack.len() < end {
            return Err(Error::OutOfBounds(end));
        }

        let ptr = unsafe {
            haystack.as_mut_ptr().offset(offset as isize)
                as *mut BlockGroupDescriptor
        };
        let slice = unsafe { slice::from_raw_parts_mut(ptr, count) };
        Ok(slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find() {
        let mut buffer = vec![0_u8; 4096];
        let addr = &buffer[2048] as *const _ as usize;
        // magic
        let table =
            BlockGroupDescriptor::find_descriptor_table(&mut buffer, 0, 0);
        assert!(
            table.is_ok(),
            "Err({:?})",
            table.err().unwrap_or_else(|| unreachable!()),
        );
        assert_eq!(table.unwrap().as_ptr() as usize, addr);
    }
}
