#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[cfg(feature = "axstd")]
use axstd::println;

#[cfg(feature = "axstd")]
fn arch_name() -> &'static str {
    option_env!("AX_ARCH").unwrap_or("unknown")
}

#[cfg_attr(feature = "axstd", unsafe(no_mangle))]
fn main() {
    println!("OSKernel-ArceOS starting");
    println!("stage = 1");
    println!("base_os = ArceOS");
    println!("arch = {}", arch_name());
    println!("status = minimal app booted");
    println!("===Vitality OS===");
    println!("Hello from VitOS!");
    println!("=================");
    vitos_kernel::init();
}
