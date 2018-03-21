use core::mem;
use core::fmt::{self, Debug};
use core::nonzero::NonZero;

use alloc::Vec;

use error::Error;
use sector::{Address, Size};
use volume::{Volume, VolumeSlice};
use sys::superblock::Superblock;
use sys::block_group::BlockGroupDescriptor;
use sys::inode::Inode as RawInode;

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

impl<S: Size, V: Volume<u8, Address<S>>> Ext2<S, V> {
    pub fn new(volume: V) -> Result<Ext2<S, V>, Error> {
        let superblock = unsafe { Struct::from(Superblock::find(&volume)?) };
        let block_groups_offset = Address::with_block_size(
            superblock.inner.first_data_block + 1,
            0,
            superblock.inner.log_block_size + 10,
        );
        let block_groups_count = superblock
            .inner
            .block_group_count()
            .map(|count| count as usize)
            .map_err(|(a, b)| Error::BadBlockGroupCount {
                by_blocks: a,
                by_inodes: b,
            })?;
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
            self.volume.commit(commit).map_err(|err| err.into())?;
        }

        // block group descriptors
        let mut offset = self.block_groups.offset;
        for descr in &self.block_groups.inner {
            let slice = VolumeSlice::from_cast(descr, offset);
            let commit = slice.commit();
            self.volume.commit(commit).map_err(|err| err.into())?;
            offset =
                offset + Address::from(mem::size_of::<BlockGroupDescriptor>());
        }

        Ok(())
    }

    pub fn read_inode<'a>(
        &'a self,
        buf: &mut [u8],
        inode: &Inode<'a, S, V>,
    ) -> Result<usize, Error> {
        let total_size = inode.size();
        let block_size = self.block_size();
        let mut offset = 0;

        for block in InodeBlocks::new(&inode) {
            match block {
                Ok((data, _)) => {
                    let data_size = block_size
                        .min(total_size - offset)
                        .min(buf.len() - offset);
                    let end = offset + data_size;
                    buf[offset..end].copy_from_slice(&data[..data_size]);
                    offset += data_size;
                }
                Err(err) => return Err(err.into()),
            }
        }

        Ok(offset)
    }

    pub fn write_inode<'a>(
        &'a self,
        _inode: &(Inode<'a, S, V>, Address<S>),
        _buf: &[u8],
    ) -> Result<usize, Error> {
        unimplemented!()
    }

    pub fn root_inode<'a>(&'a self) -> (Inode<'a, S, V>, Address<S>) {
        self.inode_nth(2).unwrap()
    }

    pub fn inode_nth<'a>(
        &'a self,
        index: usize,
    ) -> Option<(Inode<'a, S, V>, Address<S>)> {
        self.inodes_nth(index).next()
    }

    pub fn inodes<'a>(&'a self) -> Inodes<'a, S, V> {
        self.inodes_nth(1)
    }

    pub fn inodes_nth<'a>(&'a self, index: usize) -> Inodes<'a, S, V> {
        assert!(index > 0, "inodes are 1-indexed");
        Inodes {
            fs: self,
            block_groups: &self.block_groups.inner,
            log_block_size: self.log_block_size(),
            inode_size: self.inode_size(),
            inodes_per_group: self.inodes_count(),
            inodes_count: self.total_inodes_count(),
            index,
        }
    }
}

impl<S: Size, V: Volume<u8, Address<S>>> Ext2<S, V> {
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

    pub fn inode_size<'a>(&'a self) -> usize {
        if self.version().0 == 0 {
            mem::size_of::<Inode<'a, S, V>>()
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
            .map_err(|(a, b)| Error::BadBlockGroupCount {
                by_blocks: a,
                by_inodes: b,
            })
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

    pub fn sector_size(&self) -> usize {
        S::SIZE
    }

    pub fn log_sector_size(&self) -> u32 {
        S::LOG_SIZE
    }
}

impl<S: Size, V: Volume<u8, Address<S>>> Debug for Ext2<S, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Ext2<{}>", S::SIZE)
    }
}

pub struct Inodes<'a, S: 'a + Size, V: 'a + Volume<u8, Address<S>>> {
    fs: &'a Ext2<S, V>,
    block_groups: &'a [BlockGroupDescriptor],
    log_block_size: u32,
    inode_size: usize,
    inodes_per_group: usize,
    inodes_count: usize,
    index: usize,
}

impl<'a, S: Size, V: 'a + Volume<u8, Address<S>>> Iterator
    for Inodes<'a, S, V>
{
    type Item = (Inode<'a, S, V>, Address<S>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.inodes_count {
            let block_group = (self.index - 1) / self.inodes_per_group;
            let index = (self.index - 1) % self.inodes_per_group;
            self.index += 1;

            let inodes_block = self.block_groups[block_group].inode_table_block;

            let offset = Address::with_block_size(
                inodes_block,
                (index * self.inode_size) as i32,
                self.log_block_size,
            );
            let raw = unsafe {
                RawInode::find_inode(&self.fs.volume, offset, self.inode_size)
                    .ok()
            };
            raw.map(|(raw, offset)| (Inode::new(self.fs, raw), offset))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct Inode<'a, S: 'a + Size, V: 'a + Volume<u8, Address<S>>> {
    fs: &'a Ext2<S, V>,
    inner: RawInode,
}

impl<'a, S: 'a + Size, V: 'a + Volume<u8, Address<S>>> Inode<'a, S, V> {
    pub fn new(fs: &'a Ext2<S, V>, inner: RawInode) -> Inode<'a, S, V> {
        Inode { fs, inner }
    }

    pub fn block(&self, index: usize) -> Option<NonZero<u32>> {
        self.try_block(index).ok().and_then(|block| block)
    }

    pub fn try_block(
        &self,
        mut index: usize,
    ) -> Result<Option<NonZero<u32>>, Error> {
        // number of blocks in direct table: 12
        // number of blocks in indirect table: block_size/4
        //   why?
        //     - a block is n bytes long
        //     - a block address occupies 32 bits, or 4 bytes
        //     - thus, n/4
        // number of blocks in doubly table: (block_size/4)^2
        //   why?
        //     - every entry in the doubly table points to another block
        //     - that's n/4 blocks, where n is the block size
        //     - every block contains n/4 block pointers
        //     - that's n/4 blocks with n/4 pointers each = (n/4)^2
        // number of blocks in triply table: (block_size/4)^3

        fn block_index<S: Size, V: Volume<u8, Address<S>>>(
            volume: &V,
            block: u32,
            index: usize,
            log_block_size: u32,
        ) -> Result<Option<NonZero<u32>>, Error> {
            let offset = (index * 4) as i32;
            let end = offset + 4;
            let addr = Address::with_block_size(block, offset, log_block_size);
            let end = Address::with_block_size(block, end, log_block_size);
            let block = volume.slice(addr..end);
            match block {
                Ok(block) => unsafe {
                    Ok(NonZero::new(block.dynamic_cast::<u32>().0))
                },
                Err(err) => Err(err.into()),
            }
        }

        let bs4 = self.fs.block_size() / 4;
        let log_block_size = self.fs.log_block_size();

        if index < 12 {
            return Ok(NonZero::new(self.inner.direct_pointer[index]));
        }

        index -= 12;

        if index < bs4 {
            let block = self.inner.indirect_pointer;
            return block_index(&self.fs.volume, block, index, log_block_size);
        }

        index -= bs4;

        if index < bs4 * bs4 {
            let indirect_index = index >> (log_block_size + 2);
            let block = match block_index(
                &self.fs.volume,
                self.inner.doubly_indirect,
                indirect_index,
                log_block_size,
            ) {
                Ok(Some(block)) => block.get(),
                Ok(None) => return Ok(None),
                Err(err) => return Err(err),
            };
            return block_index(
                &self.fs.volume,
                block,
                index & (bs4 - 1),
                log_block_size,
            );
        }

        index -= bs4 * bs4;

        if index < bs4 * bs4 * bs4 {
            let doubly_index = index >> (2 * log_block_size + 4);
            let indirect = match block_index(
                &self.fs.volume,
                self.inner.triply_indirect,
                doubly_index,
                log_block_size,
            ) {
                Ok(Some(block)) => block.get(),
                Ok(None) => return Ok(None),
                Err(err) => return Err(err),
            };
            let indirect_index = (index >> (log_block_size + 2)) & (bs4 - 1);
            let block = match block_index(
                &self.fs.volume,
                indirect as u32,
                indirect_index,
                log_block_size,
            ) {
                Ok(Some(block)) => block.get(),
                Ok(None) => return Ok(None),
                Err(err) => return Err(err),
            };
            return block_index(
                &self.fs.volume,
                block,
                index & (bs4 - 1),
                log_block_size,
            );
        }

        Ok(None)
    }

    pub fn in_use(&self) -> bool {
        self.inner.hard_links > 0
    }

    pub fn uid(&self) -> u16 {
        self.inner.uid
    }

    pub fn sectors(&self) -> usize {
        self.inner.sectors_count as usize
    }

    pub fn size32(&self) -> u32 {
        self.inner.size_low
    }

    pub fn size64(&self) -> u64 {
        self.inner.size_low as u64 | (self.inner.size_high as u64) << 32
    }

    #[cfg(target_pointer_width = "64")]
    #[inline]
    pub fn size(&self) -> usize {
        self.size64() as usize
    }

    #[cfg(target_pointer_width = "32")]
    #[inline]
    pub fn size(&self) -> usize {
        self.size32() as usize
    }
}

pub struct InodeBlocks<'a: 'b, 'b, S: 'a + Size, V: 'a + Volume<u8, Address<S>>>
{
    inode: &'b Inode<'a, S, V>,
    index: usize,
}

impl<'a, 'b, S: Size, V: 'a + Volume<u8, Address<S>>>
    InodeBlocks<'a, 'b, S, V>
{
    pub fn new(inode: &'b Inode<'a, S, V>) -> InodeBlocks<'a, 'b, S, V> {
        InodeBlocks { inode, index: 0 }
    }
}

impl<'a, 'b, S: Size, V: 'a + Volume<u8, Address<S>>> Iterator
    for InodeBlocks<'a, 'b, S, V>
{
    type Item = Result<(VolumeSlice<'a, u8, Address<S>>, Address<S>), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let block = self.inode.try_block(self.index);
        let block = match block {
            Ok(Some(ok)) => ok,
            Ok(None) => return None,
            Err(err) => return Some(Err(err)),
        };

        self.index += 1;

        let block = block.get();
        let log_block_size = self.inode.fs.log_block_size();
        let offset = Address::with_block_size(block, 0, log_block_size);
        let end = Address::with_block_size(block + 1, 0, log_block_size);

        let slice = self.inode
            .fs
            .volume
            .slice(offset..end)
            .map(|slice| (slice, offset))
            .map_err(|err| err.into());
        Some(slice)
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::cell::RefCell;

    use sector::{Address, Size512};
    use volume::Volume;

    use super::{Ext2, InodeBlocks};

    #[test]
    fn file_len() {
        let file = RefCell::new(File::open("ext2.img").unwrap());
        assert_eq!(
            Address::<Size512>::from(2048_u64)
                - Address::<Size512>::from(1024_u64),
            Address::<Size512>::new(2, 0)
        );
        assert_eq!(
            unsafe {
                file.slice_unchecked(
                    Address::<Size512>::from(1024_u64)
                        ..Address::<Size512>::from(2048_u64),
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

    #[test]
    fn inode_blocks() {
        use std::str;
        let file = RefCell::new(File::open("ext2.img").unwrap());
        let fs = Ext2::<Size512, _>::new(file).unwrap();

        let inodes = fs.inodes().filter(|inode| {
            inode.0.in_use() && inode.0.uid() == 1000 && inode.0.size() < 1024
        });
        for inode in inodes {
            println!("{:?}", inode.0);
            let size = inode.0.size();
            for block in InodeBlocks::new(&inode.0) {
                let (data, _) = block.unwrap();
                assert_eq!(data.len(), fs.block_size());
                println!("{:?}", &data[..size]);
                let _ = str::from_utf8(&data[..size])
                    .map(|string| println!("{}", string));
            }
        }
    }

    #[test]
    fn read_inode() {
        let file = RefCell::new(File::open("ext2.img").unwrap());
        let fs = Ext2::<Size512, _>::new(file).unwrap();

        let inodes = fs.inodes().filter(|inode| {
            inode.0.in_use() && inode.0.uid() == 1000 && inode.0.size() < 1024
        });
        for (inode, _) in inodes {
            let mut buf = Vec::with_capacity(inode.size());
            unsafe {
                buf.set_len(inode.size());
            }
            let size = fs.read_inode(&mut buf[..], &inode);
            assert!(size.is_ok());
            let size = size.unwrap();
            assert_eq!(size, inode.size());
            unsafe {
                buf.set_len(size);
            }
        }
    }

    #[test]
    fn read_big() {
        let file = RefCell::new(File::open("ext2.img").unwrap());
        let fs = Ext2::<Size512, _>::new(file).unwrap();

        let inodes = fs.inodes().filter(|inode| {
            inode.0.in_use() && inode.0.uid() == 1000
                && inode.0.size() == 537600
        });
        for (inode, _) in inodes {
            let mut buf = Vec::with_capacity(inode.size());
            unsafe {
                buf.set_len(inode.size());
            }
            let size = fs.read_inode(&mut buf[..], &inode);
            assert!(size.is_ok());
            let size = size.unwrap();
            assert_eq!(size, inode.size());
            unsafe {
                buf.set_len(size);
            }

            for (i, &x) in buf.iter().enumerate() {
                if i & 1 == 0 {
                    assert_eq!(x, b'u', "{}", i);
                } else {
                    assert_eq!(x, b'\n', "{}", i);
                }
            }
        }
    }
}
