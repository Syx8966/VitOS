#![cfg(feature = "uspace")]

extern crate alloc;

use alloc::{string::String, vec::Vec};

use crate::testdisk::{self, DirEntry};

const MAX_FDS: usize = 32;
const AT_FDCWD: isize = -100;
const SEEK_SET: usize = 0;
const SEEK_CUR: usize = 1;
const SEEK_END: usize = 2;

const S_IFCHR: u32 = 0o020000;
const S_IFDIR: u32 = 0o040000;
const S_IFREG: u32 = 0o100000;
const DT_REG: u8 = 8;
const DT_DIR: u8 = 4;

#[derive(Clone, Copy)]
pub struct FileStat {
    pub mode: u32,
    pub size: u64,
    pub inode: u64,
}

pub struct FdTable {
    entries: [Option<FdEntry>; MAX_FDS],
}

enum FdEntry {
    Console,
    File {
        data: Vec<u8>,
        offset: usize,
        inode: u64,
    },
    Directory {
        entries: Vec<DirEntry>,
        offset: usize,
        inode: u64,
    },
}

pub enum ReadDirResult {
    Entry {
        inode: u64,
        offset: u64,
        file_type: u8,
        name: String,
    },
    End,
    BufferTooSmall,
}

impl FdTable {
    pub const fn new() -> Self {
        Self {
            entries: [
                Some(FdEntry::Console),
                Some(FdEntry::Console),
                Some(FdEntry::Console),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ],
        }
    }

    pub fn openat(&mut self, dirfd: isize, path: &str, _flags: usize) -> Result<usize, isize> {
        if dirfd != AT_FDCWD && !path.starts_with('/') {
            return Err(-22);
        }
        if let Ok(entries) = testdisk::list_dir(path) {
            return self.insert(FdEntry::Directory {
                entries,
                offset: 0,
                inode: 0,
            });
        }
        match testdisk::read_file(path) {
            Ok(data) => self.insert(FdEntry::File {
                inode: 0,
                data,
                offset: 0,
            }),
            Err(testdisk::TestDiskError::NotFound) => Err(-2),
            Err(testdisk::TestDiskError::NoBlockDevice) => Err(-19),
            Err(_) => Err(-5),
        }
    }

    pub fn close(&mut self, fd: usize) -> isize {
        if fd <= 2 {
            return 0;
        }
        match self.entries.get_mut(fd) {
            Some(slot @ Some(_)) => {
                *slot = None;
                0
            }
            _ => -9,
        }
    }

    pub fn read(&mut self, fd: usize, buf: &mut [u8]) -> Result<usize, isize> {
        match self.entries.get_mut(fd).and_then(Option::as_mut) {
            Some(FdEntry::Console) if fd == 0 => Ok(0),
            Some(FdEntry::File { data, offset, .. }) => {
                let n = buf.len().min(data.len().saturating_sub(*offset));
                buf[..n].copy_from_slice(&data[*offset..*offset + n]);
                *offset += n;
                Ok(n)
            }
            Some(_) => Err(-21),
            None => Err(-9),
        }
    }

    pub fn seek(&mut self, fd: usize, offset: isize, whence: usize) -> isize {
        let Some(entry) = self.entries.get_mut(fd).and_then(Option::as_mut) else {
            return -9;
        };
        let (current, len) = match entry {
            FdEntry::File { data, offset, .. } => (*offset, data.len()),
            FdEntry::Directory {
                entries, offset, ..
            } => (*offset, entries.len()),
            FdEntry::Console => return -29,
        };
        let base = match whence {
            SEEK_SET => 0,
            SEEK_CUR => current as isize,
            SEEK_END => len as isize,
            _ => return -22,
        };
        let Some(next) = base.checked_add(offset) else {
            return -22;
        };
        if next < 0 {
            return -22;
        }
        let next = next as usize;
        match entry {
            FdEntry::File { offset, .. } | FdEntry::Directory { offset, .. } => *offset = next,
            FdEntry::Console => {}
        }
        next as isize
    }

    pub fn stat(&self, fd: usize) -> Result<FileStat, isize> {
        match self.entries.get(fd).and_then(Option::as_ref) {
            Some(FdEntry::Console) => Ok(FileStat {
                mode: S_IFCHR | 0o666,
                size: 0,
                inode: fd as u64 + 1,
            }),
            Some(FdEntry::File { data, inode, .. }) => Ok(FileStat {
                mode: S_IFREG | 0o555,
                size: data.len() as u64,
                inode: *inode,
            }),
            Some(FdEntry::Directory { inode, .. }) => Ok(FileStat {
                mode: S_IFDIR | 0o555,
                size: 0,
                inode: *inode,
            }),
            None => Err(-9),
        }
    }

    pub fn next_dirent(&mut self, fd: usize, available: usize) -> Result<ReadDirResult, isize> {
        let Some(FdEntry::Directory {
            entries, offset, ..
        }) = self.entries.get_mut(fd).and_then(Option::as_mut)
        else {
            return Err(-20);
        };
        if *offset >= entries.len() {
            return Ok(ReadDirResult::End);
        }
        let entry = &entries[*offset];
        let reclen = linux_dirent64_reclen(entry.name.len());
        if available < reclen {
            return Ok(ReadDirResult::BufferTooSmall);
        }
        *offset += 1;
        let file_type = match entry.file_type {
            1 => DT_REG,
            2 => DT_DIR,
            value => value,
        };
        Ok(ReadDirResult::Entry {
            inode: entry.inode as u64,
            offset: *offset as u64,
            file_type,
            name: entry.name.clone(),
        })
    }

    fn insert(&mut self, entry: FdEntry) -> Result<usize, isize> {
        for fd in 3..MAX_FDS {
            if self.entries[fd].is_none() {
                self.entries[fd] = Some(entry);
                return Ok(fd);
            }
        }
        Err(-24)
    }
}

pub fn linux_dirent64_reclen(name_len: usize) -> usize {
    align_up(19 + name_len + 1, 8)
}

fn align_up(value: usize, align: usize) -> usize {
    value.saturating_add(align - 1) & !(align - 1)
}
