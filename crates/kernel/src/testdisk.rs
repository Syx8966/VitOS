#[cfg(feature = "uspace")]
mod imp {
    extern crate alloc;

    use alloc::{boxed::Box, string::String, vec, vec::Vec};
    use axdriver::prelude::BlockDriverOps;
    use axstd::println;

    const SUPERBLOCK_OFFSET: usize = 1024;
    const EXT4_SUPER_MAGIC: u16 = 0xef53;
    const EXTENTS_FL: u32 = 0x0008_0000;
    const EXTENT_HEADER_MAGIC: u16 = 0xf30a;
    const INODE_ROOT: u32 = 2;
    const EXT4_NAME_LEN: usize = 255;
    const EXT4_FT_REG_FILE: u8 = 1;
    const EXT4_FT_DIR: u8 = 2;
    const S_IFDIR: u16 = 0o040000;
    const S_IFREG: u16 = 0o100000;

    static mut MOUNTED_EXT4: *mut MountedExt4 = core::ptr::null_mut();

    #[derive(Clone, Debug)]
    pub struct DirEntry {
        pub name: String,
        pub file_type: u8,
        pub inode: u32,
    }

    #[derive(Debug)]
    pub enum TestDiskError {
        NoBlockDevice,
        Block,
        BadSuper,
        Unsupported,
        NotFound,
        Corrupt,
    }

    pub fn smoke() {
        let _ = smoke_and_read_basic_write();
    }

    pub fn smoke_and_read_basic_write() -> Option<Vec<u8>> {
        match try_smoke_and_read_basic_write() {
            Ok(bytes) => bytes,
            Err(TestDiskError::NoBlockDevice) => {
                println!("[testdisk] no block device, skip EXT4 smoke");
                None
            }
            Err(err) => {
                println!("[testdisk] EXT4 smoke failed: {:?}", err);
                None
            }
        }
    }

    pub fn read_file(path: &str) -> Result<Vec<u8>, TestDiskError> {
        let mounted = mounted_ext4()?;
        mounted
            .fs
            .read_file_path(&mut mounted.disk, normalize_path(path))
    }

    pub fn list_dir(path: &str) -> Result<Vec<DirEntry>, TestDiskError> {
        let mounted = mounted_ext4()?;
        mounted
            .fs
            .list_dir_path(&mut mounted.disk, normalize_path(path))
    }

    fn try_smoke_and_read_basic_write() -> Result<Option<Vec<u8>>, TestDiskError> {
        let mounted = mounted_ext4()?;

        println!(
            "[testdisk] EXT4 detected block_size={} inode_size={} inodes/group={} blocks/group={}",
            mounted.fs.block_size,
            mounted.fs.inode_size,
            mounted.fs.inodes_per_group,
            mounted.fs.blocks_per_group
        );

        for path in ["musl", "glibc", "musl/basic", "glibc/basic"] {
            match mounted.fs.lookup_path(&mut mounted.disk, path) {
                Ok(inode) => println!("[testdisk] found /{} inode={}", path, inode),
                Err(_) => println!("[testdisk] missing /{}", path),
            }
        }

        let mut basic_write = None;
        for path in ["musl/basic/write", "musl/basic/brk", "glibc/basic/write"] {
            match mounted.fs.read_file_path(&mut mounted.disk, path) {
                Ok(bytes) => {
                    println!("[testdisk] read /{} size={}", path, bytes.len());
                    if path == "musl/basic/write" {
                        basic_write = Some(bytes);
                    }
                }
                Err(_) => println!("[testdisk] cannot read /{}", path),
            }
        }

        Ok(basic_write)
    }

    fn mounted_ext4() -> Result<&'static mut MountedExt4, TestDiskError> {
        unsafe {
            if MOUNTED_EXT4.is_null() {
                let mut all_devices = axdriver::init_drivers();
                let block = all_devices
                    .block
                    .take_one()
                    .ok_or(TestDiskError::NoBlockDevice)?;
                let mut disk = BlockDisk::new(block)?;
                let fs = Ext4::open(&mut disk)?;
                MOUNTED_EXT4 = Box::leak(Box::new(MountedExt4 { disk, fs }));
            }
            Ok(&mut *MOUNTED_EXT4)
        }
    }

    fn normalize_path(path: &str) -> &str {
        path.trim_start_matches('/')
    }

    struct MountedExt4 {
        disk: BlockDisk,
        fs: Ext4,
    }

    struct BlockDisk {
        dev: axdriver::AxBlockDevice,
        block_size: usize,
        scratch: Vec<u8>,
    }

    impl BlockDisk {
        fn new(dev: axdriver::AxBlockDevice) -> Result<Self, TestDiskError> {
            let block_size = dev.block_size();
            if block_size == 0 {
                return Err(TestDiskError::Block);
            }
            Ok(Self {
                dev,
                block_size,
                scratch: vec![0; block_size],
            })
        }

        fn read_exact(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), TestDiskError> {
            let mut done = 0;
            while done < buf.len() {
                let absolute = offset + done as u64;
                let block_id = absolute / self.block_size as u64;
                let block_off = absolute as usize % self.block_size;
                self.dev
                    .read_block(block_id, &mut self.scratch)
                    .map_err(|_| TestDiskError::Block)?;
                let n = (buf.len() - done).min(self.block_size - block_off);
                buf[done..done + n].copy_from_slice(&self.scratch[block_off..block_off + n]);
                done += n;
            }
            Ok(())
        }
    }

    #[derive(Clone, Copy)]
    struct Ext4 {
        block_size: usize,
        inode_size: usize,
        inodes_per_group: u32,
        blocks_per_group: u32,
        desc_size: usize,
        inode_count: u32,
        groups: u32,
    }

    #[derive(Clone)]
    struct Inode {
        size: u64,
        mode: u16,
        flags: u32,
        block: [u8; 60],
    }

    #[derive(Clone, Copy)]
    struct Extent {
        logical: u32,
        len: u16,
        start: u64,
    }

    impl Ext4 {
        fn open(disk: &mut BlockDisk) -> Result<Self, TestDiskError> {
            let mut sb = [0_u8; 1024];
            disk.read_exact(SUPERBLOCK_OFFSET as u64, &mut sb)?;
            if le16(&sb, 56) != EXT4_SUPER_MAGIC {
                return Err(TestDiskError::BadSuper);
            }

            let log_block_size = le32(&sb, 24);
            if log_block_size > 4 {
                return Err(TestDiskError::Unsupported);
            }
            let block_size = 1024usize
                .checked_shl(log_block_size)
                .ok_or(TestDiskError::Unsupported)?;
            let inode_size = le16(&sb, 88).max(128) as usize;
            let inodes_per_group = le32(&sb, 40);
            let blocks_per_group = le32(&sb, 32);
            let inode_count = le32(&sb, 0);
            let blocks_lo = le32(&sb, 4) as u64;
            let blocks_hi = le32(&sb, 336) as u64;
            let block_count = blocks_lo | (blocks_hi << 32);
            let desc_size = le16(&sb, 254).max(32) as usize;
            if inodes_per_group == 0 || blocks_per_group == 0 {
                return Err(TestDiskError::Corrupt);
            }
            let groups = block_count.div_ceil(blocks_per_group as u64) as u32;

            Ok(Self {
                block_size,
                inode_size,
                inodes_per_group,
                blocks_per_group,
                desc_size,
                inode_count,
                groups,
            })
        }

        fn lookup_path(&self, disk: &mut BlockDisk, path: &str) -> Result<u32, TestDiskError> {
            let mut current = INODE_ROOT;
            for component in path.split('/').filter(|part| !part.is_empty()) {
                current = self.lookup_child(disk, current, component)?;
            }
            Ok(current)
        }

        fn read_file_path(
            &self,
            disk: &mut BlockDisk,
            path: &str,
        ) -> Result<Vec<u8>, TestDiskError> {
            let inode_no = self.lookup_path(disk, path)?;
            let inode = self.read_inode(disk, inode_no)?;
            if inode.mode & S_IFREG != S_IFREG {
                return Err(TestDiskError::Unsupported);
            }
            self.read_inode_data(disk, &inode)
        }

        fn list_dir_path(
            &self,
            disk: &mut BlockDisk,
            path: &str,
        ) -> Result<Vec<DirEntry>, TestDiskError> {
            let inode_no = self.lookup_path(disk, path)?;
            self.list_dir(disk, inode_no)
        }

        fn list_dir(
            &self,
            disk: &mut BlockDisk,
            dir_inode: u32,
        ) -> Result<Vec<DirEntry>, TestDiskError> {
            let inode = self.read_inode(disk, dir_inode)?;
            if inode.mode & S_IFDIR != S_IFDIR {
                return Err(TestDiskError::NotFound);
            }
            let data = self.read_inode_data(disk, &inode)?;
            let mut entries = Vec::new();
            let mut offset = 0;
            while offset + 8 <= data.len() {
                let inode_no = le32(&data, offset);
                let rec_len = le16(&data, offset + 4) as usize;
                let name_len = data[offset + 6] as usize;
                let file_type = data[offset + 7];
                if rec_len < 8 || offset + rec_len > data.len() || name_len > EXT4_NAME_LEN {
                    return Err(TestDiskError::Corrupt);
                }
                if inode_no != 0 && offset + 8 + name_len <= data.len() {
                    let name_bytes = &data[offset + 8..offset + 8 + name_len];
                    if let Ok(name) = core::str::from_utf8(name_bytes) {
                        entries.push(DirEntry {
                            name: String::from(name),
                            file_type: if file_type == 0 {
                                self.inode_file_type(disk, inode_no)?
                            } else {
                                file_type
                            },
                            inode: inode_no,
                        });
                    }
                }
                offset += rec_len;
            }
            Ok(entries)
        }

        fn lookup_child(
            &self,
            disk: &mut BlockDisk,
            dir_inode: u32,
            name: &str,
        ) -> Result<u32, TestDiskError> {
            let inode = self.read_inode(disk, dir_inode)?;
            let data = self.read_inode_data(disk, &inode)?;
            let mut offset = 0;
            while offset + 8 <= data.len() {
                let inode_no = le32(&data, offset);
                let rec_len = le16(&data, offset + 4) as usize;
                let name_len = data[offset + 6] as usize;
                if rec_len < 8 || offset + rec_len > data.len() || name_len > EXT4_NAME_LEN {
                    return Err(TestDiskError::Corrupt);
                }
                if inode_no != 0 && offset + 8 + name_len <= data.len() {
                    let entry_name = &data[offset + 8..offset + 8 + name_len];
                    if entry_name == name.as_bytes() {
                        return Ok(inode_no);
                    }
                }
                offset += rec_len;
            }
            Err(TestDiskError::NotFound)
        }

        fn read_inode(&self, disk: &mut BlockDisk, inode_no: u32) -> Result<Inode, TestDiskError> {
            if inode_no == 0 || inode_no > self.inode_count {
                return Err(TestDiskError::Corrupt);
            }
            let group = (inode_no - 1) / self.inodes_per_group;
            if group >= self.groups {
                return Err(TestDiskError::Corrupt);
            }
            let index = (inode_no - 1) % self.inodes_per_group;
            let inode_table = self.inode_table_block(disk, group)?;
            let offset = inode_table
                .checked_mul(self.block_size as u64)
                .and_then(|base| base.checked_add(index as u64 * self.inode_size as u64))
                .ok_or(TestDiskError::Corrupt)?;
            let mut raw = vec![0_u8; self.inode_size];
            disk.read_exact(offset, &mut raw)?;

            let size = le32(&raw, 4) as u64 | ((le32(&raw, 108) as u64) << 32);
            let mut block = [0_u8; 60];
            block.copy_from_slice(&raw[40..100]);
            Ok(Inode {
                size,
                mode: le16(&raw, 0),
                flags: le32(&raw, 32),
                block,
            })
        }

        fn inode_file_type(
            &self,
            disk: &mut BlockDisk,
            inode_no: u32,
        ) -> Result<u8, TestDiskError> {
            let inode = self.read_inode(disk, inode_no)?;
            if inode.mode & S_IFDIR == S_IFDIR {
                Ok(EXT4_FT_DIR)
            } else if inode.mode & S_IFREG == S_IFREG {
                Ok(EXT4_FT_REG_FILE)
            } else {
                Ok(0)
            }
        }

        fn inode_table_block(
            &self,
            disk: &mut BlockDisk,
            group: u32,
        ) -> Result<u64, TestDiskError> {
            let gdtable_block = if self.block_size == 1024 { 2 } else { 1 };
            let offset = gdtable_block as u64 * self.block_size as u64
                + group as u64 * self.desc_size as u64;
            let mut desc = vec![0_u8; self.desc_size];
            disk.read_exact(offset, &mut desc)?;
            let lo = le32(&desc, 8) as u64;
            let hi = if self.desc_size >= 64 {
                le32(&desc, 40) as u64
            } else {
                0
            };
            Ok(lo | (hi << 32))
        }

        fn read_inode_data(
            &self,
            disk: &mut BlockDisk,
            inode: &Inode,
        ) -> Result<Vec<u8>, TestDiskError> {
            if inode.flags & EXTENTS_FL == 0 {
                return Err(TestDiskError::Unsupported);
            }
            let extents = self.read_extents(disk, &inode.block)?;
            let size = usize::try_from(inode.size).map_err(|_| TestDiskError::Unsupported)?;
            let mut data = vec![0_u8; size];
            for extent in extents {
                let file_off = extent.logical as usize * self.block_size;
                let byte_len = extent.len as usize * self.block_size;
                if file_off >= data.len() {
                    continue;
                }
                let n = byte_len.min(data.len() - file_off);
                let disk_off = extent.start * self.block_size as u64;
                disk.read_exact(disk_off, &mut data[file_off..file_off + n])?;
            }
            Ok(data)
        }

        fn read_extents(
            &self,
            disk: &mut BlockDisk,
            root: &[u8; 60],
        ) -> Result<Vec<Extent>, TestDiskError> {
            self.read_extent_node(disk, root, 0)
        }

        fn read_extent_node(
            &self,
            disk: &mut BlockDisk,
            raw: &[u8],
            level: usize,
        ) -> Result<Vec<Extent>, TestDiskError> {
            if le16(raw, 0) != EXTENT_HEADER_MAGIC {
                return Err(TestDiskError::Unsupported);
            }
            let entries = le16(raw, 2) as usize;
            let depth = le16(raw, 6);
            if entries > (raw.len().saturating_sub(12) / 12) || level > 5 {
                return Err(TestDiskError::Unsupported);
            }

            let mut extents = Vec::new();
            if depth == 0 {
                for index in 0..entries {
                    let offset = 12 + index * 12;
                    let logical = le32(raw, offset);
                    let len = le16(raw, offset + 4) & 0x7fff;
                    let start_hi = le16(raw, offset + 6) as u64;
                    let start_lo = le32(raw, offset + 8) as u64;
                    extents.push(Extent {
                        logical,
                        len,
                        start: (start_hi << 32) | start_lo,
                    });
                }
                return Ok(extents);
            }

            for index in 0..entries {
                let offset = 12 + index * 12;
                let leaf_lo = le32(raw, offset + 4) as u64;
                let leaf_hi = le16(raw, offset + 8) as u64;
                let leaf = (leaf_hi << 32) | leaf_lo;
                let mut child = vec![0_u8; self.block_size];
                disk.read_exact(leaf * self.block_size as u64, &mut child)?;
                extents.extend(self.read_extent_node(disk, &child, level + 1)?);
            }
            Ok(extents)
        }
    }

    fn le16(bytes: &[u8], offset: usize) -> u16 {
        u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
    }

    fn le32(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ])
    }
}

#[cfg(not(feature = "uspace"))]
mod imp {
    extern crate alloc;

    use alloc::vec::Vec;

    pub fn smoke() {}

    pub fn smoke_and_read_basic_write() -> Option<Vec<u8>> {
        None
    }
}

pub use imp::*;
