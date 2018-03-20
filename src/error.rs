#[cfg(any(test, not(feature = "no_std")))]
use std::io;

/// The set of all possible errors
#[derive(Debug)]
pub enum Error {
    BadMagic(u16),
    OutOfBounds(usize),
    AddressOutOfBounds(usize, usize, usize),
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

impl PartialEq for Error {
    fn eq(&self, rhs: &Error) -> bool {
        match (self, rhs) {
            (&Error::BadMagic(a), &Error::BadMagic(b)) => a == b,
            (&Error::OutOfBounds(a), &Error::OutOfBounds(b)) => a == b,
            (
                &Error::BadBlockGroupCount(a1, a2),
                &Error::BadBlockGroupCount(b1, b2),
            ) => a1 == b1 && a2 == b2,
            _ => false,
        }
    }

    fn ne(&self, rhs: &Error) -> bool {
        match (self, rhs) {
            (&Error::BadMagic(a), &Error::BadMagic(b)) => a != b,
            (&Error::OutOfBounds(a), &Error::OutOfBounds(b)) => a != b,
            (
                &Error::BadBlockGroupCount(a1, a2),
                &Error::BadBlockGroupCount(b1, b2),
            ) => a1 != b1 || a2 != b2,
            _ => false,
        }
    }
}

pub enum Infallible {}
