use core::mem;
use core::marker::PhantomData;

use alloc::Vec;

use error::Error;
use sector::{Address, Size};
use volume::{Volume, VolumeSlice};
use sys::superblock::Superblock;
use sys::block_group::BlockGroupDescriptor;
use sys::inode::Inode;

struct Struct<T, S: Size> {
    pub inner: T,
    pub offset: Address<S>,
}

impl<T, S: Size> From<(T, Address<S>)> for Struct<T, S> {
    #[inline]
    fn from((inner, offset): (T, Address<S>)) -> Struct<T, S> {
        Struct { inner, offset }
    }
}

/// Safe wrapper for raw sys structs
pub struct Ext2<S: Size, V: Volume<u8, Address<S>>> {
    volume: V,
    superblock: Struct<Superblock, S>,
    block_groups: Struct<Vec<BlockGroupDescriptor>, S>,
}

impl<S: Size + Copy, V: Volume<u8, Address<S>>> Ext2<S, V>
where
    Error: From<V::Error>,
{
    pub fn new(volume: V) -> Result<Ext2<S, V>, Error> {
        let superblock = unsafe { Struct::from(Superblock::find(&volume)?) };
        let block_groups_offset = Address::with_block_size(
            superblock.inner.first_data_block as usize + 1,
            0,
            superblock.inner.log_block_size + 10,
        );
        let block_groups_count = superblock
            .inner
            .block_group_count()
            .map(|count| count as usize)
            .map_err(|(a, b)| Error::BadBlockGroupCount(a, b))?;
        let block_groups = unsafe {
            BlockGroupDescriptor::find_descriptor_table(
                &volume,
                block_groups_offset,
                block_groups_count,
            )?
        };
        let block_groups = Struct::from(block_groups);
        Ok(Ext2 {
            volume,
            superblock,
            block_groups,
        })
    }

    #[allow(dead_code)]
    fn update_global(&mut self) -> Result<(), Error> {
        // superblock
        {
            let slice = VolumeSlice::from_cast(
                &self.superblock.inner,
                self.superblock.offset,
            );
            let commit = slice.commit();
            self.volume.commit(commit).map_err(|err| Error::from(err))?;
        }

        // block group descriptors
        let mut offset = self.block_groups.offset;
        for descr in &self.block_groups.inner {
            let slice = VolumeSlice::from_cast(descr, offset);
            let commit = slice.commit();
            self.volume.commit(commit).map_err(|err| Error::from(err))?;
            offset =
                offset + Address::from(mem::size_of::<BlockGroupDescriptor>());
        }

        Ok(())
    }

    #[allow(dead_code)]
    fn update_inode(
        &mut self,
        &(ref inode, offset): &(Inode, Address<S>),
    ) -> Result<(), Error> {
        let slice = VolumeSlice::from_cast(inode, offset);
        let commit = slice.commit();
        self.volume.commit(commit).map_err(|err| Error::from(err))
    }

    pub fn root_inode(&self) -> (Inode, Address<S>) {
        self.inode_nth(2).unwrap()
    }

    pub fn inode_nth(&self, index: usize) -> Option<(Inode, Address<S>)> {
        self.inodes_nth(index).next()
    }

    pub fn inodes<'a>(&'a self) -> Inodes<'a, S, V> {
        self.inodes_nth(1)
    }

    pub fn inodes_nth<'a>(&'a self, index: usize) -> Inodes<'a, S, V> {
        assert!(index > 0, "inodes are 1-indexed");
        Inodes {
            volume: &self.volume,
            block_groups: &self.block_groups.inner,
            log_block_size: self.log_block_size(),
            inode_size: self.inode_size(),
            inodes_per_group: self.inodes_count(),
            inodes_count: self.total_inodes_count(),
            index,
            _phantom: PhantomData,
        }
    }

    fn superblock(&self) -> &Superblock {
        &self.superblock.inner
    }

    #[allow(dead_code)]
    fn superblock_mut(&mut self) -> &mut Superblock {
        &mut self.superblock.inner
    }

    pub fn version(&self) -> (u32, u16) {
        (self.superblock().rev_major, self.superblock().rev_minor)
    }

    pub fn inode_size(&self) -> usize {
        if self.version().0 == 0 {
            mem::size_of::<Inode>()
        } else {
            // note: inodes bigger than 128 are not supported
            self.superblock().inode_size as usize
        }
    }

    pub fn inodes_count(&self) -> usize {
        self.superblock().inodes_per_group as _
    }

    pub fn total_inodes_count(&self) -> usize {
        self.superblock().inodes_count as _
    }

    pub fn block_group_count(&self) -> Result<usize, Error> {
        self.superblock()
            .block_group_count()
            .map(|count| count as usize)
            .map_err(|(a, b)| Error::BadBlockGroupCount(a, b))
    }

    pub fn total_block_count(&self) -> usize {
        self.superblock().blocks_count as _
    }

    pub fn free_block_count(&self) -> usize {
        self.superblock().free_blocks_count as _
    }

    pub fn block_size(&self) -> usize {
        self.superblock().block_size()
    }

    pub fn log_block_size(&self) -> u32 {
        self.superblock().log_block_size + 10
    }
}

pub struct Inodes<'a, S: Size, V: 'a + Volume<u8, Address<S>>> {
    volume: &'a V,
    block_groups: &'a [BlockGroupDescriptor],
    log_block_size: u32,
    inode_size: usize,
    inodes_per_group: usize,
    inodes_count: usize,
    index: usize,
    _phantom: PhantomData<S>,
}

impl<'a, S: Size + Copy, V: 'a + Volume<u8, Address<S>>> Iterator
    for Inodes<'a, S, V>
where
    Error: From<V::Error>,
{
    type Item = (Inode, Address<S>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.inodes_count {
            let block_group = (self.index - 1) / self.inodes_per_group;
            let index = (self.index - 1) % self.inodes_per_group;
            self.index += 1;

            let inodes_block =
                self.block_groups[block_group].inode_table_block as usize;

            let offset = Address::with_block_size(
                inodes_block,
                index * self.inode_size,
                self.log_block_size,
            );
            unsafe {
                Inode::find_inode(self.volume, offset, self.inode_size).ok()
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::cell::RefCell;

    use sector::{Address, Size512};
    use volume::Volume;

    use super::Ext2;

    #[test]
    fn file_len() {
        let file = RefCell::new(File::open("ext2.img").unwrap());
        assert_eq!(
            Address::<Size512>::from(2048_usize)
                - Address::<Size512>::from(1024_usize),
            Address::<Size512>::new(2, 0)
        );
        assert_eq!(
            unsafe {
                file.slice_unchecked(
                    Address::<Size512>::from(1024_usize)
                        ..Address::<Size512>::from(2048_usize),
                ).len()
            },
            1024
        );
    }

    #[test]
    fn file() {
        let file = RefCell::new(File::open("ext2.img").unwrap());
        let fs = Ext2::<Size512, _>::new(file);

        assert!(
            fs.is_ok(),
            "Err({:?})",
            fs.err().unwrap_or_else(|| unreachable!()),
        );

        let fs = fs.unwrap();

        let vers = fs.version();
        println!("version: {}.{}", vers.0, vers.1);
        assert_eq!(128, fs.inode_size());
    }

    #[test]
    fn inodes() {
        let file = RefCell::new(File::open("ext2.img").unwrap());
        let fs = Ext2::<Size512, _>::new(file);

        assert!(
            fs.is_ok(),
            "Err({:?})",
            fs.err().unwrap_or_else(|| unreachable!()),
        );

        let fs = fs.unwrap();

        let inodes = fs.inodes().filter(|inode| inode.0.in_use());
        for inode in inodes {
            println!("{:?}", inode);
        }
    }
}
