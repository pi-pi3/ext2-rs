#![cfg_attr(not(test), no_std)]

#[macro_use]
extern crate bitflags;

pub mod error;
pub mod sys;

#[cfg(test)]
mod tests {
    use sys::superblock::*;
    use sys::block_group::*;
    use sys::inode::*;

    #[test]
    fn sizes() {
        use std::mem::size_of;
        assert_eq!(size_of::<Superblock>(), 1024);
        assert_eq!(size_of::<BlockGroupDescriptor>(), 32);
        assert_eq!(size_of::<Inode>(), 128);
    }
}
