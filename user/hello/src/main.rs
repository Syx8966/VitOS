#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

#[cfg(target_arch = "riscv64")]
global_asm!(
    r#"
    .section .text.entry, "ax"
    .globl _start
_start:
    li a0, 1
    la a1, message
    li a2, 16
    li a7, 64
    ecall

    li a0, 0
    li a7, 93
    ecall

1:
    j 1b

    .section .rodata, "a"
message:
    .ascii "hello from user\n"
message_end:
"#
);

#[cfg(target_arch = "loongarch64")]
global_asm!(
    r#"
    .section .text.entry, "ax"
    .globl _start
_start:
    li.d $a0, 1
    la.local $a1, message
    li.d $a2, 16
    li.d $a7, 64
    syscall 0

    li.d $a0, 0
    li.d $a7, 93
    syscall 0

1:
    b 1b

    .section .rodata, "a"
message:
    .ascii "hello from user\n"
message_end:
"#
);

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
