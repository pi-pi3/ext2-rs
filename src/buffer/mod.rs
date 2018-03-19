use core::mem;
use core::slice;
use core::ops::{Deref, DerefMut, Range};

use alloc::Vec;
use alloc::boxed::Box;
use alloc::borrow::{Cow, ToOwned};

use error::Infallible;

pub mod length;
use self::length::Length;

pub trait Buffer<T>
where
    [T]: ToOwned,
{
    type Error;

    fn len(&self) -> Length;
    fn commit(
        &mut self,
        slice: Option<BufferCommit<T>>,
    ) -> Result<(), Self::Error>;
    unsafe fn slice_unchecked<'a>(
        &'a self,
        range: Range<usize>,
    ) -> BufferSlice<'a, T>;

    fn slice<'a>(&'a self, range: Range<usize>) -> Option<BufferSlice<'a, T>> {
        if self.len() >= range.end && self.len() > range.start {
            unsafe { Some(self.slice_unchecked(range)) }
        } else {
            None
        }
    }
}

pub struct BufferSlice<'a, T: 'a>
where
    [T]: ToOwned,
{
    inner: Cow<'a, [T]>,
    index: usize,
}

impl<T> BufferSlice<'static, T>
where
    [T]: ToOwned,
{
    pub fn with_static(inner: &'static [T]) -> BufferSlice<'static, T> {
        BufferSlice {
            inner: Cow::Borrowed(inner),
            index: 0,
        }
    }

    pub fn new_owned(
        inner: <[T] as ToOwned>::Owned,
        index: usize,
    ) -> BufferSlice<'static, T> {
        BufferSlice {
            inner: Cow::Owned(inner),
            index,
        }
    }
}

impl<'a, T> BufferSlice<'a, T>
where
    [T]: ToOwned,
{
    pub fn new(inner: &'a [T], index: usize) -> BufferSlice<'a, T> {
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

    #[inline]
    pub fn at_index(&self) -> usize {
        self.index
    }
}

impl<'a> BufferSlice<'a, u8> {
    pub unsafe fn dynamic_cast<T: Copy>(&self) -> (T, usize) {
        assert!(self.inner.len() >= mem::size_of::<T>());
        let index = self.index;
        let cast = mem::transmute_copy(self.inner.as_ptr().as_ref().unwrap());
        (cast, index)
    }

    pub fn from_cast<T: Copy>(
        cast: &'a T,
        index: usize,
    ) -> BufferSlice<'a, u8> {
        let len = mem::size_of::<T>();
        let ptr = cast as *const T as *const u8;
        let slice = unsafe { slice::from_raw_parts(ptr, len) };
        BufferSlice::new(slice, index)
    }
}

impl<'a, T> BufferSlice<'a, T>
where
    [T]: ToOwned<Owned = Vec<T>>,
{
    pub fn commit(self) -> Option<BufferCommit<T>> {
        if self.is_mutated() {
            Some(BufferCommit::new(self.inner.into_owned(), self.index))
        } else {
            None
        }
    }
}

impl<'a, T> AsRef<[T]> for BufferSlice<'a, T>
where
    [T]: ToOwned,
{
    fn as_ref(&self) -> &[T] {
        self.inner.as_ref()
    }
}

impl<'a, T> AsMut<[T]> for BufferSlice<'a, T>
where
    [T]: ToOwned,
    <[T] as ToOwned>::Owned: AsMut<[T]>,
{
    fn as_mut(&mut self) -> &mut [T] {
        self.inner.to_mut().as_mut()
    }
}

impl<'a, T> Deref for BufferSlice<'a, T>
where
    [T]: ToOwned,
{
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<'a, T> DerefMut for BufferSlice<'a, T>
where
    [T]: ToOwned,
    <[T] as ToOwned>::Owned: AsMut<[T]>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

pub struct BufferCommit<T> {
    inner: Vec<T>,
    index: usize,
}

impl<T> BufferCommit<T> {
    pub fn with_vec(inner: Vec<T>) -> BufferCommit<T> {
        BufferCommit { inner, index: 0 }
    }

    pub fn new(inner: Vec<T>, index: usize) -> BufferCommit<T> {
        BufferCommit { inner, index }
    }

    pub fn into_inner(self) -> Vec<T> {
        self.inner
    }

    #[inline]
    pub fn at_index(&self) -> usize {
        self.index
    }
}

impl<T> AsRef<[T]> for BufferCommit<T> {
    fn as_ref(&self) -> &[T] {
        self.inner.as_ref()
    }
}

impl<T> AsMut<[T]> for BufferCommit<T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.inner.as_mut()
    }
}

impl<T> Deref for BufferCommit<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> DerefMut for BufferCommit<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

macro_rules! impl_slice {
    (@inner $buffer:ty $( , $lt:lifetime )* ) => {
        impl<$( $lt, )* T> Buffer<T> for $buffer
        where
            T: Clone,
            [T]: ToOwned,
        {
            type Error = Infallible;

            fn len(&self) -> Length {
                Length::Bounded(<Self as AsRef<[T]>>::as_ref(self).len())
            }

            fn commit(&mut self, slice: Option<BufferCommit<T>>) -> Result<(), Infallible> {
                slice.map(|slice| {
                    let index = slice.at_index();
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
                range: Range<usize>,
            ) -> BufferSlice<'a, T> {
                let index = range.start;
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

    use super::{Buffer, BufferCommit, BufferSlice};
    use super::length::Length;

    impl Buffer<u8> for RefCell<File> {
        type Error = io::Error;

        fn len(&self) -> Length {
            Length::Bounded(
                self.borrow()
                    .metadata()
                    .map(|data| data.len() as usize)
                    .unwrap_or(0),
            )
        }

        fn commit(
            &mut self,
            slice: Option<BufferCommit<u8>>,
        ) -> Result<(), Self::Error> {
            slice
                .map(|slice| {
                    let index = slice.at_index();
                    let end = index + slice.as_ref().len();
                    let mut refmut = self.borrow_mut();
                    refmut
                        .seek(SeekFrom::Start(index as u64))
                        .and_then(|_| refmut.write(&slice.as_ref()[index..end]))
                        .map(|_| ())
                })
                .unwrap_or(Ok(()))
        }

        unsafe fn slice_unchecked<'a>(
            &'a self,
            range: Range<usize>,
        ) -> BufferSlice<'a, u8> {
            let index = range.start;
            let len = range.end - range.start;
            let mut vec = Vec::with_capacity(len);
            vec.set_len(len);
            let mut refmut = self.borrow_mut();
            refmut
                .seek(SeekFrom::Start(index as u64))
                .and_then(|_| refmut.read_exact(&mut vec[..]))
                .unwrap_or_else(|err| {
                    panic!("could't read from File Buffer: {:?}", err)
                });
            BufferSlice::new_owned(vec, index)
        }

        fn slice<'a>(
            &'a self,
            range: Range<usize>,
        ) -> Option<BufferSlice<'a, u8>> {
            let index = range.start;
            let mut vec = Vec::with_capacity(range.end - range.start);
            let mut refmut = self.borrow_mut();
            refmut
                .seek(SeekFrom::Start(index as u64))
                .and_then(|_| refmut.read_exact(&mut vec[range]))
                .map(move |_| BufferSlice::new_owned(vec, index))
                .ok()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer() {
        let mut buffer = vec![0; 1024];
        let commit = {
            let mut slice = buffer.slice(256..512).unwrap();
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
