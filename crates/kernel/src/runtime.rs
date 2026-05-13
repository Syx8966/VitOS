#[cfg(feature = "uspace")]
mod imp {
    extern crate alloc;

    use alloc::{boxed::Box, string::String};
    use axhal::context::{TrapFrame, UspaceContext};
    use axhal::paging::MappingFlags;
    use axhal::trap::{SYSCALL, register_trap_handler};
    use axmm::AddrSpace;
    #[cfg(feature = "axstd")]
    use axstd::{print, println};
    use core::time::Duration;
    use linux_abi::syscall::{
        SYS_BRK, SYS_CHDIR, SYS_CLONE, SYS_CLOSE, SYS_DUP, SYS_DUP3, SYS_EXECVE, SYS_EXIT,
        SYS_FCNTL, SYS_FSTAT, SYS_GETCWD, SYS_GETDENTS64, SYS_GETPID, SYS_GETPPID,
        SYS_GETTIMEOFDAY, SYS_IOCTL, SYS_LSEEK, SYS_MKDIRAT, SYS_MMAP, SYS_MOUNT, SYS_MUNMAP,
        SYS_NANOSLEEP, SYS_OPENAT, SYS_PIPE2, SYS_READ, SYS_SCHED_YIELD, SYS_TIMES, SYS_UMOUNT2,
        SYS_UNAME, SYS_UNLINKAT, SYS_WAIT4, SYS_WRITE,
    };
    use memory_addr::{MemoryAddr, PAGE_SIZE_4K, VirtAddr, va};

    use crate::elf::{self, LoadSegment, ParsedElf};
    use crate::fd::{FdTable, ReadDirResult, linux_dirent64_reclen};

    const USER_ASPACE_BASE: usize = 0x1000;
    const USER_ASPACE_SIZE: usize = 0x0000_8000_0000_0000 - USER_ASPACE_BASE;
    const USER_STACK_TOP: usize = 0x0000_0000_8000_0000;
    const USER_STACK_SIZE: usize = 0x4000;
    const USER_HEAP_LIMIT: usize = USER_STACK_TOP - USER_STACK_SIZE;
    const USER_MMAP_BASE: usize = 0x0000_0000_3000_0000;
    const USER_MMAP_LIMIT: usize = 0x0000_0000_3800_0000;

    static mut ACTIVE_USER_CONTEXT: *mut UserContext = core::ptr::null_mut();

    #[derive(Debug)]
    pub enum RuntimeError {
        Elf(elf::ElfError),
        Map,
        Copy,
        TaskJoin,
    }

    impl From<elf::ElfError> for RuntimeError {
        fn from(err: elf::ElfError) -> Self {
            Self::Elf(err)
        }
    }

    struct UserImage {
        aspace: AddrSpace,
        entry: usize,
        stack_top: usize,
        heap_start: usize,
    }

    struct UserContext {
        image: UserImage,
        fds: FdTable,
        heap_end: usize,
        mmap_cursor: usize,
    }

    pub fn run_embedded_hello() -> Result<i32, RuntimeError> {
        run_elf(
            "embedded-hello",
            elf::embedded_user_hello_for_current_arch(),
        )
    }

    pub fn run_elf(name: &'static str, elf_bytes: &'static [u8]) -> Result<i32, RuntimeError> {
        let task = arceos_api::task::ax_spawn(
            move || match run_elf_current_task(name, elf_bytes) {
                Ok(()) => arceos_api::task::ax_exit(0),
                Err(err) => {
                    #[cfg(feature = "axstd")]
                    println!("[runtime] {} failed in user task: {:?}", name, err);
                    arceos_api::task::ax_exit(127);
                }
            },
            String::from(name),
            0x20000,
        );

        arceos_api::task::ax_wait_for_exit(task).ok_or(RuntimeError::TaskJoin)
    }

    fn run_elf_current_task(name: &str, elf_bytes: &[u8]) -> Result<(), RuntimeError> {
        let parsed = elf::parse(elf_bytes)?;
        let image = load_image(elf_bytes, &parsed)?;
        let heap_start = image.heap_start;
        let context = UserContext {
            image,
            fds: FdTable::new(),
            heap_end: heap_start,
            mmap_cursor: USER_MMAP_BASE,
        };

        #[cfg(feature = "axstd")]
        println!(
            "[runtime] {} ready entry=0x{:x} stack_top=0x{:x} root=0x{:x}",
            name,
            context.image.entry,
            context.image.stack_top,
            context.image.aspace.page_table_root().as_usize()
        );

        enter_user(Box::new(context))
    }

    fn load_image(elf_bytes: &[u8], parsed: &ParsedElf) -> Result<UserImage, RuntimeError> {
        let mut aspace = axmm::new_user_aspace(va!(USER_ASPACE_BASE), USER_ASPACE_SIZE)
            .map_err(|_| RuntimeError::Map)?;

        let mut heap_start = 0;
        for segment in parsed.load_segments() {
            map_load_segment(&mut aspace, elf_bytes, segment)?;
            let segment_end = usize::try_from(segment.vaddr.saturating_add(segment.memsz))
                .map_err(|_| RuntimeError::Map)?;
            heap_start = heap_start.max(align_up(segment_end, PAGE_SIZE_4K));
        }

        let stack_bottom = va!(USER_STACK_TOP - USER_STACK_SIZE);
        aspace
            .map_alloc(
                stack_bottom,
                USER_STACK_SIZE,
                MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
                true,
            )
            .map_err(|_| RuntimeError::Map)?;

        Ok(UserImage {
            aspace,
            entry: parsed.header.entry as usize,
            stack_top: USER_STACK_TOP,
            heap_start,
        })
    }

    fn map_load_segment(
        aspace: &mut AddrSpace,
        elf_bytes: &[u8],
        segment: LoadSegment,
    ) -> Result<(), RuntimeError> {
        let vaddr = va!(segment.vaddr as usize);
        let map_start = vaddr.align_down_4k();
        let map_end = (vaddr + segment.memsz as usize).align_up_4k();
        let map_size = map_end - map_start;
        let page_offset = vaddr.as_usize() - map_start.as_usize();

        let mut flags = MappingFlags::USER;
        if segment.flags & 0x1 != 0 {
            flags |= MappingFlags::EXECUTE;
        }
        if segment.flags & 0x2 != 0 {
            flags |= MappingFlags::WRITE;
        }
        if segment.flags & 0x4 != 0 {
            flags |= MappingFlags::READ;
        }

        aspace
            .map_alloc(map_start, map_size, flags, true)
            .map_err(|_| RuntimeError::Map)?;

        let file_start = segment.offset as usize;
        let file_end = file_start
            .checked_add(segment.filesz as usize)
            .ok_or(RuntimeError::Copy)?;
        let data = elf_bytes
            .get(file_start..file_end)
            .ok_or(RuntimeError::Copy)?;
        aspace
            .write(map_start + page_offset, data)
            .map_err(|_| RuntimeError::Copy)?;

        Ok(())
    }

    fn enter_user(context: Box<UserContext>) -> Result<(), RuntimeError> {
        let context = Box::leak(context);
        unsafe {
            ACTIVE_USER_CONTEXT = context;
            axhal::asm::write_user_page_table(context.image.aspace.page_table_root());
            axhal::asm::flush_tlb(None);
        }

        let uspace_context =
            UspaceContext::new(context.image.entry, va!(context.image.stack_top), 0);
        let kstack_top = current_kernel_stack_top();

        #[cfg(feature = "axstd")]
        println!("[runtime] enter user");

        unsafe {
            uspace_context.enter_uspace(kstack_top);
        }
    }

    fn current_kernel_stack_top() -> VirtAddr {
        let local = 0usize;
        va!((&local as *const usize as usize).align_up_4k())
    }

    #[register_trap_handler(SYSCALL)]
    fn handle_syscall(tf: &TrapFrame, nr: usize) -> isize {
        match nr {
            SYS_READ => sys_read(tf.arg0(), tf.arg1(), tf.arg2()),
            SYS_WRITE => sys_write(tf.arg0(), tf.arg1(), tf.arg2()),
            SYS_CLOSE => sys_close(tf.arg0()),
            SYS_FSTAT => sys_fstat(tf.arg0(), tf.arg1()),
            SYS_LSEEK => sys_lseek(tf.arg0(), tf.arg1() as isize, tf.arg2()),
            SYS_GETDENTS64 => sys_getdents64(tf.arg0(), tf.arg1(), tf.arg2()),
            SYS_OPENAT => sys_openat(tf.arg0() as isize, tf.arg1(), tf.arg2(), tf.arg3()),
            SYS_EXIT => sys_exit(tf.arg0()),
            SYS_BRK => sys_brk(tf.arg0()),
            SYS_MMAP => sys_mmap(
                tf.arg0(),
                tf.arg1(),
                tf.arg2(),
                tf.arg3(),
                tf.arg4(),
                tf.arg5(),
            ),
            SYS_MUNMAP => sys_munmap(tf.arg0(), tf.arg1()),
            SYS_GETTIMEOFDAY => sys_gettimeofday(tf.arg0()),
            SYS_NANOSLEEP => sys_nanosleep(tf.arg0()),
            SYS_TIMES => sys_times(tf.arg0()),
            SYS_UNAME => sys_uname(tf.arg0()),
            SYS_GETPID => 1,
            SYS_GETPPID => 0,
            SYS_SCHED_YIELD => {
                arceos_api::task::ax_yield_now();
                0
            }
            SYS_MOUNT | SYS_UMOUNT2 => 0,
            SYS_DUP | SYS_DUP3 | SYS_FCNTL | SYS_IOCTL | SYS_GETCWD | SYS_CHDIR | SYS_MKDIRAT
            | SYS_UNLINKAT | SYS_PIPE2 | SYS_CLONE | SYS_EXECVE | SYS_WAIT4 => {
                #[cfg(feature = "axstd")]
                println!("[syscall] unsupported pending-fs/proc nr={}", nr);
                -38
            }
            _ => {
                #[cfg(feature = "axstd")]
                println!("[syscall] unsupported nr={}", nr);
                -38
            }
        }
    }

    fn sys_read(fd: usize, buf: usize, len: usize) -> isize {
        let context = match active_user_context_mut() {
            Ok(context) => context,
            Err(err) => return err,
        };
        let mut chunk = [0_u8; 256];
        let mut total = 0;
        while total < len {
            let n = (len - total).min(chunk.len());
            match context.fds.read(fd, &mut chunk[..n]) {
                Ok(0) if total == 0 => return 0,
                Ok(0) => break,
                Ok(read) => {
                    if write_user(buf + total, &chunk[..read]).is_err() {
                        return -14;
                    }
                    total += read;
                }
                Err(err) => return err,
            }
        }
        total as isize
    }

    fn sys_write(fd: usize, buf: usize, len: usize) -> isize {
        if fd != 1 && fd != 2 {
            return -9;
        }

        let mut chunk = [0_u8; 128];
        let mut written = 0;
        while written < len {
            let n = (len - written).min(chunk.len());
            if read_user(buf + written, &mut chunk[..n]).is_err() {
                return -14;
            }
            #[cfg(feature = "axstd")]
            if let Ok(s) = core::str::from_utf8(&chunk[..n]) {
                print!("{}", s);
            }
            #[cfg(feature = "axstd")]
            if core::str::from_utf8(&chunk[..n]).is_err() {
                for byte in &chunk[..n] {
                    print!("{}", *byte as char);
                }
            }
            written += n;
        }

        len as isize
    }

    fn sys_close(fd: usize) -> isize {
        match active_user_context_mut() {
            Ok(context) => context.fds.close(fd),
            Err(err) => err,
        }
    }

    fn sys_fstat(fd: usize, stat_buf: usize) -> isize {
        let context = match active_user_context_mut() {
            Ok(context) => context,
            Err(err) => return err,
        };
        let stat = match context.fds.stat(fd) {
            Ok(stat) => stat,
            Err(err) => return err,
        };
        let data = linux_stat_bytes(stat.mode, stat.size, stat.inode);
        if write_user(stat_buf, &data).is_err() {
            return -14;
        }
        0
    }

    fn sys_lseek(fd: usize, offset: isize, whence: usize) -> isize {
        match active_user_context_mut() {
            Ok(context) => context.fds.seek(fd, offset, whence),
            Err(err) => err,
        }
    }

    fn sys_openat(dirfd: isize, path_ptr: usize, flags: usize, _mode: usize) -> isize {
        let path = match read_user_cstr(path_ptr) {
            Ok(path) => path,
            Err(err) => return err,
        };
        match active_user_context_mut() {
            Ok(context) => match context.fds.openat(dirfd, &path, flags) {
                Ok(fd) => fd as isize,
                Err(err) => err,
            },
            Err(err) => err,
        }
    }

    fn sys_getdents64(fd: usize, dirp: usize, len: usize) -> isize {
        let context = match active_user_context_mut() {
            Ok(context) => context,
            Err(err) => return err,
        };
        let mut written = 0;
        loop {
            match context.fds.next_dirent(fd, len - written) {
                Ok(ReadDirResult::Entry {
                    inode,
                    offset,
                    file_type,
                    name,
                }) => {
                    let reclen = linux_dirent64_reclen(name.len());
                    let mut dirent = [0_u8; 280];
                    if reclen > dirent.len() {
                        return -22;
                    }
                    dirent[0..8].copy_from_slice(&inode.to_ne_bytes());
                    dirent[8..16].copy_from_slice(&offset.to_ne_bytes());
                    dirent[16..18].copy_from_slice(&(reclen as u16).to_ne_bytes());
                    dirent[18] = file_type;
                    dirent[19..19 + name.len()].copy_from_slice(name.as_bytes());
                    if write_user(dirp + written, &dirent[..reclen]).is_err() {
                        return -14;
                    }
                    written += reclen;
                }
                Ok(ReadDirResult::End) => return written as isize,
                Ok(ReadDirResult::BufferTooSmall) => {
                    return if written == 0 { -22 } else { written as isize };
                }
                Err(err) => return err,
            }
        }
    }

    fn sys_exit(code: usize) -> isize {
        #[cfg(feature = "axstd")]
        println!("[syscall] exit({})", code);
        arceos_api::task::ax_exit(code as i32);
    }

    fn sys_brk(addr: usize) -> isize {
        let context = match active_user_context_mut() {
            Ok(context) => context,
            Err(err) => return err,
        };
        if addr == 0 {
            return context.heap_end as isize;
        }
        if addr < context.image.heap_start || addr > USER_HEAP_LIMIT {
            return context.heap_end as isize;
        }

        let old_end = context.heap_end;
        if addr > old_end {
            let map_start = va!(old_end).align_up_4k();
            let map_end = va!(addr).align_up_4k();
            if map_end > map_start
                && context
                    .image
                    .aspace
                    .map_alloc(
                        map_start,
                        map_end - map_start,
                        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
                        true,
                    )
                    .is_err()
            {
                return old_end as isize;
            }
        }

        context.heap_end = addr;
        addr as isize
    }

    fn sys_mmap(
        addr: usize,
        len: usize,
        prot: usize,
        flags: usize,
        fd: usize,
        _offset: usize,
    ) -> isize {
        const MAP_FIXED: usize = 0x10;
        const MAP_ANONYMOUS: usize = 0x20;

        if len == 0 {
            return -22;
        }
        if flags & MAP_ANONYMOUS == 0 && fd > 2 {
            return -9;
        }

        let context = match active_user_context_mut() {
            Ok(context) => context,
            Err(err) => return err,
        };
        let size = align_up(len, PAGE_SIZE_4K);
        let start = if flags & MAP_FIXED != 0 {
            align_down(addr, PAGE_SIZE_4K)
        } else {
            let next = align_up(context.mmap_cursor, PAGE_SIZE_4K);
            context.mmap_cursor = next.saturating_add(size);
            next
        };

        if start < USER_MMAP_BASE || start.saturating_add(size) > USER_MMAP_LIMIT {
            return -12;
        }

        let mut map_flags = MappingFlags::USER;
        if prot & 0x1 != 0 {
            map_flags |= MappingFlags::READ;
        }
        if prot & 0x2 != 0 {
            map_flags |= MappingFlags::WRITE;
        }
        if prot & 0x4 != 0 {
            map_flags |= MappingFlags::EXECUTE;
        }
        if map_flags == MappingFlags::USER {
            map_flags |= MappingFlags::READ;
        }

        match context
            .image
            .aspace
            .map_alloc(va!(start), size, map_flags, true)
        {
            Ok(()) => start as isize,
            Err(_) => -12,
        }
    }

    fn sys_munmap(addr: usize, len: usize) -> isize {
        if len == 0 {
            return -22;
        }
        let context = match active_user_context_mut() {
            Ok(context) => context,
            Err(err) => return err,
        };
        let start = va!(align_down(addr, PAGE_SIZE_4K));
        let size = align_up(len + (addr - start.as_usize()), PAGE_SIZE_4K);
        match context.image.aspace.unmap(start, size) {
            Ok(()) => 0,
            Err(_) => -22,
        }
    }

    fn sys_gettimeofday(tv: usize) -> isize {
        if tv == 0 {
            return 0;
        }
        let now = axhal::time::wall_time();
        let mut data = [0_u8; 16];
        data[..8].copy_from_slice(&(now.as_secs() as i64).to_ne_bytes());
        data[8..16].copy_from_slice(&(now.subsec_micros() as i64).to_ne_bytes());
        write_user(tv, &data).map_or(-14, |_| 0)
    }

    fn sys_nanosleep(req: usize) -> isize {
        let mut data = [0_u8; 16];
        if read_user(req, &mut data).is_err() {
            return -14;
        }
        let sec = i64::from_ne_bytes(data[..8].try_into().unwrap());
        let nsec = i64::from_ne_bytes(data[8..16].try_into().unwrap());
        if sec < 0 || !(0..1_000_000_000).contains(&nsec) {
            return -22;
        }
        arceos_api::task::ax_sleep_until(
            axhal::time::wall_time() + Duration::new(sec as u64, u32::try_from(nsec).unwrap_or(0)),
        );
        0
    }

    fn sys_times(buf: usize) -> isize {
        if buf != 0 {
            let data = [0_u8; 32];
            if write_user(buf, &data).is_err() {
                return -14;
            }
        }
        let millis = axhal::time::monotonic_time().as_millis();
        millis.min(isize::MAX as u128) as isize
    }

    fn sys_uname(buf: usize) -> isize {
        let mut data = [0_u8; 65 * 6];
        write_cstr_field(&mut data, 0, "Linux");
        write_cstr_field(&mut data, 65, "vitos");
        write_cstr_field(&mut data, 65 * 2, "0.1.0");
        write_cstr_field(&mut data, 65 * 3, "0.1.0");
        write_cstr_field(&mut data, 65 * 4, arch_name());
        write_cstr_field(&mut data, 65 * 5, "vitos");
        write_user(buf, &data).map_or(-14, |_| 0)
    }

    fn read_user(start: usize, buf: &mut [u8]) -> Result<(), ()> {
        let context = unsafe { ACTIVE_USER_CONTEXT.as_ref().ok_or(())? };
        if !context.image.aspace.can_access_range(
            va!(start),
            buf.len(),
            MappingFlags::READ | MappingFlags::USER,
        ) {
            return Err(());
        }
        with_user_memory_access(|| unsafe {
            core::ptr::copy_nonoverlapping(start as *const u8, buf.as_mut_ptr(), buf.len());
        });
        Ok(())
    }

    fn read_user_cstr(start: usize) -> Result<String, isize> {
        let mut bytes = alloc::vec::Vec::new();
        for offset in 0..4096 {
            let mut byte = [0_u8; 1];
            if read_user(start + offset, &mut byte).is_err() {
                return Err(-14);
            }
            if byte[0] == 0 {
                return core::str::from_utf8(&bytes)
                    .map(String::from)
                    .map_err(|_| -22);
            }
            bytes.push(byte[0]);
        }
        Err(-36)
    }

    fn write_user(start: usize, buf: &[u8]) -> Result<(), ()> {
        let context = unsafe { ACTIVE_USER_CONTEXT.as_ref().ok_or(())? };
        if !context.image.aspace.can_access_range(
            va!(start),
            buf.len(),
            MappingFlags::WRITE | MappingFlags::USER,
        ) {
            return Err(());
        }
        with_user_memory_access(|| unsafe {
            core::ptr::copy_nonoverlapping(buf.as_ptr(), start as *mut u8, buf.len());
        });
        Ok(())
    }

    fn active_user_context_mut() -> Result<&'static mut UserContext, isize> {
        unsafe { ACTIVE_USER_CONTEXT.as_mut().ok_or(-14) }
    }

    fn align_down(value: usize, align: usize) -> usize {
        value & !(align - 1)
    }

    fn align_up(value: usize, align: usize) -> usize {
        value.saturating_add(align - 1) & !(align - 1)
    }

    fn write_cstr_field(buf: &mut [u8], offset: usize, value: &str) {
        let bytes = value.as_bytes();
        let len = bytes.len().min(64);
        buf[offset..offset + len].copy_from_slice(&bytes[..len]);
    }

    fn linux_stat_bytes(mode: u32, size: u64, inode: u64) -> [u8; 128] {
        let mut stat = [0_u8; 128];
        stat[0..8].copy_from_slice(&0_u64.to_ne_bytes());
        stat[8..16].copy_from_slice(&inode.to_ne_bytes());
        stat[16..20].copy_from_slice(&mode.to_ne_bytes());
        stat[24..32].copy_from_slice(&1_u64.to_ne_bytes());
        stat[32..36].copy_from_slice(&0_u32.to_ne_bytes());
        stat[36..40].copy_from_slice(&0_u32.to_ne_bytes());
        stat[48..56].copy_from_slice(&size.to_ne_bytes());
        stat[56..64].copy_from_slice(&4096_i64.to_ne_bytes());
        stat[64..72].copy_from_slice(&0_i64.to_ne_bytes());
        stat
    }

    fn arch_name() -> &'static str {
        match option_env!("VITOS_BOOT_ARCH") {
            Some("loongarch64") => "loongarch64",
            _ => "riscv64",
        }
    }

    #[cfg(target_arch = "riscv64")]
    fn with_user_memory_access<T>(f: impl FnOnce() -> T) -> T {
        const SSTATUS_SUM: usize = 1 << 18;
        let saved: usize;
        unsafe {
            core::arch::asm!("csrr {0}, sstatus", out(reg) saved);
            core::arch::asm!("csrs sstatus, {0}", in(reg) SSTATUS_SUM);
        }
        let result = f();
        unsafe {
            core::arch::asm!("csrw sstatus, {0}", in(reg) saved);
        }
        result
    }

    #[cfg(not(target_arch = "riscv64"))]
    fn with_user_memory_access<T>(f: impl FnOnce() -> T) -> T {
        f()
    }
}

#[cfg(not(feature = "uspace"))]
mod imp {
    #[derive(Debug)]
    pub enum RuntimeError {
        Disabled,
    }

    pub fn run_embedded_hello() -> Result<i32, RuntimeError> {
        Err(RuntimeError::Disabled)
    }
}

pub use imp::*;
