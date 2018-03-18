#[cfg(test)]
use std::mem;
#[cfg(test)]
use std::slice;

#[cfg(not(test))]
use core::mem;
#[cfg(not(test))]
use core::slice;

use error::Error;
use block_group::BlockGroupDescriptor;

/// Ext2 signature (0xef53), used to help confirm the presence of Ext2 on a
/// volume
pub const EXT2_MAGIC: u16 = 0xef53;

/// Filesystem is free of errors
pub const FS_CLEAN: u16 = 1;
/// Filesystem has errors
pub const FS_ERR: u16 = 2;

/// Ignore errors
pub const ERR_IGNORE: u16 = 1;
/// Remount as read-only on error
pub const ERR_RONLY: u16 = 2;
/// Panic on error
pub const ERR_PANIC: u16 = 3;

/// Creator OS is Linux
pub const OS_LINUX: u32 = 0;
/// Creator OS is Hurd
pub const OS_HURD: u32 = 1;
/// Creator OS is Masix
pub const OS_MASIX: u32 = 2;
/// Creator OS is FreeBSD
pub const OS_FREEBSD: u32 = 3;
/// Creator OS is a BSD4.4-Lite derivative
pub const OS_LITE: u32 = 4;

/// The Superblock contains all information about the layout of the file system
/// and possibly contains other important information like what optional
/// features were used to create the file system.
///
/// The Superblock is always located at byte 1024 from the beginning of the
/// volume and is exactly 1024 bytes in length. For example, if the disk uses
/// 512 byte sectors, the Superblock will begin at LBA 2 and will occupy all of
/// sector 2 and 3.
#[repr(C, packed)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Superblock {
    // taken from https://wiki.osdev.org/Ext2
    /// Total number of inodes in file system
    inodes_count: u32,
    /// Total number of blocks in file system
    blocks_count: u32,
    /// Number of blocks reserved for superuser (see offset 80)
    r_blocks_count: u32,
    /// Total number of unallocated blocks
    free_blocks_count: u32,
    /// Total number of unallocated inodes
    free_inodes_count: u32,
    /// Block number of the block containing the superblock
    first_data_block: u32,
    /// log2 (block size) - 10. (In other words, the number to shift 1,024
    /// to the left by to obtain the block size)
    log_block_size: u32,
    /// log2 (fragment size) - 10. (In other words, the number to shift
    /// 1,024 to the left by to obtain the fragment size)
    log_frag_size: i32,
    /// Number of blocks in each block group
    blocks_per_group: u32,
    /// Number of fragments in each block group
    frags_per_group: u32,
    /// Number of inodes in each block group
    inodes_per_group: u32,
    /// Last mount time (in POSIX time)
    mtime: u32,
    /// Last written time (in POSIX time)
    wtime: u32,
    /// Number of times the volume has been mounted since its last
    /// consistency check (fsck)
    mnt_count: u16,
    /// Number of mounts allowed before a consistency check (fsck) must be
    /// done
    max_mnt_count: i16,
    /// Ext2 signature (0xef53), used to help confirm the presence of Ext2
    /// on a volume
    magic: u16,
    /// File system state (see `FS_CLEAN` and `FS_ERR`)
    state: u16,
    /// What to do when an error is detected (see `ERR_IGNORE`, `ERR_RONLY` and
    /// `ERR_PANIC`)
    errors: u16,
    /// Minor portion of version (combine with Major portion below to
    /// construct full version field)
    rev_minor: u16,
    /// POSIX time of last consistency check (fsck)
    lastcheck: u32,
    /// Interval (in POSIX time) between forced consistency checks (fsck)
    checkinterval: u32,
    /// Operating system ID from which the filesystem on this volume was
    /// created
    creator_os: u32,
    /// Major portion of version (combine with Minor portion above to
    /// construct full version field)
    rev_major: u32,
    /// User ID that can use reserved blocks
    block_uid: u16,
    /// Group ID that can use reserved blocks
    block_gid: u16,

    /// First non-reserved inode in file system.
    first_inode: u32,
    /// Size of each inode structure in bytes.
    inode_size: u16,
    /// Block group that this superblock is part of (if backup copy)
    block_group: u16,
    /// Optional features present (features that are not required to read
    /// or write, but usually result in a performance increase)
    features_opt: FeaturesOptional,
    /// Required features present (features that are required to be
    /// supported to read or write)
    features_req: FeaturesRequired,
    /// Features that if not supported, the volume must be mounted
    /// read-only)
    features_ronly: FeaturesROnly,
    /// File system ID (what is output by blkid)
    fs_id: [u8; 16],
    /// Volume name (C-style string: characters terminated by a 0 byte)
    volume_name: [u8; 16],
    /// Path volume was last mounted to (C-style string: characters
    /// terminated by a 0 byte)
    last_mnt_path: [u8; 64],
    /// Compression algorithms used (see Required features above)
    compression: u32,
    /// Number of blocks to preallocate for files
    prealloc_blocks_files: u8,
    /// Number of blocks to preallocate for directories
    prealloc_blocks_dirs: u8,
    #[doc(hidden)]
    _unused: [u8; 2],
    /// Journal ID (same style as the File system ID above)
    journal_id: [u8; 16],
    /// Journal inode
    journal_inode: u32,
    /// Journal device
    journal_dev: u32,
    /// Head of orphan inode list
    journal_orphan_head: u32,
    #[doc(hidden)]
    _reserved: [u8; 788],
}

impl Superblock {
    pub fn find<'a>(
        haystack: &'a mut [u8],
    ) -> Result<&'a mut Superblock, Error> {
        let offset = 1024;
        let end = offset + mem::size_of::<Superblock>();
        if haystack.len() < end {
            return Err(Error::OutOfBounds(end));
        }

        let superblock: &mut Superblock = unsafe {
            let ptr =
                haystack.as_ptr().offset(offset as isize) as *mut Superblock;
            ptr.as_mut().unwrap()
        };

        if superblock.magic != EXT2_MAGIC {
            Err(Error::BadMagic(superblock.magic))
        } else {
            Ok(superblock)
        }
    }

    pub fn find_block_table<'a>(
        &self,
        haystack: &'a mut [u8],
    ) -> Result<&'a mut [BlockGroupDescriptor], Error> {
        let count = self.block_group_count()
            .map_err(|(a, b)| Error::BadBlockGroupCount(a, b))?
            as usize;

        let offset = 2048;
        let end = offset + count * mem::size_of::<BlockGroupDescriptor>();
        if haystack.len() < end {
            return Err(Error::OutOfBounds(end));
        }

        let ptr = unsafe {
            haystack.as_ptr().offset(offset as isize)
                as *mut BlockGroupDescriptor
        };
        let slice = unsafe { slice::from_raw_parts_mut(ptr, count) };
        Ok(slice)
    }

    #[inline]
    pub fn block_size(&self) -> usize {
        1024 << self.log_block_size
    }

    #[inline]
    pub fn frag_size(&self) -> usize {
        1024 << self.log_frag_size
    }

    pub fn block_group_count(&self) -> Result<u32, (u32, u32)> {
        let blocks_mod = self.blocks_count % self.blocks_per_group;
        let inodes_mod = self.inodes_count % self.inodes_per_group;
        let blocks_inc = if blocks_mod == 0 { 0 } else { 1 };
        let inodes_inc = if inodes_mod == 0 { 0 } else { 1 };
        let by_blocks = self.blocks_count / self.blocks_per_group + blocks_inc;
        let by_inodes = self.inodes_count / self.inodes_per_group + inodes_inc;
        if by_blocks == by_inodes {
            Ok(by_blocks)
        } else {
            Err((by_blocks, by_inodes))
        }
    }
}

bitflags! {
    /// Optional features
    pub struct FeaturesOptional: u32 {
        /// Preallocate some number of (contiguous?) blocks (see
        /// `Superblock::prealloc_blocks_dirs`) to a directory when creating a new one
        const PREALLOCATE = 0x0001;
        /// AFS server inodes exist
        const AFS = 0x0002;
        /// File system has a journal (Ext3)
        const JOURNAL = 0x0004;
        /// Inodes have extended attributes
        const EXTENDED_INODE = 0x0008;
        /// File system can resize itself for larger partitions
        const SELF_RESIZE = 0x0010;
        /// Directories use hash index
        const HASH_INDEX = 0x0020;
    }
}

bitflags! {
    /// Required features. If these are not supported; can't mount
    pub struct FeaturesRequired: u32 {
        /// Compression is used
        const REQ_COMPRESSION = 0x0001;
        /// Directory entries contain a type field
        const REQ_DIRECTORY_TYPE = 0x0002;
        /// File system needs to replay its journal
        const REQ_REPLAY_JOURNAL = 0x0004;
        /// File system uses a journal device
        const REQ_JOURNAL_DEVICE = 0x0008;
    }
}

bitflags! {
    /// ROnly features. If these are not supported; remount as read-only
    pub struct FeaturesROnly: u32 {
        /// Sparse superblocks and group descriptor tables
        const RONLY_SPARSE = 0x0001;
        /// File system uses a 64-bit file size
        const RONLY_FILE_SIZE_64 = 0x0002;
        /// Directory contents are stored in the form of a Binary Tree
        const RONLY_BTREE_DIRECTORY = 0x0004;
    }
}
