use core::marker::PhantomData;
use core::ops::{Add, Sub};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Address<S: Size> {
    block: usize,
    offset: usize,
    _phantom: PhantomData<S>,
}

impl<S: Size> Address<S> {
    pub fn new(block: usize, offset: usize) -> Address<S> {
        let block = block + offset >> (S::LOG_SIZE + 10);
        let offset = offset & S::OFFSET_MASK;
        let _phantom = PhantomData;
        Address {
            block,
            offset,
            _phantom,
        }
    }

    pub fn into_index(&self) -> Option<usize> {
        self.block
            .checked_shl(S::LOG_SIZE + 10)
            .and_then(|block| block.checked_add(self.offset))
    }

    pub fn block(&self) -> usize {
        self.block
    }

    pub fn offset(&self) -> usize {
        self.offset
    }
}

impl<S: Size> From<usize> for Address<S> {
    fn from(idx: usize) -> Address<S> {
        let block = idx >> (S::LOG_SIZE + 10);
        let offset = idx & S::OFFSET_MASK;
        Address::new(block, offset)
    }
}

impl<S: Size> Add for Address<S> {
    type Output = Address<S>;
    fn add(self, rhs: Address<S>) -> Address<S> {
        let offset = self.offset + rhs.offset;
        let block = offset >> (S::LOG_SIZE + 10);
        let offset = offset & S::OFFSET_MASK;
        Address::new(self.block + rhs.block + block, offset)
    }
}

impl<S: Size> Sub for Address<S> {
    type Output = Address<S>;
    fn sub(mut self, rhs: Address<S>) -> Address<S> {
        if rhs.offset > self.offset {
            self.offset += S::SIZE;
            self.block -= 1;
        }
        let offset = self.offset - rhs.offset;
        Address::new(self.block - rhs.block, offset)
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

        let a = Address::<Size2048>::new(0, 1024);
        let b = Address::<Size2048>::new(0, 1024);
        assert_eq!(a + b, Address::<Size2048>::new(1, 0));
        assert_eq!((a + b).into_index(), Some(2048));

        let a = Address::<Size1024>::new(0, 4096);
        let b = Address::<Size1024>::new(0, 512);
        assert_eq!(a - b, Address::<Size1024>::new(3, 512));
        assert_eq!((a + b).into_index(), Some(3584));
    }
}
