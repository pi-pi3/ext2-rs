use core::fmt::{self, Display};
use alloc::{String, Vec};

#[cfg(any(test, not(feature = "no_std")))]
use std::io;

/// The set of all possible errors
#[derive(Debug)]
pub enum Error {
    Other(String),
    BadMagic {
        magic: u16,
    },
    OutOfBounds {
        index: usize,
    },
    AddressOutOfBounds {
        sector: u32,
        offset: u32,
        size: usize,
    },
    BadBlockGroupCount {
        by_blocks: u32,
        by_inodes: u32,
    },
    InodeNotFound {
        inode: u32,
    },
    NotADirectory {
        inode: u32,
        name: String,
    },
    NotAbsolute {
        name: String,
    },
    NotFound {
        name: String,
    },
    #[cfg(any(test, not(feature = "no_std")))]
    Io {
        inner: io::Error,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Other(ref msg) => write!(f, "{}", msg),
            Error::BadMagic {
                magic,
            } => write!(f, "invalid magic value: {}", magic),
            Error::OutOfBounds {
                index,
            } => write!(f, "index ouf of bounds: {}", index),
            Error::AddressOutOfBounds {
                sector,
                offset,
                size,
            } => write!(f, "address ouf of bounds: {}:{} with a block size of: {}",
                   sector, offset, size),
            Error::BadBlockGroupCount {
                by_blocks,
                by_inodes,
            } => write!(f, "conflicting block group count data; by blocks: {}, by inodes: {}", by_blocks, by_inodes),
            Error::InodeNotFound {
                inode,
            } => write!(f, "couldn't find inode no. {}", &inode),
            Error::NotADirectory {
                inode,
                ref name,
            } => write!(f, "inode no. {} at: {} is not a directory", inode, &name),
            Error::NotAbsolute {
                ref name,
            } => write!(f, "{} is not an absolute path", &name),
            Error::NotFound {
                ref name,
            } => write!(f, "couldn't find {}", &name),
            #[cfg(any(test, not(feature = "no_std")))]
            Error::Io {
                ref inner,
            } => write!(f, "io error: {}", inner),
        }
    }
}

impl From<Infallible> for Error {
    fn from(_: Infallible) -> Error {
        unreachable!()
    }
}

#[cfg(any(test, not(feature = "no_std")))]
impl From<io::Error> for Error {
    fn from(inner: io::Error) -> Error {
        Error::Io { inner }
    }
}

pub enum Infallible {}
