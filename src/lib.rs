#![feature(alloc)]
#![feature(specialization)]
#![feature(swap_with_slice)]
#![feature(macro_lifetime_matcher)]
#![cfg_attr(all(not(test), feature = "no_std"), no_std)]

extern crate alloc;
#[macro_use]
extern crate bitflags;
#[cfg(any(test, not(feature = "no_std")))]
extern crate core;

pub mod error;
pub mod sys;
pub mod buffer;
pub mod fs;

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
