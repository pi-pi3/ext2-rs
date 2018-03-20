use core::mem;
use core::slice;
use core::ops::{Deref, DerefMut, Range};

use alloc::Vec;
use alloc::boxed::Box;
use alloc::borrow::{Cow, ToOwned};

use error::Infallible;
use block::{Address, Size};

pub mod length;
use self::length::Length;

pub trait Buffer<T, Idx>
where
    [T]: ToOwned,
    Idx: PartialEq + PartialOrd,
{
    type Error;

    fn len(&self) -> Length<Idx>;
    fn commit(
        &mut self,
        slice: Option<BufferCommit<T, Idx>>,
    ) -> Result<(), Self::Error>;
    unsafe fn slice_unchecked<'a>(
        &'a self,
        range: Range<Idx>,
    ) -> BufferSlice<'a, T, Idx>;

    fn slice<'a>(
        &'a self,
        range: Range<Idx>,
    ) -> Option<BufferSlice<'a, T, Idx>> {
        if self.len() >= range.end && self.len() > range.start {
            unsafe { Some(self.slice_unchecked(range)) }
        } else {
            None
        }
    }
}

pub struct BufferSlice<'a, T: 'a, Idx>
where
    [T]: ToOwned,
{
    inner: Cow<'a, [T]>,
    index: Idx,
}

impl<T, Idx: Default> BufferSlice<'static, T, Idx>
where
    [T]: ToOwned,
{
    pub fn with_static(inner: &'static [T]) -> BufferSlice<'static, T, Idx> {
        BufferSlice {
            inner: Cow::Borrowed(inner),
            index: Idx::default(),
        }
    }
}

impl<T, Idx> BufferSlice<'static, T, Idx>
where
    [T]: ToOwned,
{
    pub fn new_owned(
        inner: <[T] as ToOwned>::Owned,
        index: Idx,
    ) -> BufferSlice<'static, T, Idx> {
        BufferSlice {
            inner: Cow::Owned(inner),
            index,
        }
    }
}

impl<'a, T, Idx> BufferSlice<'a, T, Idx>
where
    [T]: ToOwned,
{
    pub fn new(inner: &'a [T], index: Idx) -> BufferSlice<'a, T, Idx> {
        BufferSlice {
            inner: Cow::Borrowed(inner),
            index,
        }
    }

    pub fn is_mutated(&self) -> bool {
        match self.inner {
            Cow::Borrowed(_) => false,
            Cow::Owned(_) => true,
        }
    }

    pub fn at_index(&self) -> &Idx {
        &self.index
    }
}

impl<'a, Idx: Copy> BufferSlice<'a, u8, Idx> {
    pub unsafe fn dynamic_cast<T: Copy>(&self) -> (T, Idx) {
        assert!(self.inner.len() >= mem::size_of::<T>());
        let index = self.index;
        let cast = mem::transmute_copy(self.inner.as_ptr().as_ref().unwrap());
        (cast, index)
    }

    pub fn from_cast<T: Copy>(
        cast: &'a T,
        index: Idx,
    ) -> BufferSlice<'a, u8, Idx> {
        let len = mem::size_of::<T>();
        let ptr = cast as *const T as *const u8;
        let slice = unsafe { slice::from_raw_parts(ptr, len) };
        BufferSlice::new(slice, index)
    }
}

impl<'a, T, Idx> BufferSlice<'a, T, Idx>
where
    [T]: ToOwned<Owned = Vec<T>>,
{
    pub fn commit(self) -> Option<BufferCommit<T, Idx>> {
        if self.is_mutated() {
            Some(BufferCommit::new(self.inner.into_owned(), self.index))
        } else {
            None
        }
    }
}

impl<'a, T, Idx> AsRef<[T]> for BufferSlice<'a, T, Idx>
where
    [T]: ToOwned,
{
    fn as_ref(&self) -> &[T] {
        self.inner.as_ref()
    }
}

impl<'a, T, Idx> AsMut<[T]> for BufferSlice<'a, T, Idx>
where
    [T]: ToOwned,
    <[T] as ToOwned>::Owned: AsMut<[T]>,
{
    fn as_mut(&mut self) -> &mut [T] {
        self.inner.to_mut().as_mut()
    }
}

impl<'a, T, Idx> Deref for BufferSlice<'a, T, Idx>
where
    [T]: ToOwned,
{
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<'a, T, Idx> DerefMut for BufferSlice<'a, T, Idx>
where
    [T]: ToOwned,
    <[T] as ToOwned>::Owned: AsMut<[T]>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

pub struct BufferCommit<T, Idx> {
    inner: Vec<T>,
    index: Idx,
}

impl<T, Idx: Default> BufferCommit<T, Idx> {
    pub fn with_vec(inner: Vec<T>) -> BufferCommit<T, Idx> {
        BufferCommit {
            inner,
            index: Idx::default(),
        }
    }
}

impl<T, Idx> BufferCommit<T, Idx> {
    pub fn new(inner: Vec<T>, index: Idx) -> BufferCommit<T, Idx> {
        BufferCommit { inner, index }
    }

    pub fn into_inner(self) -> Vec<T> {
        self.inner
    }

    pub fn at_index(&self) -> &Idx {
        &self.index
    }
}

impl<T, Idx> AsRef<[T]> for BufferCommit<T, Idx> {
    fn as_ref(&self) -> &[T] {
        self.inner.as_ref()
    }
}

impl<T, Idx> AsMut<[T]> for BufferCommit<T, Idx> {
    fn as_mut(&mut self) -> &mut [T] {
        self.inner.as_mut()
    }
}

impl<T, Idx> Deref for BufferCommit<T, Idx> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T, Idx> DerefMut for BufferCommit<T, Idx> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

macro_rules! impl_slice {
    (@inner $buffer:ty $( , $lt:lifetime )* ) => {
        impl<$( $lt, )* S: Size + PartialOrd + Copy, T> Buffer<T, Address<S>> for $buffer
        where
            T: Clone,
            [T]: ToOwned,
        {
            type Error = Infallible;

            fn len(&self) -> Length<Address<S>> {
                Length::Bounded(Address::from(<Self as AsRef<[T]>>::as_ref(self).len()))
            }

            fn commit(&mut self, slice: Option<BufferCommit<T, Address<S>>>) -> Result<(), Infallible> {
                slice.map(|slice| {
                    let index = slice.at_index().index64() as usize;
                    let end = index + slice.as_ref().len();
                    // XXX: it would be much better to drop the contents of dst
                    // and move the contents of slice instead of cloning
                    let dst =
                        &mut <Self as AsMut<[T]>>::as_mut(self)[index..end];
                    dst.clone_from_slice(slice.as_ref());
                });
                Ok(())
            }

            unsafe fn slice_unchecked<'a>(
                &'a self,
                range: Range<Address<S>>,
            ) -> BufferSlice<'a, T, Address<S>> {
                let index = range.start;
                let range = range.start.index64() as usize..range.end.index64() as usize;
                BufferSlice::new(
                    <Self as AsRef<[T]>>::as_ref(self).get_unchecked(range),
                    index,
                )
            }
        }
    };
    ($buffer:ty) => {
        impl_slice!(@inner $buffer);
    };
    ($buffer:ty $( , $lt:lifetime )* ) => {
        impl_slice!(@inner $buffer $( , $lt )* );
    };
}

impl_slice!(&'b mut [T], 'b);
impl_slice!(Vec<T>);
impl_slice!(Box<[T]>);

#[cfg(any(test, not(feature = "no_std")))]
mod file {
    use std::ops::Range;
    use std::io::{self, Read, Seek, SeekFrom, Write};
    use std::fs::File;
    use std::cell::RefCell;

    use block::{Address, Size};

    use super::{Buffer, BufferCommit, BufferSlice};
    use super::length::Length;

    impl<S: Size + PartialOrd + Copy> Buffer<u8, Address<S>> for RefCell<File> {
        type Error = io::Error;

        fn len(&self) -> Length<Address<S>> {
            Length::Bounded(
                self.borrow()
                    .metadata()
                    .map(|data| Address::from(data.len()))
                    .unwrap_or(Address::from(0_usize)),
            )
        }

        fn commit(
            &mut self,
            slice: Option<BufferCommit<u8, Address<S>>>,
        ) -> Result<(), Self::Error> {
            slice
                .map(|slice| {
                    let index = *slice.at_index();
                    let end = index + Address::from(slice.as_ref().len());
                    let mut refmut = self.borrow_mut();
                    refmut
                        .seek(SeekFrom::Start(index.index64()))
                        .and_then(|_| refmut.write(slice.as_ref()))
                        .map(|_| ())
                })
                .unwrap_or(Ok(()))
        }

        unsafe fn slice_unchecked<'a>(
            &'a self,
            range: Range<Address<S>>,
        ) -> BufferSlice<'a, u8, Address<S>> {
            let index = range.start;
            let len = range.end - range.start;
            let mut vec = Vec::with_capacity(len.index64() as usize);
            vec.set_len(len.index64() as usize);
            let mut refmut = self.borrow_mut();
            refmut
                .seek(SeekFrom::Start(index.index64()))
                .and_then(|_| refmut.read_exact(&mut vec[..]))
                .unwrap_or_else(|err| {
                    panic!("could't read from File Buffer: {:?}", err)
                });
            BufferSlice::new_owned(vec, index)
        }

        fn slice<'a>(
            &'a self,
            range: Range<Address<S>>,
        ) -> Option<BufferSlice<'a, u8, Address<S>>> {
            let index = range.start;
            let mut vec = Vec::with_capacity(
                (range.end - range.start).index64() as usize,
            );
            let mut refmut = self.borrow_mut();
            refmut
                .seek(SeekFrom::Start(index.index64()))
                .and_then(|_| refmut.read_exact(&mut vec[..]))
                .map(move |_| BufferSlice::new_owned(vec, index))
                .ok()
        }
    }
}

#[cfg(test)]
mod tests {
    use block::{Address, Size512};
    use super::*;

    #[test]
    fn buffer() {
        let mut buffer = vec![0; 1024];
        let commit = {
            let mut slice = buffer
                .slice(
                    Address::<Size512>::from(256_usize)
                        ..Address::<Size512>::from(512_usize),
                )
                .unwrap();
            slice.iter_mut().for_each(|x| *x = 1);
            slice.commit()
        };
        assert!(buffer.commit(commit).is_ok());

        for (i, &x) in buffer.iter().enumerate() {
            if i < 256 || i >= 512 {
                assert_eq!(x, 0);
            } else {
                assert_eq!(x, 1);
            }
        }
    }
}
