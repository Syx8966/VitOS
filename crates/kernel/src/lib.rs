#![cfg_attr(feature = "axstd", no_std)]

pub mod loader;

#[cfg(feature = "axstd")]
use axstd::println;

pub fn init() {
    let _loader_state = loader::init();
    linux_abi::syscall::init_trace();

    #[cfg(feature = "axstd")]
    println!("stage2 = loader/syscall scaffold ready");
}
