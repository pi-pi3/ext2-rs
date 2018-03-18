#![cfg_attr(not(test), no_std)]

#[macro_use]
extern crate bitflags;

pub mod superblock;
pub mod block_group;
pub mod inode;

#[cfg(test)]
mod tests {
    use super::superblock::*;
    use super::block_group::*;
    use super::inode::*;

    #[test]
    fn sizes() {
        use std::mem::size_of;
        assert_eq!(size_of::<Superblock>(), 1024);
        assert_eq!(size_of::<BlockGroupDescriptor>(), 32);
        assert_eq!(size_of::<Inode>(), 128);
    }
}
