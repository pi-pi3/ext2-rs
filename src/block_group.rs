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
