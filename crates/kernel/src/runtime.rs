#[cfg(feature = "uspace")]
mod imp {
    extern crate alloc;

    use alloc::{boxed::Box, collections::VecDeque, string::String, vec::Vec};
    use axhal::context::{TrapFrame, UspaceContext};
    use axhal::paging::MappingFlags;
    use axhal::trap::{SYSCALL, register_trap_handler};
    use axmm::AddrSpace;
    #[cfg(feature = "axstd")]
    use axstd::{print, println};
    use core::time::Duration;
    use linux_abi::syscall::{
        SYS_BRK, SYS_CHDIR, SYS_CLONE, SYS_CLOSE, SYS_DUP, SYS_DUP3, SYS_EXECVE, SYS_EXIT,
        SYS_EXIT_GROUP, SYS_FACCESSAT, SYS_FCNTL, SYS_FSTAT, SYS_FSTATFS, SYS_GETCWD,
        SYS_GETDENTS64, SYS_GETPID, SYS_GETPPID, SYS_GETTID, SYS_GETTIMEOFDAY, SYS_IOCTL,
        SYS_LSEEK, SYS_MKDIRAT, SYS_MMAP, SYS_MOUNT, SYS_MUNMAP, SYS_NANOSLEEP, SYS_NEWFSTATAT,
        SYS_OPENAT, SYS_PIPE2, SYS_READ, SYS_READLINKAT, SYS_SCHED_YIELD, SYS_SET_TID_ADDRESS,
        SYS_STATFS, SYS_TGKILL, SYS_TIMES, SYS_UMOUNT2, SYS_UNAME, SYS_UNLINKAT, SYS_WAIT4,
        SYS_WRITE,
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
    const SIGCHLD: usize = 17;

    const MAX_USER_CONTEXTS: usize = 32;

    static mut ACTIVE_USER_CONTEXTS: [UserContextSlot; MAX_USER_CONTEXTS] =
        [UserContextSlot::empty(); MAX_USER_CONTEXTS];
    static mut NEXT_PID: usize = 2;

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
        mappings: Vec<UserMapping>,
    }

    #[derive(Clone, Copy)]
    struct UserMapping {
        start: usize,
        size: usize,
        flags: MappingFlags,
    }

    struct UserContext {
        image: UserImage,
        fds: FdTable,
        pid: usize,
        ppid: usize,
        cwd: String,
        exe_path: String,
        children: VecDeque<ChildProcess>,
        clear_tid_address: usize,
        heap_end: usize,
        mmap_cursor: usize,
        resume_tf: Option<TrapFrame>,
    }

    struct ChildProcess {
        pid: usize,
        handle: Option<arceos_api::task::AxTaskHandle>,
        exited_code: Option<i32>,
    }

    #[derive(Clone, Copy)]
    struct UserContextSlot {
        task_id: u64,
        context: *mut UserContext,
    }

    impl UserContextSlot {
        const fn empty() -> Self {
            Self {
                task_id: 0,
                context: core::ptr::null_mut(),
            }
        }
    }

    impl UserContext {
        fn fork_for_child(
            &self,
            child_pid: usize,
            parent_tf: &TrapFrame,
            child_stack: usize,
        ) -> Result<Self, isize> {
            let image = self.image.try_clone()?;
            Ok(Self {
                image,
                fds: self.fds.clone(),
                pid: child_pid,
                ppid: self.pid,
                cwd: self.cwd.clone(),
                exe_path: self.exe_path.clone(),
                children: VecDeque::new(),
                clear_tid_address: 0,
                heap_end: self.heap_end,
                mmap_cursor: self.mmap_cursor,
                resume_tf: Some(child_trap_frame(parent_tf, child_stack)),
            })
        }
    }

    impl UserImage {
        fn try_clone(&self) -> Result<Self, isize> {
            let mut aspace = axmm::new_user_aspace(va!(USER_ASPACE_BASE), USER_ASPACE_SIZE)
                .map_err(|_| -12_isize)?;
            for mapping in &self.mappings {
                aspace
                    .map_alloc(va!(mapping.start), mapping.size, mapping.flags, true)
                    .map_err(|_| -12_isize)?;
                copy_user_range(&self.aspace, &aspace, mapping.start, mapping.size)
                    .map_err(|_| -14_isize)?;
            }
            let stack_top = self.stack_top;
            Ok(Self {
                aspace,
                entry: self.entry,
                stack_top,
                heap_start: self.heap_start,
                mappings: self.mappings.clone(),
            })
        }
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
            pid: 1,
            ppid: 0,
            cwd: String::from("/"),
            exe_path: String::from(name),
            children: VecDeque::new(),
            clear_tid_address: 0,
            heap_end: heap_start,
            mmap_cursor: USER_MMAP_BASE,
            resume_tf: None,
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
        let mut mappings = Vec::new();
        for segment in parsed.load_segments() {
            let mapping = map_load_segment(&mut aspace, elf_bytes, segment)?;
            heap_start = heap_start.max(mapping.start + mapping.size);
            mappings_push_or_extend(&mut mappings, mapping);
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
        mappings.push(UserMapping {
            start: stack_bottom.as_usize(),
            size: USER_STACK_SIZE,
            flags: MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
        });

        Ok(UserImage {
            aspace,
            entry: parsed.header.entry as usize,
            stack_top: USER_STACK_TOP,
            heap_start,
            mappings,
        })
    }

    fn map_load_segment(
        aspace: &mut AddrSpace,
        elf_bytes: &[u8],
        segment: LoadSegment,
    ) -> Result<UserMapping, RuntimeError> {
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

        Ok(UserMapping {
            start: map_start.as_usize(),
            size: map_size,
            flags,
        })
    }

    fn enter_user(context: Box<UserContext>) -> Result<(), RuntimeError> {
        let context = Box::leak(context);
        if !register_user_context(arceos_api::task::ax_current_task_id(), context) {
            return Err(RuntimeError::Map);
        }
        unsafe {
            axhal::asm::write_user_page_table(context.image.aspace.page_table_root());
            axhal::asm::flush_tlb(None);
        }

        let uspace_context = if let Some(tf) = context.resume_tf.as_ref() {
            UspaceContext::from(tf)
        } else {
            UspaceContext::new(context.image.entry, va!(context.image.stack_top), 0)
        };
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
        activate_current_user_aspace();
        let ret = match nr {
            SYS_READ => sys_read(tf.arg0(), tf.arg1(), tf.arg2()),
            SYS_WRITE => sys_write(tf.arg0(), tf.arg1(), tf.arg2()),
            SYS_CLOSE => sys_close(tf.arg0()),
            SYS_FSTAT => sys_fstat(tf.arg0(), tf.arg1()),
            SYS_NEWFSTATAT => sys_newfstatat(tf.arg0() as isize, tf.arg1(), tf.arg2(), tf.arg3()),
            SYS_READLINKAT => sys_readlinkat(tf.arg0() as isize, tf.arg1(), tf.arg2(), tf.arg3()),
            SYS_FACCESSAT => sys_faccessat(tf.arg0() as isize, tf.arg1(), tf.arg2(), tf.arg3()),
            SYS_STATFS => sys_statfs(tf.arg0(), tf.arg1()),
            SYS_FSTATFS => sys_fstatfs(tf.arg0(), tf.arg1()),
            SYS_LSEEK => sys_lseek(tf.arg0(), tf.arg1() as isize, tf.arg2()),
            SYS_GETDENTS64 => sys_getdents64(tf.arg0(), tf.arg1(), tf.arg2()),
            SYS_OPENAT => sys_openat(tf.arg0() as isize, tf.arg1(), tf.arg2(), tf.arg3()),
            SYS_DUP => sys_dup(tf.arg0()),
            SYS_DUP3 => sys_dup3(tf.arg0(), tf.arg1(), tf.arg2()),
            SYS_FCNTL => sys_fcntl(tf.arg0(), tf.arg1(), tf.arg2()),
            SYS_IOCTL => sys_ioctl(tf.arg0(), tf.arg1(), tf.arg2()),
            SYS_GETCWD => sys_getcwd(tf.arg0(), tf.arg1()),
            SYS_CHDIR => sys_chdir(tf.arg0()),
            SYS_EXIT | SYS_EXIT_GROUP => sys_exit(tf.arg0()),
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
            SYS_GETPID => active_user_context_mut().map_or(-14, |context| context.pid as isize),
            SYS_GETPPID => active_user_context_mut().map_or(-14, |context| context.ppid as isize),
            SYS_GETTID => active_user_context_mut().map_or(-14, |context| context.pid as isize),
            SYS_SET_TID_ADDRESS => sys_set_tid_address(tf.arg0()),
            SYS_SCHED_YIELD => {
                arceos_api::task::ax_yield_now();
                0
            }
            SYS_MOUNT | SYS_UMOUNT2 => 0,
            SYS_CLONE => sys_clone(tf, tf.arg0(), tf.arg1(), tf.arg2(), tf.arg3(), tf.arg4()),
            SYS_EXECVE => sys_execve(tf.arg0()),
            SYS_WAIT4 => sys_wait4(tf.arg0() as isize, tf.arg1(), tf.arg2()),
            SYS_TGKILL => sys_tgkill(tf.arg0(), tf.arg1(), tf.arg2()),
            SYS_MKDIRAT | SYS_UNLINKAT | SYS_PIPE2 => {
                #[cfg(feature = "axstd")]
                println!("[syscall] unsupported pending-fs nr={}", nr);
                -38
            }
            _ => {
                #[cfg(feature = "axstd")]
                println!("[syscall] unsupported nr={}", nr);
                -38
            }
        };
        activate_current_user_aspace();
        ret
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

    fn sys_newfstatat(dirfd: isize, path_ptr: usize, stat_buf: usize, _flags: usize) -> isize {
        let path = match read_user_cstr(path_ptr) {
            Ok(path) => path,
            Err(err) => return err,
        };
        let context = match active_user_context_mut() {
            Ok(context) => context,
            Err(err) => return err,
        };
        let resolved = match resolve_user_path(context, dirfd, &path) {
            Ok(path) => path,
            Err(err) => return err,
        };
        let stat = match context.fds.stat_path(&resolved) {
            Ok(stat) => stat,
            Err(err) => return err,
        };
        let data = linux_stat_bytes(stat.mode, stat.size, stat.inode);
        if write_user(stat_buf, &data).is_err() {
            return -14;
        }
        0
    }

    fn sys_readlinkat(_dirfd: isize, path_ptr: usize, buf: usize, len: usize) -> isize {
        let path = match read_user_cstr(path_ptr) {
            Ok(path) => path,
            Err(err) => return err,
        };
        let context = match active_user_context_mut() {
            Ok(context) => context,
            Err(err) => return err,
        };
        let target = if path == "/proc/self/exe" {
            context.exe_path.as_str()
        } else if path == "/" {
            "/"
        } else {
            return -2;
        };
        let bytes = target.as_bytes();
        let n = bytes.len().min(len);
        if write_user(buf, &bytes[..n]).is_err() {
            return -14;
        }
        n as isize
    }

    fn sys_faccessat(_dirfd: isize, path_ptr: usize, _mode: usize, _flags: usize) -> isize {
        let path = match read_user_cstr(path_ptr) {
            Ok(path) => path,
            Err(err) => return err,
        };
        let context = match active_user_context_mut() {
            Ok(context) => context,
            Err(err) => return err,
        };
        let resolved = match resolve_user_path(context, -100, &path) {
            Ok(path) => path,
            Err(err) => return err,
        };
        match context.fds.stat_path(&resolved) {
            Ok(_) => 0,
            Err(err) => err,
        }
    }

    fn sys_statfs(path_ptr: usize, buf: usize) -> isize {
        let path = match read_user_cstr(path_ptr) {
            Ok(path) => path,
            Err(err) => return err,
        };
        let context = match active_user_context_mut() {
            Ok(context) => context,
            Err(err) => return err,
        };
        let resolved = match resolve_user_path(context, -100, &path) {
            Ok(path) => path,
            Err(err) => return err,
        };
        let stat = match context.fds.stat_path(&resolved) {
            Ok(stat) => stat,
            Err(err) => return err,
        };
        let data = linux_statfs_bytes(stat.mode, stat.size);
        if write_user(buf, &data).is_err() {
            return -14;
        }
        0
    }

    fn sys_fstatfs(fd: usize, buf: usize) -> isize {
        let context = match active_user_context_mut() {
            Ok(context) => context,
            Err(err) => return err,
        };
        let stat = match context.fds.stat(fd) {
            Ok(stat) => stat,
            Err(err) => return err,
        };
        let data = linux_statfs_bytes(stat.mode, stat.size);
        if write_user(buf, &data).is_err() {
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
            Ok(context) => {
                let resolved = match resolve_user_path(context, dirfd, &path) {
                    Ok(path) => path,
                    Err(err) => return err,
                };
                match context.fds.openat(-100, &resolved, flags) {
                    Ok(fd) => fd as isize,
                    Err(err) => err,
                }
            }
            Err(err) => err,
        }
    }

    fn sys_dup(oldfd: usize) -> isize {
        match active_user_context_mut() {
            Ok(context) => context.fds.dup(oldfd, 3),
            Err(err) => err,
        }
    }

    fn sys_dup3(oldfd: usize, newfd: usize, flags: usize) -> isize {
        const O_CLOEXEC: usize = 0o2000000;
        if flags & !O_CLOEXEC != 0 {
            return -22;
        }
        match active_user_context_mut() {
            Ok(context) => context.fds.dup_to(oldfd, newfd),
            Err(err) => err,
        }
    }

    fn sys_fcntl(fd: usize, cmd: usize, arg: usize) -> isize {
        const F_DUPFD: usize = 0;
        const F_GETFD: usize = 1;
        const F_SETFD: usize = 2;
        const F_GETFL: usize = 3;
        const F_SETFL: usize = 4;
        const F_DUPFD_CLOEXEC: usize = 1030;

        let context = match active_user_context_mut() {
            Ok(context) => context,
            Err(err) => return err,
        };
        match cmd {
            F_DUPFD | F_DUPFD_CLOEXEC => context.fds.dup(fd, arg),
            F_GETFD => context.fds.access(fd),
            F_SETFD => context.fds.access(fd),
            F_GETFL => {
                let status = context.fds.access(fd);
                if status < 0 { status } else { 0 }
            }
            F_SETFL => context.fds.access(fd),
            _ => -22,
        }
    }

    fn sys_ioctl(fd: usize, request: usize, _arg: usize) -> isize {
        const TIOCGWINSZ: usize = 0x5413;

        let status = match active_user_context_mut() {
            Ok(context) => context.fds.access(fd),
            Err(err) => return err,
        };
        if status < 0 {
            return status;
        }
        match request {
            TIOCGWINSZ => -25,
            _ => -25,
        }
    }

    fn sys_getcwd(buf: usize, len: usize) -> isize {
        let context = match active_user_context_mut() {
            Ok(context) => context,
            Err(err) => return err,
        };
        let cwd = context.cwd.as_bytes();
        if len < cwd.len() + 1 {
            return -34;
        }
        if write_user(buf, cwd).is_err() || write_user(buf + cwd.len(), &[0]).is_err() {
            return -14;
        }
        buf as isize
    }

    fn sys_chdir(path_ptr: usize) -> isize {
        let path = match read_user_cstr(path_ptr) {
            Ok(path) => path,
            Err(err) => return err,
        };
        let context = match active_user_context_mut() {
            Ok(context) => context,
            Err(err) => return err,
        };
        let resolved = match resolve_user_path(context, -100, &path) {
            Ok(path) => path,
            Err(err) => return err,
        };
        if resolved == "/" {
            context.cwd = String::from("/");
            return 0;
        }
        match crate::testdisk::list_dir(resolved.trim_start_matches('/')) {
            Ok(_) => {
                context.cwd = resolved;
                0
            }
            Err(crate::testdisk::TestDiskError::NotFound) => -2,
            Err(crate::testdisk::TestDiskError::NoBlockDevice) => -19,
            Err(_) => -20,
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

    fn sys_clone(
        parent_tf: &TrapFrame,
        flags: usize,
        stack: usize,
        _ptid: usize,
        _ctid: usize,
        _tls: usize,
    ) -> isize {
        let exit_signal = flags & 0xff;
        if exit_signal != 0 && exit_signal != SIGCHLD {
            #[cfg(feature = "axstd")]
            println!("[syscall] clone unsupported flags=0x{:x}", flags);
            return -22;
        }
        let child_pid = alloc_pid();
        let context = match active_user_context_mut() {
            Ok(context) => context,
            Err(err) => return err,
        };

        let child_context = match context.fork_for_child(child_pid, parent_tf, stack) {
            Ok(context) => context,
            Err(err) => return err,
        };

        #[cfg(feature = "axstd")]
        println!(
            "[syscall] clone minimal pid={} flags=0x{:x} stack=0x{:x}",
            child_pid, flags, stack
        );

        let handle = arceos_api::task::ax_spawn(
            move || {
                let code = match enter_user(Box::new(child_context)) {
                    Ok(()) => 0,
                    Err(err) => {
                        #[cfg(feature = "axstd")]
                        println!("[runtime] child {} failed: {:?}", child_pid, err);
                        127
                    }
                };
                unregister_user_context(arceos_api::task::ax_current_task_id());
                arceos_api::task::ax_exit(code);
            },
            String::from("user-child"),
            0x20000,
        );
        context.children.push_back(ChildProcess {
            pid: child_pid,
            handle: Some(handle),
            exited_code: None,
        });
        child_pid as isize
    }

    fn sys_wait4(pid: isize, status_ptr: usize, options: usize) -> isize {
        const WNOHANG: usize = 1;
        if options & !WNOHANG != 0 {
            return -22;
        }
        let context = match active_user_context_mut() {
            Ok(context) => context,
            Err(err) => return err,
        };
        let Some(index) = context
            .children
            .iter()
            .position(|child| pid == -1 || pid == 0 || child.pid == pid as usize)
        else {
            return if options & WNOHANG != 0 { 0 } else { -10 };
        };
        let mut child = context.children.remove(index).unwrap();
        let code = match (child.exited_code, child.handle.take()) {
            (Some(code), _) => code,
            (None, Some(handle)) => match arceos_api::task::ax_wait_for_exit(handle) {
                Some(code) => code,
                None => return -10,
            },
            (None, None) => return -10,
        };
        if status_ptr != 0 {
            let status = ((code as u32) & 0xff) << 8;
            if write_user(status_ptr, &status.to_ne_bytes()).is_err() {
                return -14;
            }
        }
        child.pid as isize
    }

    fn sys_execve(path_ptr: usize) -> isize {
        let path = match read_user_cstr(path_ptr) {
            Ok(path) => path,
            Err(err) => return err,
        };
        let context = match active_user_context_mut() {
            Ok(context) => context,
            Err(err) => return err,
        };
        let resolved = match resolve_user_path(context, -100, &path) {
            Ok(path) => path,
            Err(err) => return err,
        };
        let elf_bytes = match crate::testdisk::read_file(resolved.trim_start_matches('/')) {
            Ok(bytes) => bytes,
            Err(crate::testdisk::TestDiskError::NotFound) => return -2,
            Err(crate::testdisk::TestDiskError::NoBlockDevice) => return -19,
            Err(_) => return -5,
        };
        let parsed = match elf::parse(&elf_bytes) {
            Ok(parsed) => parsed,
            Err(_) => return -8,
        };
        let image = match load_image(&elf_bytes, &parsed) {
            Ok(image) => image,
            Err(_) => return -12,
        };

        #[cfg(feature = "axstd")]
        println!("[syscall] execve({})", resolved);

        context.image = image;
        context.fds = FdTable::new();
        context.heap_end = context.image.heap_start;
        context.mmap_cursor = USER_MMAP_BASE;
        context.exe_path = resolved;
        context.resume_tf = None;
        unsafe {
            axhal::asm::write_user_page_table(context.image.aspace.page_table_root());
            axhal::asm::flush_tlb(None);
        }

        let uspace_context =
            UspaceContext::new(context.image.entry, va!(context.image.stack_top), 0);
        let kstack_top = current_kernel_stack_top();
        unsafe {
            uspace_context.enter_uspace(kstack_top);
        }
    }

    fn sys_set_tid_address(tidptr: usize) -> isize {
        match active_user_context_mut() {
            Ok(context) => {
                context.clear_tid_address = tidptr;
                context.pid as isize
            }
            Err(err) => err,
        }
    }

    fn sys_tgkill(tgid: usize, tid: usize, sig: usize) -> isize {
        let context = match active_user_context_mut() {
            Ok(context) => context,
            Err(err) => return err,
        };
        if (tgid != 0 && tgid != context.pid) || (tid != 0 && tid != context.pid) {
            return -3;
        }
        if sig == 0 { 0 } else { -22 }
    }

    fn sys_exit(code: usize) -> isize {
        #[cfg(feature = "axstd")]
        println!("[syscall] exit({})", code);
        if let Ok(context) = active_user_context_mut() {
            if context.clear_tid_address != 0 {
                let _ = write_user(context.clear_tid_address, &0_u32.to_ne_bytes());
            }
        }
        unregister_user_context(arceos_api::task::ax_current_task_id());
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
            if map_end > map_start {
                mappings_push_or_extend(
                    &mut context.image.mappings,
                    UserMapping {
                        start: map_start.as_usize(),
                        size: map_end - map_start,
                        flags: MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
                    },
                );
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
            Ok(()) => {
                mappings_push_or_extend(
                    &mut context.image.mappings,
                    UserMapping {
                        start,
                        size,
                        flags: map_flags,
                    },
                );
                start as isize
            }
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
            Ok(()) => {
                remove_mapping_range(&mut context.image.mappings, start.as_usize(), size);
                0
            }
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
        let context = active_user_context().ok_or(())?;
        if !context.image.aspace.can_access_range(
            va!(start),
            buf.len(),
            MappingFlags::READ | MappingFlags::USER,
        ) {
            return Err(());
        }
        activate_user_aspace(context);
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
        let context = active_user_context().ok_or(())?;
        if !context.image.aspace.can_access_range(
            va!(start),
            buf.len(),
            MappingFlags::WRITE | MappingFlags::USER,
        ) {
            return Err(());
        }
        activate_user_aspace(context);
        with_user_memory_access(|| unsafe {
            core::ptr::copy_nonoverlapping(buf.as_ptr(), start as *mut u8, buf.len());
        });
        Ok(())
    }

    fn active_user_context_mut() -> Result<&'static mut UserContext, isize> {
        active_user_context_mut_by_task(arceos_api::task::ax_current_task_id()).ok_or(-14)
    }

    fn active_user_context() -> Option<&'static UserContext> {
        let task_id = arceos_api::task::ax_current_task_id();
        unsafe {
            let base = core::ptr::addr_of!(ACTIVE_USER_CONTEXTS) as *const UserContextSlot;
            for idx in 0..MAX_USER_CONTEXTS {
                let slot = *base.add(idx);
                if slot.task_id == task_id && !slot.context.is_null() {
                    return Some(&*slot.context);
                }
            }
        }
        None
    }

    fn activate_current_user_aspace() {
        if let Some(context) = active_user_context() {
            activate_user_aspace(context);
        }
    }

    fn activate_user_aspace(context: &UserContext) {
        unsafe {
            axhal::asm::write_user_page_table(context.image.aspace.page_table_root());
            axhal::asm::flush_tlb(None);
        }
    }

    fn active_user_context_mut_by_task(task_id: u64) -> Option<&'static mut UserContext> {
        unsafe {
            let base = core::ptr::addr_of_mut!(ACTIVE_USER_CONTEXTS) as *mut UserContextSlot;
            for idx in 0..MAX_USER_CONTEXTS {
                let slot = base.add(idx);
                if (*slot).task_id == task_id && !(*slot).context.is_null() {
                    return Some(&mut *(*slot).context);
                }
            }
        }
        None
    }

    fn register_user_context(task_id: u64, context: *mut UserContext) -> bool {
        unsafe {
            let base = core::ptr::addr_of_mut!(ACTIVE_USER_CONTEXTS) as *mut UserContextSlot;
            let mut free_slot: *mut UserContextSlot = core::ptr::null_mut();
            for idx in 0..MAX_USER_CONTEXTS {
                let slot = base.add(idx);
                if (*slot).task_id == task_id {
                    (*slot).context = context;
                    return true;
                }
                if free_slot.is_null() && (*slot).context.is_null() {
                    free_slot = slot;
                }
            }
            if !free_slot.is_null() {
                (*free_slot) = UserContextSlot { task_id, context };
                return true;
            }
        }
        false
    }

    fn unregister_user_context(task_id: u64) {
        unsafe {
            let base = core::ptr::addr_of_mut!(ACTIVE_USER_CONTEXTS) as *mut UserContextSlot;
            for idx in 0..MAX_USER_CONTEXTS {
                let slot = base.add(idx);
                if (*slot).task_id == task_id {
                    *slot = UserContextSlot::empty();
                    return;
                }
            }
        }
    }

    fn alloc_pid() -> usize {
        unsafe {
            let pid = NEXT_PID;
            NEXT_PID = NEXT_PID.saturating_add(1);
            pid
        }
    }

    fn mappings_push_or_extend(mappings: &mut Vec<UserMapping>, mapping: UserMapping) {
        if mapping.size == 0 {
            return;
        }
        if let Some(last) = mappings.last_mut()
            && last.start + last.size == mapping.start
            && last.flags == mapping.flags
        {
            last.size += mapping.size;
            return;
        }
        mappings.push(mapping);
    }

    fn remove_mapping_range(mappings: &mut Vec<UserMapping>, start: usize, size: usize) {
        let end = start.saturating_add(size);
        let mut next = Vec::new();
        for mapping in mappings.iter().copied() {
            let mapping_end = mapping.start.saturating_add(mapping.size);
            if mapping_end <= start || mapping.start >= end {
                next.push(mapping);
                continue;
            }
            if mapping.start < start {
                next.push(UserMapping {
                    start: mapping.start,
                    size: start - mapping.start,
                    flags: mapping.flags,
                });
            }
            if mapping_end > end {
                next.push(UserMapping {
                    start: end,
                    size: mapping_end - end,
                    flags: mapping.flags,
                });
            }
        }
        *mappings = next;
    }

    fn copy_user_range(
        src: &AddrSpace,
        dst: &AddrSpace,
        start: usize,
        size: usize,
    ) -> Result<(), ()> {
        let mut offset = 0;
        let mut buf = [0_u8; 256];
        while offset < size {
            let n = (size - offset).min(buf.len());
            src.read(va!(start + offset), &mut buf[..n])
                .map_err(|_| ())?;
            dst.write(va!(start + offset), &buf[..n]).map_err(|_| ())?;
            offset += n;
        }
        Ok(())
    }

    fn child_trap_frame(parent_tf: &TrapFrame, child_stack: usize) -> TrapFrame {
        let mut tf = *parent_tf;
        #[cfg(target_arch = "riscv64")]
        {
            tf.regs.a0 = 0;
            if child_stack != 0 {
                tf.regs.sp = child_stack.saturating_sub(16);
            }
            tf.sepc = tf.sepc.saturating_add(4);
        }
        #[cfg(target_arch = "loongarch64")]
        {
            tf.regs.a0 = 0;
            if child_stack != 0 {
                tf.regs.sp = child_stack.saturating_sub(16) & !0xf;
            }
            tf.era = tf.era.saturating_add(4);
        }
        tf
    }

    fn resolve_user_path(context: &UserContext, dirfd: isize, path: &str) -> Result<String, isize> {
        const AT_FDCWD: isize = -100;
        if path.is_empty() {
            return Err(-2);
        }
        if path.starts_with('/') {
            return Ok(normalize_abs_path(path));
        }
        if dirfd != AT_FDCWD {
            return Err(-22);
        }
        let mut base = context.cwd.clone();
        if !base.ends_with('/') {
            base.push('/');
        }
        base.push_str(path);
        Ok(normalize_abs_path(&base))
    }

    fn normalize_abs_path(path: &str) -> String {
        let mut parts: Vec<&str> = Vec::new();
        for part in path.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    parts.pop();
                }
                value => parts.push(value),
            }
        }
        if parts.is_empty() {
            String::from("/")
        } else {
            let mut result = String::from("/");
            for (idx, part) in parts.iter().enumerate() {
                if idx != 0 {
                    result.push('/');
                }
                result.push_str(part);
            }
            result
        }
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

    fn linux_statfs_bytes(mode: u32, size: u64) -> [u8; 128] {
        let mut statfs = [0_u8; 128];
        statfs[0..8].copy_from_slice(&0xEF53_u64.to_ne_bytes());
        statfs[8..16].copy_from_slice(&4096_u64.to_ne_bytes());
        statfs[16..24].copy_from_slice(&4096_u64.to_ne_bytes());
        statfs[24..32].copy_from_slice(&size.to_ne_bytes());
        statfs[32..40].copy_from_slice(&size.to_ne_bytes());
        statfs[48..56].copy_from_slice(&mode.to_ne_bytes());
        statfs
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
