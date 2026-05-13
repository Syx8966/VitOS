#![cfg_attr(feature = "axstd", no_std)]

pub mod elf;
pub mod loader;
pub mod runner;
pub mod runtime;
pub mod testdisk;

#[cfg(feature = "axstd")]
use axstd::println;

pub fn init() {
    let _loader_state = loader::init();
    linux_abi::syscall::init_trace();

    #[cfg(feature = "axstd")]
    println!("stage2 = loader/syscall scaffold ready");

    runner::run_stage3();
}
