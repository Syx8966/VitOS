#[cfg(feature = "uspace")]
mod imp {
    use axhal::context::{TrapFrame, UspaceContext};
    use axhal::paging::MappingFlags;
    use axhal::trap::{SYSCALL, register_trap_handler};
    use axmm::AddrSpace;
    #[cfg(feature = "axstd")]
    use axstd::{print, println};
    use linux_abi::syscall::{SYS_EXIT, SYS_WRITE};
    use memory_addr::{MemoryAddr, VirtAddr, va};

    use crate::elf::{self, LoadSegment, ParsedElf};

    const USER_ASPACE_BASE: usize = 0x1000;
    const USER_ASPACE_SIZE: usize = 0x0000_8000_0000_0000 - USER_ASPACE_BASE;
    const USER_STACK_TOP: usize = 0x0000_4000_0000_0000;
    const USER_STACK_SIZE: usize = 0x4000;

    static mut ACTIVE_USER_ASPACE: *const AddrSpace = core::ptr::null();

    #[derive(Debug)]
    pub enum RuntimeError {
        Elf(elf::ElfError),
        Map,
        Copy,
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
    }

    pub fn run_embedded_hello() -> Result<(), RuntimeError> {
        let elf_bytes = elf::embedded_user_hello_for_current_arch();
        let parsed = elf::parse(elf_bytes)?;
        let image = load_image(elf_bytes, &parsed)?;

        #[cfg(feature = "axstd")]
        println!(
            "[runtime] user image ready entry=0x{:x} stack_top=0x{:x} root=0x{:x}",
            image.entry,
            image.stack_top,
            image.aspace.page_table_root().as_usize()
        );

        enter_user(image)
    }

    fn load_image(elf_bytes: &[u8], parsed: &ParsedElf) -> Result<UserImage, RuntimeError> {
        let mut aspace = axmm::new_user_aspace(va!(USER_ASPACE_BASE), USER_ASPACE_SIZE)
            .map_err(|_| RuntimeError::Map)?;

        for segment in parsed.load_segments() {
            map_load_segment(&mut aspace, elf_bytes, segment)?;
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
        let data = elf_bytes.get(file_start..file_end).ok_or(RuntimeError::Copy)?;
        aspace
            .write(map_start + page_offset, data)
            .map_err(|_| RuntimeError::Copy)?;

        Ok(())
    }

    fn enter_user(image: UserImage) -> Result<(), RuntimeError> {
        unsafe {
            ACTIVE_USER_ASPACE = &image.aspace;
            axhal::asm::write_user_page_table(image.aspace.page_table_root());
            axhal::asm::flush_tlb(None);
        }

        let context = UspaceContext::new(image.entry, va!(image.stack_top), 0);
        let kstack_top = current_kernel_stack_top();

        #[cfg(feature = "axstd")]
        println!("[runtime] enter user");

        unsafe {
            context.enter_uspace(kstack_top);
        }
    }

    fn current_kernel_stack_top() -> VirtAddr {
        let local = 0usize;
        va!((&local as *const usize as usize).align_up_4k())
    }

    #[register_trap_handler(SYSCALL)]
    fn handle_syscall(tf: &TrapFrame, nr: usize) -> isize {
        match nr {
            SYS_WRITE => sys_write(tf.arg0(), tf.arg1(), tf.arg2()),
            SYS_EXIT => sys_exit(tf.arg0()),
            _ => {
                #[cfg(feature = "axstd")]
                println!("[syscall] unsupported nr={}", nr);
                -38
            }
        }
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

    fn sys_exit(code: usize) -> isize {
        #[cfg(feature = "axstd")]
        println!("[syscall] exit({})", code);
        axhal::power::system_off();
    }

    fn read_user(start: usize, buf: &mut [u8]) -> Result<(), ()> {
        let aspace = unsafe { ACTIVE_USER_ASPACE.as_ref().ok_or(())? };
        aspace.read(va!(start), buf).map_err(|_| ())
    }
}

#[cfg(not(feature = "uspace"))]
mod imp {
    #[derive(Debug)]
    pub enum RuntimeError {
        Disabled,
    }

    pub fn run_embedded_hello() -> Result<(), RuntimeError> {
        Err(RuntimeError::Disabled)
    }
}

pub use imp::*;
