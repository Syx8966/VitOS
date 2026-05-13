#[cfg(feature = "uspace")]
mod imp {
    extern crate alloc;

    use alloc::boxed::Box;
    use axstd::println;

    pub fn run_stage3() {
        println!("[runner] stage3 basic runner scaffold");
        println!("#### OS COMP TEST GROUP START basic ####");

        let ext4_basic_write = crate::testdisk::smoke_and_read_basic_write();
        run_case(
            "embedded-hello",
            crate::elf::embedded_user_hello_for_current_arch(),
        );
        run_case(
            "local-basic",
            crate::elf::embedded_local_basic_for_current_arch(),
        );
        run_optional_ext4_case("ext4-musl-basic-write", ext4_basic_write);

        println!("#### OS COMP TEST GROUP END basic ####");
        axhal::power::system_off();
    }

    fn run_case(name: &'static str, elf: &'static [u8]) {
        println!("[runner] running {}", name);
        match crate::runtime::run_elf(name, elf) {
            Ok(code) => println!("[runner] {} exit_code={}", name, code),
            Err(err) => println!("[runner] {} failed: {:?}", name, err),
        }
    }

    fn run_optional_ext4_case(name: &'static str, bytes: Option<alloc::vec::Vec<u8>>) {
        let Some(bytes) = bytes else {
            println!("[runner] {} skipped: no EXT4 file", name);
            return;
        };

        let elf: &'static [u8] = Box::leak(bytes.into_boxed_slice());
        match crate::elf::parse(elf) {
            Ok(parsed) if parsed.header.machine == current_machine() => run_case(name, elf),
            Ok(parsed) => println!(
                "[runner] {} skipped: ELF machine {} does not match current arch {}",
                name,
                parsed.header.machine,
                current_machine()
            ),
            Err(err) => println!("[runner] {} skipped: not executable ELF: {:?}", name, err),
        }
    }

    fn current_machine() -> u16 {
        match option_env!("VITOS_BOOT_ARCH") {
            Some("loongarch64") => 258,
            _ => 243,
        }
    }
}

#[cfg(not(feature = "uspace"))]
mod imp {
    pub fn run_stage3() {}
}

pub use imp::*;
