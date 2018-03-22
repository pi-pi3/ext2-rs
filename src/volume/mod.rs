use core::mem;
use core::slice;
use core::ops::{Deref, DerefMut, Range};

use alloc::Vec;
use alloc::boxed::Box;
use alloc::borrow::{Cow, ToOwned};

use error::Error;
use sector::{Address, SectorSize};

pub mod size;
use self::size::Size;

pub trait Volume<T: Clone, S: SectorSize> {
    type Error: Into<Error>;

    fn size(&self) -> Size<S>;
    fn commit(
        &mut self,
        slice: Option<VolumeCommit<T, S>>,
    ) -> Result<(), Self::Error>;
    unsafe fn slice_unchecked<'a>(
        &'a self,
        range: Range<Address<S>>,
    ) -> VolumeSlice<'a, T, S>;

    fn slice<'a>(
        &'a self,
        range: Range<Address<S>>,
    ) -> Result<VolumeSlice<'a, T, S>, Self::Error>;
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct VolumeSlice<'a, T: 'a + Clone, S: SectorSize> {
    inner: Cow<'a, [T]>,
    index: Address<S>,
}

impl<T: Clone, S: SectorSize> VolumeSlice<'static, T, S> {
    pub fn with_static(inner: &'static [T]) -> VolumeSlice<'static, T, S> {
        VolumeSlice {
            inner: Cow::Borrowed(inner),
            index: Address::new(0, 0),
        }
    }

    pub fn new_owned(
        inner: <[T] as ToOwned>::Owned,
        index: Address<S>,
    ) -> VolumeSlice<'static, T, S> {
        VolumeSlice {
            inner: Cow::Owned(inner),
            index,
        }
    }
}

impl<'a, T: Clone, S: SectorSize> VolumeSlice<'a, T, S> {
    pub fn new(inner: &'a [T], index: Address<S>) -> VolumeSlice<'a, T, S> {
        VolumeSlice {
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

    pub fn address(&self) -> Address<S> {
        self.index
    }
}

impl<'a, S: SectorSize> VolumeSlice<'a, u8, S> {
    pub unsafe fn dynamic_cast<T: Copy>(&self) -> (T, Address<S>) {
        assert!(self.inner.len() >= mem::size_of::<T>());
        let index = self.index;
        let cast = mem::transmute_copy(self.inner.as_ptr().as_ref().unwrap());
        (cast, index)
    }

    pub fn from_cast<T: Copy>(
        cast: &'a T,
        index: Address<S>,
    ) -> VolumeSlice<'a, u8, S> {
        let len = mem::size_of::<T>();
        let ptr = cast as *const T as *const u8;
        let slice = unsafe { slice::from_raw_parts(ptr, len) };
        VolumeSlice::new(slice, index)
    }
}

impl<'a, T: Clone, S: SectorSize> VolumeSlice<'a, T, S> {
    pub fn commit(self) -> Option<VolumeCommit<T, S>> {
        if self.is_mutated() {
            Some(VolumeCommit::new(self.inner.into_owned(), self.index))
        } else {
            None
        }
    }
}

impl<'a, T: Clone, S: SectorSize> AsRef<[T]> for VolumeSlice<'a, T, S> {
    fn as_ref(&self) -> &[T] {
        self.inner.as_ref()
    }
}

impl<'a, T: Clone, S: SectorSize> AsMut<[T]> for VolumeSlice<'a, T, S> {
    fn as_mut(&mut self) -> &mut [T] {
        self.inner.to_mut().as_mut()
    }
}

impl<'a, T: Clone, S: SectorSize> Deref for VolumeSlice<'a, T, S> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<'a, T: Clone, S: SectorSize> DerefMut for VolumeSlice<'a, T, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

pub struct VolumeCommit<T, S: SectorSize> {
    inner: Vec<T>,
    index: Address<S>,
}

impl<T: Clone, S: SectorSize> VolumeCommit<T, S> {
    pub fn with_vec(inner: Vec<T>) -> VolumeCommit<T, S> {
        VolumeCommit {
            inner,
            index: Address::new(0, 0),
        }
    }
}

impl<T: Clone, S: SectorSize> VolumeCommit<T, S> {
    pub fn new(inner: Vec<T>, index: Address<S>) -> VolumeCommit<T, S> {
        VolumeCommit { inner, index }
    }

    pub fn into_inner(self) -> Vec<T> {
        self.inner
    }

    pub fn address(&self) -> Address<S> {
        self.index
    }
}

impl<T: Clone, S: SectorSize> AsRef<[T]> for VolumeCommit<T, S> {
    fn as_ref(&self) -> &[T] {
        self.inner.as_ref()
    }
}

impl<T: Clone, S: SectorSize> AsMut<[T]> for VolumeCommit<T, S> {
    fn as_mut(&mut self) -> &mut [T] {
        self.inner.as_mut()
    }
}

impl<T: Clone, S: SectorSize> Deref for VolumeCommit<T, S> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T: Clone, S: SectorSize> DerefMut for VolumeCommit<T, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

macro_rules! impl_slice {
    (@inner $volume:ty $( , $lt:lifetime )* ) => {
        impl<$( $lt, )* T: Clone, S: SectorSize> Volume<T, S>
            for $volume
        {
            type Error = Error;

            fn size(&self) -> Size<S> {
                Size::Bounded(
                    Address::from(<Self as AsRef<[T]>>::as_ref(self).len())
                )
            }

            fn commit(
                &mut self,
                slice: Option<VolumeCommit<T, S>>,
            ) -> Result<(), Self::Error> {
                slice.map(|slice| {
                    let index = slice.address().into_index() as usize;
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
            ) -> VolumeSlice<'a, T, S> {
                let index = range.start;
                let range = range.start.into_index() as usize
                    ..range.end.into_index() as usize;
                VolumeSlice::new(
                    <Self as AsRef<[T]>>::as_ref(self).get_unchecked(range),
                    index,
                )
            }

            fn slice<'a>(
                &'a self,
                range: Range<Address<S>>,
            ) -> Result<VolumeSlice<'a, T, S>, Self::Error> {
                if self.size() >= range.end {
                    unsafe { Ok(self.slice_unchecked(range)) }
                } else {
                    Err(Error::AddressOutOfBounds {
                        sector: range.end.sector(),
                        offset: range.end.offset(),
                        size: range.end.sector_size()
                    })
                }
            }
        }
    };
    ($volume:ty) => {
        impl_slice!(@inner $volume);
    };
    ($volume:ty $( , $lt:lifetime )* ) => {
        impl_slice!(@inner $volume $( , $lt )* );
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

    use sector::{Address, SectorSize};

    use super::{Volume, VolumeCommit, VolumeSlice};
    use super::size::Size;

    impl<S: SectorSize> Volume<u8, S> for RefCell<File> {
        type Error = io::Error;

        fn size(&self) -> Size<S> {
            Size::Bounded(
                self.borrow()
                    .metadata()
                    .map(|data| Address::from(data.len()))
                    .unwrap_or(Address::new(0, 0)),
            )
        }

        fn commit(
            &mut self,
            slice: Option<VolumeCommit<u8, S>>,
        ) -> Result<(), Self::Error> {
            slice
                .map(|slice| {
                    let index = slice.address();
                    let mut refmut = self.borrow_mut();
                    refmut
                        .seek(SeekFrom::Start(index.into_index()))
                        .and_then(|_| refmut.write(slice.as_ref()))
                        .map(|_| ())
                })
                .unwrap_or(Ok(()))
        }

        unsafe fn slice_unchecked<'a>(
            &'a self,
            range: Range<Address<S>>,
        ) -> VolumeSlice<'a, u8, S> {
            let index = range.start;
            let len = range.end - range.start;
            let mut vec = Vec::with_capacity(len.into_index() as usize);
            vec.set_len(len.into_index() as usize);
            let mut refmut = self.borrow_mut();
            refmut
                .seek(SeekFrom::Start(index.into_index()))
                .and_then(|_| refmut.read_exact(&mut vec[..]))
                .unwrap_or_else(|err| {
                    panic!("could't read from File Volume: {:?}", err)
                });
            VolumeSlice::new_owned(vec, index)
        }

        fn slice<'a>(
            &'a self,
            range: Range<Address<S>>,
        ) -> Result<VolumeSlice<'a, u8, S>, Self::Error> {
            let index = range.start;
            let mut vec = Vec::with_capacity((range.end - range.start)
                .into_index()
                as usize);
            unsafe {
                vec.set_len((range.end - range.start).into_index() as usize);
            }
            let mut refmut = self.borrow_mut();
            refmut
                .seek(SeekFrom::Start(index.into_index()))
                .and_then(|_| refmut.read_exact(&mut vec[..]))
                .map(move |_| VolumeSlice::new_owned(vec, index))
        }
    }
}

#[cfg(test)]
mod tests {
    use sector::{Address, Size512};
    use super::*;

    #[test]
    fn volume() {
        let mut volume = vec![0; 1024];
        let commit = {
            let mut slice = volume
                .slice(
                    Address::<Size512>::from(256_u64)
                        ..Address::<Size512>::from(512_u64),
                )
                .unwrap();
            slice.iter_mut().for_each(|x| *x = 1);
            slice.commit()
        };
        assert!(volume.commit(commit).is_ok());

        for (i, &x) in volume.iter().enumerate() {
            if i < 256 || i >= 512 {
                assert_eq!(x, 0);
            } else {
                assert_eq!(x, 1);
            }
        }
    }
}
