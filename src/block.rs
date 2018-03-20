use core::mem;
use core::marker::PhantomData;
use core::ops::{Add, Sub};
use core::fmt::{self, Debug, Display, LowerHex};
use core::iter::Step;

pub trait Size {
    // log_block_size = log_2(block_size) - 10
    // i.e. block_size = 1024 << log_block_size
    const LOG_SIZE: u32;
    const SIZE: usize = 1024 << Self::LOG_SIZE;
    const OFFSET_MASK: usize = Self::SIZE - 1;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Size1024;
impl Size for Size1024 {
    const LOG_SIZE: u32 = 0;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Size2048;
impl Size for Size2048 {
    const LOG_SIZE: u32 = 1;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Size4096;
impl Size for Size4096 {
    const LOG_SIZE: u32 = 2;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Size8192;
impl Size for Size8192 {
    const LOG_SIZE: u32 = 3;
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
        let block = (block as isize + (offset >> (S::LOG_SIZE + 10))) as usize;
        let offset = offset.abs() as usize & S::OFFSET_MASK;
        unsafe { Address::new_unchecked(block, offset) }
    }

    pub fn into_index(&self) -> Option<usize> {
        self.block
            .checked_shl(S::LOG_SIZE + 10)
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

impl<S: Size> From<usize> for Address<S> {
    fn from(idx: usize) -> Address<S> {
        let block = idx >> (S::LOG_SIZE + 10);
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
            Address::<Size1024>::new(0, 1024),
            Address::<Size1024>::new(1, 0),
        );

        assert_eq!(
            Address::<Size1024>::new(2, -512),
            Address::<Size1024>::new(1, 512),
        );

        let a = Address::<Size2048>::new(0, 1024);
        let b = Address::<Size2048>::new(0, 1024);
        assert_eq!(a + b, Address::<Size2048>::new(1, 0));
        assert_eq!((a + b).into_index(), Some(2048));

        let a = Address::<Size1024>::new(0, 4096);
        let b = Address::<Size1024>::new(0, 512);
        assert_eq!(a - b, Address::<Size1024>::new(3, 512));
        assert_eq!((a - b).into_index(), Some(3584));
    }
}
