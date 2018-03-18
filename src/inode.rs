#[repr(C, packed)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Inode {
    /// Type and Permissions (see below)
    type_perm: u16,
    /// User ID
    uid: u16,
    /// Lower 32 bits of size in bytes
    size_low: u32,
    /// Last Access Time (in POSIX time)
    atime: u32,
    /// Creation Time (in POSIX time)
    ctime: u32,
    /// Last Modification time (in POSIX time)
    mtime: u32,
    /// Deletion time (in POSIX time)
    dtime: u32,
    /// Group ID
    gid: u16,
    /// Count of hard links (directory entries) to this inode. When this
    /// reaches 0, the data blocks are marked as unallocated.
    hard_links: u16,
    /// Count of disk sectors (not Ext2 blocks) in use by this inode, not
    /// counting the actual inode structure nor directory entries linking
    /// to the inode.
    sectors_count: u32,
    /// Flags
    flags: u32,
    /// Operating System Specific value #1
    _os_specific_1: u32,
    /// Direct block pointers
    direct_pointer: [u32; 12],
    /// Singly Indirect Block Pointer (Points to a block that is a list of
    /// block pointers to data)
    indirect_pointer: u32,
    /// Doubly Indirect Block Pointer (Points to a block that is a list of
    /// block pointers to Singly Indirect Blocks)
    doubly_indirect: u32,
    /// Triply Indirect Block Pointer (Points to a block that is a list of
    /// block pointers to Doubly Indirect Blocks)
    triply_indirect: u32,
    /// Generation number (Primarily used for NFS)
    gen_number: u32,
    /// In Ext2 version 0, this field is reserved. In version >= 1,
    /// Extended attribute block (File ACL).
    ext_attribute_block: u32,
    /// In Ext2 version 0, this field is reserved. In version >= 1, Upper
    /// 32 bits of file size (if feature bit set) if it's a file,
    /// Directory ACL if it's a directory
    size_high: u32,
    /// Block address of fragment
    frag_block_addr: u32,
    /// Operating System Specific Value #2
    _os_specific_2: [u8; 12],
}

bitflags! {
    pub struct TypePerm: u16 {
        /// FIFO
        const FIFO = 0x1000;
        /// Character device
        const CHAR_DEVICE = 0x2000;
        /// Directory
        const DIRECTORY = 0x4000;
        /// Block device
        const BLOCK_DEVICE = 0x6000;
        /// Regular file
        const FILE = 0x8000;
        /// Symbolic link
        const SYMLINK = 0xA000;
        /// Unix socket
        const SOCKET = 0xC000;
        /// Other—execute permission
        const O_EXEC = 0x001;
        /// Other—write permission
        const O_WRITE = 0x002;
        /// Other—read permission
        const O_READ = 0x004;
        /// Group—execute permission
        const G_EXEC = 0x008;
        /// Group—write permission
        const G_WRITE = 0x010;
        /// Group—read permission
        const G_READ = 0x020;
        /// User—execute permission
        const U_EXEC = 0x040;
        /// User—write permission
        const U_WRITE = 0x080;
        /// User—read permission
        const U_READ = 0x100;
        /// Sticky Bit
        const STICKY = 0x200;
        /// Set group ID
        const SET_GID = 0x400;
        /// Set user ID
        const SET_UID = 0x800;
    }
}

bitflags! {
    pub struct InodeFlags: u32 {
        /// Secure deletion (not used)
        const SECURE_DEL = 0x00000001;
        /// Keep a copy of data when deleted (not used)
        const KEEP_COPY = 0x00000002;
        /// File compression (not used)
        const COMPRESSION = 0x00000004;
        /// Synchronous updates—new data is written immediately to disk
        const SYNC_UPDATE = 0x00000008;
        /// Immutable file (content cannot be changed)
        const IMMUTABLE = 0x00000010;
        /// Append only
        const APPEND_ONLY = 0x00000020;
        /// File is not included in 'dump' command
        const NODUMP = 0x00000040;
        /// Last accessed time should not updated
        const DONT_ATIME = 0x00000080;
        /// Hash indexed directory
        const HASH_DIR = 0x00010000;
        /// AFS directory
        const AFS_DIR = 0x00020000;
        /// Journal file data
        const JOURNAL_DATA = 0x00040000;
    }
}
