use core::mem;
use core::marker::PhantomData;
use core::ops::{Add, Sub};
use core::fmt::{self, Debug, Display, LowerHex};
use core::iter::Step;

pub trait Size: PartialOrd {
    // log_block_size = log_2(block_size)
    const LOG_SIZE: u32;
    const SIZE: usize = 1 << Self::LOG_SIZE;
    const OFFSET_MASK: usize = Self::SIZE - 1;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Size512;
impl Size for Size512 {
    const LOG_SIZE: u32 = 9;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Size2048;
impl Size for Size2048 {
    const LOG_SIZE: u32 = 11;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Size4096;
impl Size for Size4096 {
    const LOG_SIZE: u32 = 12;
}

/// Address in a physical sector
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Address<S: Size> {
    block: usize,
    offset: usize,
    _phantom: PhantomData<S>,
}

impl<S: Size> Address<S> {
    pub unsafe fn new_unchecked(block: usize, offset: usize) -> Address<S> {
        assert!(offset < S::SIZE, "offset out of block bounds");
        let _phantom = PhantomData;
        Address {
            block,
            offset,
            _phantom,
        }
    }

    pub fn new(block: usize, offset: isize) -> Address<S> {
        let block = (block as isize + (offset >> S::LOG_SIZE)) as usize;
        let offset = offset.abs() as usize & S::OFFSET_MASK;
        unsafe { Address::new_unchecked(block, offset) }
    }

    pub fn with_block_size(
        block: usize,
        offset: usize,
        log_block_size: u32,
    ) -> Address<S> {
        let log_diff = log_block_size as isize - S::LOG_SIZE as isize;
        let top_offset = offset >> S::LOG_SIZE;
        let offset = offset >> log_diff;
        let block = block << log_diff | top_offset;
        Address::new(block, offset as isize)
    }

    pub fn index64(&self) -> u64 {
        ((self.block as u64) << S::LOG_SIZE) + self.offset as u64
    }

    pub fn into_index(&self) -> Option<usize> {
        self.block
            .checked_shl(S::LOG_SIZE)
            .and_then(|block| block.checked_add(self.offset))
    }

    pub const fn block_size(&self) -> usize {
        S::SIZE
    }

    pub const fn log_block_size(&self) -> u32 {
        S::LOG_SIZE
    }

    pub fn block(&self) -> usize {
        self.block
    }

    pub fn offset(&self) -> usize {
        self.offset
    }
}

impl<S: Size + Clone + PartialOrd> Step for Address<S> {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        if end.block >= start.block {
            Some(end.block - start.block)
        } else {
            None
        }
    }

    fn replace_one(&mut self) -> Self {
        mem::replace(self, Address::new(1, 0))
    }

    fn replace_zero(&mut self) -> Self {
        mem::replace(self, Address::new(0, 0))
    }

    fn add_one(&self) -> Self {
        Address::new(self.block + 1, 0)
    }

    fn sub_one(&self) -> Self {
        Address::new(self.block - 1, 0)
    }

    fn add_usize(&self, n: usize) -> Option<Self> {
        self.block
            .checked_add(n)
            .map(|block| Address::new(block, 0))
    }
}

impl<S: Size> Debug for Address<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = format!("Address<{}>", S::SIZE);
        f.debug_struct(&name)
            .field("block", &self.block)
            .field("offset", &self.offset)
            .finish()
    }
}

impl<S: Size> Display for Address<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.block, self.offset)
    }
}

impl<S: Size> LowerHex for Address<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:x}:{:x}", self.block, self.offset)
    }
}

impl<S: Size> From<u64> for Address<S> {
    fn from(idx: u64) -> Address<S> {
        let block = idx >> S::LOG_SIZE;
        let offset = idx & S::OFFSET_MASK as u64;
        Address::new(block as usize, offset as isize)
    }
}

impl<S: Size> From<usize> for Address<S> {
    fn from(idx: usize) -> Address<S> {
        let block = idx >> S::LOG_SIZE;
        let offset = idx & S::OFFSET_MASK;
        Address::new(block, offset as isize)
    }
}

impl<S: Size> Add for Address<S> {
    type Output = Address<S>;
    fn add(self, rhs: Address<S>) -> Address<S> {
        Address::new(
            self.block + rhs.block,
            (self.offset + rhs.offset) as isize,
        )
    }
}

impl<S: Size> Sub for Address<S> {
    type Output = Address<S>;
    fn sub(self, rhs: Address<S>) -> Address<S> {
        Address::new(
            self.block + rhs.block,
            self.offset as isize - rhs.offset as isize,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic() {
        assert_eq!(
            Address::<Size512>::new(0, 512),
            Address::<Size512>::new(1, 0),
        );

        assert_eq!(
            Address::<Size512>::new(2, -256),
            Address::<Size512>::new(1, 256),
        );

        let a = Address::<Size2048>::new(0, 1024);
        let b = Address::<Size2048>::new(0, 1024);
        assert_eq!(a + b, Address::<Size2048>::new(1, 0));
        assert_eq!((a + b).into_index(), Some(2048));

        let a = Address::<Size512>::new(0, 2048);
        let b = Address::<Size512>::new(0, 256);
        assert_eq!(a - b, Address::<Size512>::new(3, 256));
        assert_eq!((a - b).into_index(), Some(1792));
    }
}
