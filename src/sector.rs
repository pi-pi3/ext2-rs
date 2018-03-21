use core::mem;
use core::marker::PhantomData;
use core::ops::{Add, Sub};
use core::fmt::{self, Debug, Display, LowerHex};
use core::iter::Step;

pub trait Size: Clone + Copy + PartialOrd {
    // log_sector_size = log_2(sector_size)
    const LOG_SIZE: u32;
    const SIZE: usize = 1 << Self::LOG_SIZE;
    const OFFSET_MASK: u32 = (Self::SIZE - 1) as u32;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Size512;
impl Size for Size512 {
    const LOG_SIZE: u32 = 9;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Size1024;
impl Size for Size1024 {
    const LOG_SIZE: u32 = 10;
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
    sector: u32,
    offset: u32,
    _phantom: PhantomData<S>,
}

impl<S: Size> Address<S> {
    pub unsafe fn new_unchecked(sector: u32, offset: u32) -> Address<S> {
        assert!((offset as usize) < S::SIZE, "offset out of sector bounds");
        let _phantom = PhantomData;
        Address {
            sector,
            offset,
            _phantom,
        }
    }

    pub fn new(sector: u32, offset: i32) -> Address<S> {
        let sector = (sector as i32 + (offset >> S::LOG_SIZE)) as u32;
        let offset = offset.abs() as u32 & S::OFFSET_MASK;
        unsafe { Address::new_unchecked(sector, offset) }
    }

    pub fn with_block_size(
        block: u32,
        offset: i32,
        log_block_size: u32,
    ) -> Address<S> {
        let block = (block as i32 + (offset >> log_block_size)) as u32;
        let offset = offset.abs() as u32 & ((1 << log_block_size) - 1);

        let log_diff = log_block_size as i32 - S::LOG_SIZE as i32;
        let top_offset = offset >> S::LOG_SIZE;
        let offset = offset & ((1 << S::LOG_SIZE) - 1);
        let sector = block << log_diff | top_offset;
        unsafe { Address::new_unchecked(sector, offset) }
    }

    pub fn into_index(&self) -> u64 {
        ((self.sector as u64) << S::LOG_SIZE) + self.offset as u64
    }

    pub const fn sector_size(&self) -> usize {
        S::SIZE
    }

    pub const fn log_sector_size(&self) -> u32 {
        S::LOG_SIZE
    }

    pub fn sector(&self) -> u32 {
        self.sector
    }

    pub fn offset(&self) -> u32 {
        self.offset
    }
}

impl<S: Size> Step for Address<S> {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        if end.sector >= start.sector {
            Some(end.sector as usize - start.sector as usize)
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
        Address::new(self.sector + 1, 0)
    }

    fn sub_one(&self) -> Self {
        Address::new(self.sector - 1, 0)
    }

    fn add_usize(&self, n: usize) -> Option<Self> {
        self.sector
            .checked_add(n as u32)
            .map(|sector| Address::new(sector, 0))
    }
}

impl<S: Size> Debug for Address<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = format!("Address<{}>", S::SIZE);
        f.debug_struct(&name)
            .field("sector", &self.sector)
            .field("offset", &self.offset)
            .finish()
    }
}

impl<S: Size> Display for Address<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.sector, self.offset)
    }
}

impl<S: Size> LowerHex for Address<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:x}:{:x}", self.sector, self.offset)
    }
}

impl<S: Size> From<u64> for Address<S> {
    fn from(idx: u64) -> Address<S> {
        let sector = idx >> S::LOG_SIZE;
        let offset = idx & S::OFFSET_MASK as u64;
        Address::new(sector as u32, offset as i32)
    }
}

impl<S: Size> From<usize> for Address<S> {
    fn from(idx: usize) -> Address<S> {
        let sector = idx >> S::LOG_SIZE;
        let offset = idx & S::OFFSET_MASK as usize;
        Address::new(sector as u32, offset as i32)
    }
}

impl<S: Size> Add for Address<S> {
    type Output = Address<S>;
    fn add(self, rhs: Address<S>) -> Address<S> {
        Address::new(
            self.sector + rhs.sector,
            (self.offset + rhs.offset) as i32,
        )
    }
}

impl<S: Size> Sub for Address<S> {
    type Output = Address<S>;
    fn sub(self, rhs: Address<S>) -> Address<S> {
        Address::new(
            self.sector - rhs.sector,
            self.offset as i32 - rhs.offset as i32,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conv() {
        assert_eq!(Address::<Size512>::new(0, 1024).into_index(), 1024);
        assert_eq!(Address::<Size512>::from(1024_u64).into_index(), 1024);
        assert_eq!(
            Address::<Size512>::with_block_size(1, 256, 10).into_index(),
            1024 + 256
        );
        assert_eq!(
            Address::<Size512>::with_block_size(2, 0, 10).into_index(),
            2048
        );
        assert_eq!(
            Address::<Size512>::with_block_size(0, 1792, 10).into_index(),
            1792
        );
    }

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
        assert_eq!((a + b).into_index(), 2048);

        let a = Address::<Size512>::new(0, 2048);
        let b = Address::<Size512>::new(0, 256);
        assert_eq!(a - b, Address::<Size512>::new(3, 256));
        assert_eq!((a - b).into_index(), 1792);
    }
}
