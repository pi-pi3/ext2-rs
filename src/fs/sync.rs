use core::fmt::{self, Debug};
use core::nonzero::NonZero;
use core::iter::Iterator;

use alloc::{String, Vec};
use alloc::arc::Arc;

use spin::{Mutex, MutexGuard};
use genfs::*;

use error::Error;
use sector::{Address, SectorSize};
use volume::Volume;
use sys::inode::Inode as RawInode;

use super::Ext2;

pub struct Synced<T> {
    inner: Arc<Mutex<T>>,
}

impl<T> Synced<T> {
    pub fn with_inner(inner: T) -> Synced<T> {
        Synced {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    pub fn inner<'a>(&'a self) -> MutexGuard<'a, T> {
        self.inner.lock()
    }
}

impl<T> Clone for Synced<T> {
    fn clone(&self) -> Self {
        Synced {
            inner: self.inner.clone(),
        }
    }
}

impl<S: SectorSize, V: Volume<u8, S>> Synced<Ext2<S, V>> {
    pub fn new(volume: V) -> Result<Synced<Ext2<S, V>>, Error> {
        Ext2::new(volume).map(|inner| Synced::with_inner(inner))
    }

    pub fn root_inode(&self) -> Inode<S, V> {
        self.inode_nth(2).unwrap()
    }

    pub fn inode_nth(&self, index: usize) -> Option<Inode<S, V>> {
        self.inodes_nth(index).next()
    }

    pub fn inodes(&self) -> Inodes<S, V> {
        self.inodes_nth(1)
    }

    pub fn inodes_nth(&self, index: usize) -> Inodes<S, V> {
        assert!(index > 0, "inodes are 1-indexed");
        let inner = self.inner();
        Inodes {
            fs: self.clone(),
            log_block_size: inner.log_block_size(),
            inode_size: inner.inode_size(),
            inodes_per_group: inner.inodes_count(),
            inodes_count: inner.total_inodes_count(),
            index,
        }
    }

    pub fn sector_size(&self) -> usize {
        S::SIZE
    }

    pub fn log_sector_size(&self) -> u32 {
        S::LOG_SIZE
    }
}

impl<S: SectorSize, V: Volume<u8, S>> Fs for Synced<Ext2<S, V>> {
    type Path = [u8];
    type PathOwned = Vec<u8>;
    type File = Inode<S, V>;
    type Dir = Directory<S, V>;
    type DirEntry = DirectoryEntry;
    type Metadata = (); // TODO
    type Permissions = (); // TODO
    type Error = Error;

    fn open(
        &self,
        abs_path: &Self::Path,
        _options: &OpenOptions<Self::Permissions>,
    ) -> Result<Self::File, Self::Error> {
        fn inner<'a, S, V, I>(
            fs: &Synced<Ext2<S, V>>,
            inode: Inode<S, V>,
            mut path: I,
            abs_path: &[u8],
        ) -> Result<Inode<S, V>, Error>
        where
            S: SectorSize,
            V: Volume<u8, S>,
            I: Iterator<Item = &'a [u8]>,
        {
            let name = match path.next() {
                Some(name) => name,
                None => return Ok(inode),
            };

            let mut dir =
                inode.directory().ok_or_else(|| Error::NotADirectory {
                    inode: inode.num,
                    name: String::from_utf8_lossy(abs_path).into_owned(),
                })?;

            let entry = dir.find(|entry| {
                entry.is_err() || entry.as_ref().unwrap().name == name
            }).ok_or_else(|| Error::NotFound {
                name: String::from_utf8_lossy(abs_path).into_owned(),
            })??;

            let inode = fs.inode_nth(entry.inode)
                .ok_or(Error::InodeNotFound { inode: inode.num })?;

            inner(fs, inode, path, abs_path)
        }

        if abs_path.len() == 0 || abs_path[0] != b'/' {
            return Err(Error::NotAbsolute {
                name: String::from_utf8_lossy(abs_path).into_owned(),
            });
        }

        if abs_path == b"/" {
            return Ok(self.root_inode());
        }

        let mut path = abs_path.split(|byte| *byte == b'/');
        path.next();
        let root = self.root_inode();

        inner(self, root, path, abs_path)
    }

    fn remove_file(&mut self, _path: &Self::Path) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn metadata(
        &self,
        _path: &Self::Path,
    ) -> Result<Self::Metadata, Self::Error> {
        unimplemented!()
    }

    fn symlink_metadata(
        &self,
        _path: &Self::Path,
    ) -> Result<Self::Metadata, Self::Error> {
        unimplemented!()
    }

    fn rename(
        &mut self,
        _from: &Self::Path,
        _to: &Self::Path,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn copy(
        &mut self,
        _from: &Self::Path,
        _to: &Self::Path,
    ) -> Result<u64, Self::Error> {
        unimplemented!()
    }

    fn hard_link(
        &mut self,
        _src: &Self::Path,
        _dst: &Self::Path,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn symlink(
        &mut self,
        _src: &Self::Path,
        _dst: &Self::Path,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn read_link(
        &self,
        _path: &Self::Path,
    ) -> Result<Self::PathOwned, Self::Error> {
        unimplemented!()
    }

    fn canonicalize(
        &self,
        _path: &Self::Path,
    ) -> Result<Self::PathOwned, Self::Error> {
        unimplemented!()
    }

    fn create_dir(
        &mut self,
        _path: &Self::Path,
        _options: &DirOptions<Self::Permissions>,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn remove_dir(&mut self, _path: &Self::Path) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn remove_dir_all(
        &mut self,
        _path: &Self::Path,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn read_dir(&self, path: &Self::Path) -> Result<Self::Dir, Self::Error> {
        let inode = self.open(path, OpenOptions::new().read(true))?;
        inode.directory().ok_or(Error::NotADirectory {
            inode: inode.num,
            name: String::from_utf8_lossy(path).into_owned(),
        })
    }

    fn set_permissions(
        &mut self,
        _path: &Self::Path,
        _perm: Self::Permissions,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }
}

impl<S: SectorSize, V: Volume<u8, S>> Debug for Synced<Ext2<S, V>> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Synced<Ext2<{}>>", S::SIZE)
    }
}

#[derive(Debug, Clone)]
pub struct Inodes<S: SectorSize, V: Volume<u8, S>> {
    fs: Synced<Ext2<S, V>>,
    log_block_size: u32,
    inode_size: usize,
    inodes_per_group: usize,
    inodes_count: usize,
    index: usize,
}

impl<S: SectorSize, V: Volume<u8, S>> Iterator for Inodes<S, V> {
    type Item = Inode<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.inodes_count {
            let block_group = (self.index - 1) / self.inodes_per_group;
            let index = (self.index - 1) % self.inodes_per_group;
            self.index += 1;

            let fs = self.fs.inner();

            let inodes_block =
                fs.block_groups.inner[block_group].inode_table_block;

            let offset = Address::with_block_size(
                inodes_block,
                (index * self.inode_size) as i32,
                self.log_block_size,
            );
            let raw = unsafe {
                RawInode::find_inode(&fs.volume, offset, self.inode_size).ok()
            };
            raw.map(|(raw, offset)| {
                Inode::new(
                    self.fs.clone(),
                    raw,
                    offset,
                    (self.index - 1) as u32,
                )
            })
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct Inode<S: SectorSize, V: Volume<u8, S>> {
    fs: Synced<Ext2<S, V>>,
    inner: RawInode,
    addr: Address<S>,
    num: u32,
}

impl<S: SectorSize, V: Volume<u8, S>> Clone for Inode<S, V> {
    fn clone(&self) -> Self {
        Inode {
            fs: self.fs.clone(),
            inner: self.inner,
            addr: self.addr,
            num: self.num,
        }
    }
}

impl<S: SectorSize, V: Volume<u8, S>> Inode<S, V> {
    pub fn new(
        fs: Synced<Ext2<S, V>>,
        inner: RawInode,
        addr: Address<S>,
        num: u32,
    ) -> Inode<S, V> {
        Inode {
            fs,
            inner,
            addr,
            num,
        }
    }

    pub fn read_to_end(&self, buf: &mut Vec<u8>) -> Result<usize, Error> {
        let total_size = self.size();
        let capacity = buf.capacity();
        if capacity < total_size {
            buf.reserve_exact(total_size - capacity);
        }
        unsafe {
            buf.set_len(total_size);
        }
        let size = self.read(&mut buf[..]);
        size.and_then(|size| {
            unsafe {
                buf.set_len(size);
            }
            Ok(size)
        }).or_else(|err| {
            unsafe {
                buf.set_len(0);
            }
            Err(err)
        })
    }

    pub fn blocks(&self) -> InodeBlocks<S, V> {
        InodeBlocks {
            inode: self.clone(),
            index: 0,
        }
    }

    pub fn directory(&self) -> Option<Directory<S, V>> {
        if self.is_dir() {
            Some(Directory {
                blocks: self.blocks(),
                offset: 0,
                buffer: None,
                block_size: {
                    let fs = self.fs.inner();
                    fs.block_size()
                },
            })
        } else {
            None
        }
    }

    pub fn is_dir(&self) -> bool {
        use sys::inode::TypePerm;
        unsafe { self.inner.type_perm.contains(TypePerm::DIRECTORY) }
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

        fn block_index<S: SectorSize, V: Volume<u8, S>>(
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

        let fs = self.fs.inner();

        let bs4 = fs.block_size() / 4;
        let log_block_size = fs.log_block_size();

        if index < 12 {
            return Ok(NonZero::new(self.inner.direct_pointer[index]));
        }

        index -= 12;

        if index < bs4 {
            let block = self.inner.indirect_pointer;
            return block_index(&fs.volume, block, index, log_block_size);
        }

        index -= bs4;

        if index < bs4 * bs4 {
            let indirect_index = index >> (log_block_size + 2);
            let block = match block_index(
                &fs.volume,
                self.inner.doubly_indirect,
                indirect_index,
                log_block_size,
            ) {
                Ok(Some(block)) => block.get(),
                Ok(None) => return Ok(None),
                Err(err) => return Err(err),
            };
            return block_index(
                &fs.volume,
                block,
                index & (bs4 - 1),
                log_block_size,
            );
        }

        index -= bs4 * bs4;

        if index < bs4 * bs4 * bs4 {
            let doubly_index = index >> (2 * log_block_size + 4);
            let indirect = match block_index(
                &fs.volume,
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
                &fs.volume,
                indirect as u32,
                indirect_index,
                log_block_size,
            ) {
                Ok(Some(block)) => block.get(),
                Ok(None) => return Ok(None),
                Err(err) => return Err(err),
            };
            return block_index(
                &fs.volume,
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

impl<S: SectorSize, V: Volume<u8, S>> File for Inode<S, V> {
    type Error = Error;

    fn read(&self, buf: &mut [u8]) -> Result<usize, Error> {
        let total_size = self.size();
        let block_size = {
            let fs = self.fs.inner();
            fs.block_size()
        };
        let mut offset = 0;

        for block in self.blocks() {
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

    fn write(&mut self, _buf: &[u8]) -> Result<usize, Self::Error> {
        unimplemented!()
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn seek(&mut self, _pos: SeekFrom) -> Result<u64, Self::Error> {
        unimplemented!()
    }
}

#[derive(Debug, Clone)]
pub struct InodeBlocks<S: SectorSize, V: Volume<u8, S>> {
    inode: Inode<S, V>,
    index: usize,
}

impl<S: SectorSize, V: Volume<u8, S>> Iterator for InodeBlocks<S, V> {
    type Item = Result<(Vec<u8>, Address<S>), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let block = self.inode.try_block(self.index);
        let block = match block {
            Ok(Some(ok)) => ok,
            Ok(None) => return None,
            Err(err) => return Some(Err(err)),
        };

        self.index += 1;
        let fs = self.inode.fs.inner();

        let block = block.get();
        let log_block_size = fs.log_block_size();
        let offset = Address::with_block_size(block, 0, log_block_size);
        let end = Address::with_block_size(block + 1, 0, log_block_size);

        let slice = fs.volume
            .slice(offset..end)
            .map(|slice| (slice.to_vec(), offset))
            .map_err(|err| err.into());
        Some(slice)
    }
}

#[derive(Debug, Clone)]
pub struct Directory<S: SectorSize, V: Volume<u8, S>> {
    blocks: InodeBlocks<S, V>,
    offset: usize,
    buffer: Option<Vec<u8>>,
    block_size: usize,
}

impl<S: SectorSize, V: Volume<u8, S>> Dir<DirectoryEntry, Error>
    for Directory<S, V>
{
}

impl<S: SectorSize, V: Volume<u8, S>> Iterator for Directory<S, V> {
    type Item = Result<DirectoryEntry, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.buffer.is_none() || self.offset >= self.block_size {
            self.buffer = match self.blocks.next() {
                None => return None,
                Some(Ok((block, _))) => Some(block),
                Some(Err(err)) => return Some(Err(err)),
            };

            self.offset = 0;
        }

        let buffer = &self.buffer.as_ref().unwrap()[self.offset..];

        let inode = buffer[0] as u32 | (buffer[1] as u32) << 8
            | (buffer[2] as u32) << 16
            | (buffer[3] as u32) << 24;
        if inode == 0 {
            return None;
        }

        let size = buffer[4] as u16 | (buffer[5] as u16) << 8;
        let len = buffer[6];
        let ty = buffer[7];

        let name = buffer[8..8 + len as usize].to_vec();

        self.offset += size as usize;

        Some(Ok(DirectoryEntry {
            name: name,
            inode: inode as usize,
            ty: ty,
        }))
    }
}

#[derive(Clone)]
pub struct DirectoryEntry {
    pub name: Vec<u8>,
    pub inode: usize,
    pub ty: u8,
}

impl DirEntry for DirectoryEntry {
    type Path = [u8];
    type PathOwned = Vec<u8>;
    type Metadata = (); // TODO
    type FileType = u8; // TODO: enum FileType
    type Error = Error;

    fn path(&self) -> Self::PathOwned {
        unimplemented!()
    }

    fn metadata(&self) -> Result<Self::Metadata, Self::Error> {
        unimplemented!()
    }

    fn file_type(&self) -> Result<Self::FileType, Self::Error> {
        Ok(self.ty)
    }

    fn file_name(&self) -> &Self::Path {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::cell::RefCell;

    use genfs::{File as GenFile, Fs, OpenOptions};

    use sector::{SectorSize, Size512};
    use volume::Volume;

    use super::{Ext2, Inode, Synced};

    #[test]
    fn file() {
        let file = RefCell::new(File::open("ext2.img").unwrap());
        let fs = Synced::<Ext2<Size512, _>>::new(file);

        assert!(
            fs.is_ok(),
            "Err({:?})",
            fs.err().unwrap_or_else(|| unreachable!()),
        );

        let fs = fs.unwrap();
        let inner = fs.inner();

        let vers = inner.version();
        println!("version: {}.{}", vers.0, vers.1);
        assert_eq!(128, inner.inode_size());
    }

    #[test]
    fn inodes() {
        let file = RefCell::new(File::open("ext2.img").unwrap());
        let fs = Synced::<Ext2<Size512, _>>::new(file);

        assert!(
            fs.is_ok(),
            "Err({:?})",
            fs.err().unwrap_or_else(|| unreachable!()),
        );

        let fs = fs.unwrap();

        let inodes = fs.inodes().filter(|inode| inode.in_use());
        for inode in inodes {
            println!("{:?}", inode);
        }
    }

    #[test]
    fn inode_blocks() {
        use std::str;
        let file = RefCell::new(File::open("ext2.img").unwrap());
        let fs = Synced::<Ext2<Size512, _>>::new(file).unwrap();

        let inodes = fs.inodes().filter(|inode| {
            inode.in_use() && inode.uid() == 1000 && inode.size() < 1024
        });
        for inode in inodes {
            println!("{:?}", inode);
            let size = inode.size();
            for block in inode.blocks() {
                let (data, _) = block.unwrap();
                assert_eq!(data.len(), {
                    let fs = fs.inner();
                    fs.block_size()
                });
                println!("{:?}", &data[..size]);
                let _ = str::from_utf8(&data[..size])
                    .map(|string| println!("{}", string));
            }
        }
    }

    #[test]
    fn read_inode() {
        let file = RefCell::new(File::open("ext2.img").unwrap());
        let fs = Synced::<Ext2<Size512, _>>::new(file).unwrap();

        let inodes = fs.inodes().filter(|inode| {
            inode.in_use() && inode.uid() == 1000 && inode.size() < 1024
        });
        for inode in inodes {
            let mut buf = Vec::with_capacity(inode.size());
            unsafe {
                buf.set_len(inode.size());
            }
            let size = inode.read(&mut buf[..]);
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
        let fs = Synced::<Ext2<Size512, _>>::new(file).unwrap();

        let inodes = fs.inodes().filter(|inode| {
            inode.in_use() && inode.uid() == 1000 && inode.size() == 537600
        });
        for inode in inodes {
            let mut buf = Vec::with_capacity(inode.size());
            unsafe {
                buf.set_len(inode.size());
            }
            let size = inode.read(&mut buf[..]);
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

    #[test]
    fn walkdir() {
        use std::str;

        fn walk<'vol, S: SectorSize, V: Volume<u8, S>>(
            fs: &'vol Synced<Ext2<S, V>>,
            inode: Inode<S, V>,
            name: String,
        ) {
            inode.directory().map(|dir| {
                for entry in dir {
                    assert!(entry.is_ok());
                    let entry = entry.unwrap();
                    let entry_name = str::from_utf8(&entry.name).unwrap_or("?");
                    println!("{}/{} => {}", name, entry_name, entry.inode,);
                    if entry_name != "." && entry_name != ".." {
                        walk(
                            fs,
                            fs.inode_nth(entry.inode).unwrap(),
                            format!("{}/{}", name, entry_name),
                        );
                    }
                }
            });
        }

        let file = RefCell::new(File::open("ext2.img").unwrap());
        let fs = Synced::<Ext2<Size512, _>>::new(file).unwrap();

        let root = fs.root_inode();
        walk(&fs, root, String::new());
    }

    #[test]
    fn find() {
        use std::str;
        let file = RefCell::new(File::open("ext2.img").unwrap());
        let fs = Synced::<Ext2<Size512, _>>::new(file).unwrap();

        let found = fs.open(b"/home/funky/README.md", &OpenOptions::new());

        assert!(found.is_ok());
        let inode = found.unwrap();
        let mut vec = Vec::new();
        assert!(inode.read_to_end(&mut vec).is_ok());
        println!("{}", str::from_utf8(&vec).unwrap());
    }
}
