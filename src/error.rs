/// Wrapper around the raw `ErrorKind`
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Error {
    kind: ErrorKind,
}

impl Error {
    pub fn from_kind(kind: ErrorKind) -> Error {
        Error { kind }
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

/// The set of all possible errors
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    BadMagic,
    OutOfBounds,
}
