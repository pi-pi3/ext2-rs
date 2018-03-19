#[cfg(any(test, not(feature = "no_std")))]
use std::io;

/// The set of all possible errors
#[derive(Debug)]
pub enum Error {
    BadMagic(u16),
    OutOfBounds(usize),
    BadBlockGroupCount(u32, u32),
    #[cfg(any(test, not(feature = "no_std")))]
    Io(io::Error),
}

impl From<Infallible> for Error {
    fn from(_: Infallible) -> Error {
        unreachable!()
    }
}

#[cfg(any(test, not(feature = "no_std")))]
impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

pub enum Infallible {}
