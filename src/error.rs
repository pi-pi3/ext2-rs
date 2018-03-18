/// The set of all possible errors
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Error {
    BadMagic(u16),
    OutOfBounds(usize),
    BadBlockGroupCount(u32, u32),
}
