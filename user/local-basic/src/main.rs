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
    addi sp, sp, -512

    li a0, 1
    la a1, start_msg
    li a2, 19
    li a7, 64
    ecall

    li a0, 1
    la a1, brk_msg
    li a2, 13
    li a7, 64
    ecall

    li a0, 0
    li a7, 214
    ecall
    beqz a0, fail
    mv s0, a0

    li a0, 1
    la a1, brk0_msg
    li a2, 11
    li a7, 64
    ecall

    li t0, 4096
    add a0, s0, t0
    mv s1, a0
    li a7, 214
    ecall
    bltu a0, s1, fail

    li a0, 1
    la a1, brk1_msg
    li a2, 13
    li a7, 64
    ecall

    li a0, 1
    la a1, mmap_msg
    li a2, 14
    li a7, 64
    ecall

    li a0, 0
    li a1, 4096
    li a2, 3
    li a3, 0x22
    li a4, -1
    li a5, 0
    li a7, 222
    ecall
    bltz a0, fail
    mv s2, a0
    li t0, 0x41
    sb t0, 0(s2)

    mv a0, s2
    li a1, 4096
    li a7, 215
    ecall
    bnez a0, fail

    li a0, 1
    la a1, time_msg
    li a2, 14
    li a7, 64
    ecall

    mv a0, sp
    li a1, 0
    li a7, 169
    ecall
    bnez a0, fail

    li a0, 1
    la a1, uname_msg
    li a2, 15
    li a7, 64
    ecall

    addi a0, sp, 32
    li a7, 160
    ecall
    bnez a0, fail

    li a0, 1
    la a1, yield_msg
    li a2, 15
    li a7, 64
    ecall

    li a7, 124
    ecall
    bnez a0, fail

    li a0, 1
    la a1, sleep_msg
    li a2, 15
    li a7, 64
    ecall

    sd zero, 448(sp)
    sd zero, 456(sp)
    addi a0, sp, 448
    li a1, 0
    li a7, 101
    ecall
    bnez a0, fail

    li a0, 1
    la a1, fstat_msg
    li a2, 15
    li a7, 64
    ecall

    li a0, 1
    addi a1, sp, 64
    li a7, 80
    ecall
    bnez a0, fail

    li a0, 1
    la a1, fd_msg
    li a2, 10
    li a7, 64
    ecall

    li a0, 1
    li a7, 23
    ecall
    bltz a0, fail
    mv s4, a0

    mv a0, s4
    li a1, 3
    li a2, 0
    li a7, 25
    ecall
    bltz a0, fail

    mv a0, s4
    li a7, 57
    ecall
    bnez a0, fail

    li a0, 1
    li a1, 9
    li a2, 0
    li a7, 24
    ecall
    li t0, 9
    bne a0, t0, fail
    li a0, 9
    li a7, 57
    ecall
    bnez a0, fail

    addi a0, sp, 320
    li a1, 64
    li a7, 17
    ecall
    beqz a0, fail
    lb t0, 320(sp)
    li t1, 0x2f
    bne t0, t1, fail

    la a0, root_path
    li a7, 49
    ecall
    bnez a0, fail

    li a0, 1
    li a1, 0x5413
    addi a2, sp, 320
    li a7, 29
    ecall
    li t0, -25
    bne a0, t0, fail

    li a0, 1
    la a1, proc_msg
    li a2, 14
    li a7, 64
    ecall

    li a0, 17
    li a1, 0
    li a7, 220
    ecall
    beqz a0, proc_child
    bltz a0, fail
    mv s5, a0

    mv a0, s5
    addi a1, sp, 400
    li a2, 0
    li a3, 0
    li a7, 260
    ecall
    bne a0, s5, fail
    lw t0, 400(sp)
    li t1, 768
    bne t0, t1, fail

    li a0, 1
    la a1, proc_ok_msg
    li a2, 27
    li a7, 64
    ecall
    j proc_done

proc_child:
    li a0, 1
    la a1, proc_child_msg
    li a2, 11
    li a7, 64
    ecall
    li a0, 3
    li a7, 93
    ecall

proc_done:

    li a0, -100
    la a1, file_path
    addi a2, sp, 0
    li a3, 0
    li a7, 79
    ecall
    li t0, -19
    beq a0, t0, fs_skip
    li t0, -2
    beq a0, t0, fs_skip
    bltz a0, fail

    li a0, -100
    la a1, root_path
    addi a2, sp, 320
    li a3, 64
    li a7, 78
    ecall
    li t0, -2
    beq a0, t0, fs_skip

    li a0, -100
    la a1, file_path
    li a2, 4
    li a3, 0
    li a7, 48
    ecall
    li t0, -19
    beq a0, t0, fs_skip
    li t0, -2
    beq a0, t0, fs_skip
    blt a0, zero, fail

    la a0, root_path
    mv a1, sp
    li a7, 43
    ecall
    li t0, -19
    beq a0, t0, fs_skip
    blt a0, zero, fail

    li a0, 1
    mv a1, sp
    li a7, 44
    ecall
    li t0, -9
    beq a0, t0, fs_skip
    blt a0, zero, fail

    li a0, 1
    la a1, fs_msg
    li a2, 12
    li a7, 64
    ecall

    li a0, -100
    la a1, file_path
    li a2, 0
    li a3, 0
    li a7, 56
    ecall
    li t0, -19
    beq a0, t0, fs_skip
    li t0, -2
    beq a0, t0, fs_skip
    bltz a0, fail
    mv s3, a0

    mv a0, s3
    addi a1, sp, 64
    li a7, 80
    ecall
    bnez a0, fail

    mv a0, s3
    addi a1, sp, 320
    li a2, 4
    li a7, 63
    ecall
    li t0, 4
    bne a0, t0, fail
    lb t0, 320(sp)
    li t1, 0x7f
    bne t0, t1, fail
    lb t0, 321(sp)
    li t1, 0x45
    bne t0, t1, fail
    lb t0, 322(sp)
    li t1, 0x4c
    bne t0, t1, fail
    lb t0, 323(sp)
    li t1, 0x46
    bne t0, t1, fail

    mv a0, s3
    li a1, 0
    li a2, 0
    li a7, 62
    ecall
    bnez a0, fail

    mv a0, s3
    li a7, 57
    ecall
    bnez a0, fail

    li a0, -100
    la a1, dir_path
    li a2, 0
    li a3, 0
    li a7, 56
    ecall
    bltz a0, fail
    mv s3, a0

    mv a0, s3
    addi a1, sp, 320
    li a2, 128
    li a7, 61
    ecall
    blez a0, fail

    mv a0, s3
    li a7, 57
    ecall
    bnez a0, fail

    li a0, 1
    la a1, fs_ok_msg
    li a2, 25
    li a7, 64
    ecall
fs_skip:

    li a0, 1
    la a1, ok_msg
    li a2, 30
    li a7, 64
    ecall

    li a0, 0
    li a7, 93
    ecall

fail:
    li a0, 1
    la a1, fail_msg
    li a2, 18
    li a7, 64
    ecall

    li a0, 1
    li a7, 93
    ecall

1:
    j 1b

    .section .rodata, "a"
start_msg:
    .ascii "local-basic: start\n"
start_msg_end:
ok_msg:
    .ascii "local-basic: syscall smoke ok\n"
ok_msg_end:
brk_msg:
    .ascii "check: brk()\n"
brk0_msg:
    .ascii "brk(0): ok\n"
brk1_msg:
    .ascii "brk grow: ok\n"
mmap_msg:
    .ascii "check: mmap()\n"
time_msg:
    .ascii "check: time()\n"
uname_msg:
    .ascii "check: uname()\n"
yield_msg:
    .ascii "check: yield()\n"
sleep_msg:
    .ascii "check: sleep()\n"
fstat_msg:
    .ascii "check: fstat()\n"
fd_msg:
    .ascii "check: fd\n"
proc_msg:
    .ascii "check: proc()\n"
proc_child_msg:
    .ascii "proc child\n"
proc_ok_msg:
    .ascii "local-basic: proc smoke ok\n"
fs_msg:
    .ascii "check: fs()\n"
fs_ok_msg:
    .ascii "local-basic: fs smoke ok\n"
file_path:
    .asciz "/musl/basic/write"
dir_path:
    .asciz "/musl/basic"
root_path:
    .asciz "/"
fail_msg:
    .ascii "local-basic: FAIL\n"
fail_msg_end:
"#
);

#[cfg(target_arch = "loongarch64")]
global_asm!(
    r#"
    .section .text.entry, "ax"
    .globl _start
_start:
    addi.d $sp, $sp, -512

    li.d $a0, 1
    la.local $a1, start_msg
    li.d $a2, 19
    li.d $a7, 64
    syscall 0

    li.d $a0, 1
    la.local $a1, brk_msg
    li.d $a2, 13
    li.d $a7, 64
    syscall 0

    li.d $a0, 0
    li.d $a7, 214
    syscall 0
    beqz $a0, fail
    move $s0, $a0

    li.d $a0, 1
    la.local $a1, brk0_msg
    li.d $a2, 11
    li.d $a7, 64
    syscall 0

    li.d $t0, 4096
    add.d $a0, $s0, $t0
    move $s1, $a0
    li.d $a7, 214
    syscall 0
    bltu $a0, $s1, fail

    li.d $a0, 1
    la.local $a1, brk1_msg
    li.d $a2, 13
    li.d $a7, 64
    syscall 0

    li.d $a0, 1
    la.local $a1, mmap_msg
    li.d $a2, 14
    li.d $a7, 64
    syscall 0

    li.d $a0, 0
    li.d $a1, 4096
    li.d $a2, 3
    li.d $a3, 0x22
    li.d $a4, -1
    li.d $a5, 0
    li.d $a7, 222
    syscall 0
    blt $a0, $zero, fail
    move $s2, $a0
    li.d $t0, 0x41
    st.b $t0, $s2, 0

    move $a0, $s2
    li.d $a1, 4096
    li.d $a7, 215
    syscall 0
    bnez $a0, fail

    li.d $a0, 1
    la.local $a1, time_msg
    li.d $a2, 14
    li.d $a7, 64
    syscall 0

    move $a0, $sp
    li.d $a1, 0
    li.d $a7, 169
    syscall 0
    bnez $a0, fail

    li.d $a0, 1
    la.local $a1, uname_msg
    li.d $a2, 15
    li.d $a7, 64
    syscall 0

    addi.d $a0, $sp, 32
    li.d $a7, 160
    syscall 0
    bnez $a0, fail

    li.d $a0, 1
    la.local $a1, yield_msg
    li.d $a2, 15
    li.d $a7, 64
    syscall 0

    li.d $a7, 124
    syscall 0
    bnez $a0, fail

    li.d $a0, 1
    la.local $a1, sleep_msg
    li.d $a2, 15
    li.d $a7, 64
    syscall 0

    st.d $zero, $sp, 448
    st.d $zero, $sp, 456
    addi.d $a0, $sp, 448
    li.d $a1, 0
    li.d $a7, 101
    syscall 0
    bnez $a0, fail

    li.d $a0, 1
    la.local $a1, fstat_msg
    li.d $a2, 15
    li.d $a7, 64
    syscall 0

    li.d $a0, 1
    addi.d $a1, $sp, 64
    li.d $a7, 80
    syscall 0
    bnez $a0, fail

    li.d $a0, 1
    la.local $a1, fd_msg
    li.d $a2, 10
    li.d $a7, 64
    syscall 0

    li.d $a0, 1
    li.d $a7, 23
    syscall 0
    blt $a0, $zero, fail
    move $s4, $a0

    move $a0, $s4
    li.d $a1, 3
    li.d $a2, 0
    li.d $a7, 25
    syscall 0
    blt $a0, $zero, fail

    move $a0, $s4
    li.d $a7, 57
    syscall 0
    bnez $a0, fail

    li.d $a0, 1
    li.d $a1, 9
    li.d $a2, 0
    li.d $a7, 24
    syscall 0
    li.d $t0, 9
    bne $a0, $t0, fail
    li.d $a0, 9
    li.d $a7, 57
    syscall 0
    bnez $a0, fail

    addi.d $a0, $sp, 320
    li.d $a1, 64
    li.d $a7, 17
    syscall 0
    beqz $a0, fail
    ld.b $t0, $sp, 320
    li.d $t1, 0x2f
    bne $t0, $t1, fail

    la.local $a0, root_path
    li.d $a7, 49
    syscall 0
    bnez $a0, fail

    li.d $a0, 1
    li.d $a1, 0x5413
    addi.d $a2, $sp, 320
    li.d $a7, 29
    syscall 0
    li.d $t0, -25
    bne $a0, $t0, fail

    li.d $a0, 1
    la.local $a1, proc_msg
    li.d $a2, 14
    li.d $a7, 64
    syscall 0

    li.d $a0, 17
    li.d $a1, 0
    li.d $a7, 220
    syscall 0
    beqz $a0, proc_child
    blt $a0, $zero, fail
    move $s5, $a0

    move $a0, $s5
    addi.d $a1, $sp, 400
    li.d $a2, 0
    li.d $a3, 0
    li.d $a7, 260
    syscall 0
    bne $a0, $s5, fail
    ld.w $t0, $sp, 400
    li.d $t1, 768
    bne $t0, $t1, fail

    li.d $a0, 1
    la.local $a1, proc_ok_msg
    li.d $a2, 27
    li.d $a7, 64
    syscall 0
    b proc_done

proc_child:
    li.d $a0, 1
    la.local $a1, proc_child_msg
    li.d $a2, 11
    li.d $a7, 64
    syscall 0
    li.d $a0, 3
    li.d $a7, 93
    syscall 0

proc_done:

    li.d $a0, 1
    la.local $a1, fs_msg
    li.d $a2, 12
    li.d $a7, 64
    syscall 0

    li.d $a0, -100
    la.local $a1, file_path
    li.d $a2, 0
    li.d $a3, 0
    li.d $a7, 56
    syscall 0
    li.d $t0, -19
    beq $a0, $t0, fs_skip
    li.d $t0, -2
    beq $a0, $t0, fs_skip
    blt $a0, $zero, fail
    move $s3, $a0

    move $a0, $s3
    addi.d $a1, $sp, 64
    li.d $a7, 80
    syscall 0
    bnez $a0, fail

    move $a0, $s3
    addi.d $a1, $sp, 320
    li.d $a2, 4
    li.d $a7, 63
    syscall 0
    li.d $t0, 4
    bne $a0, $t0, fail
    ld.b $t0, $sp, 320
    li.d $t1, 0x7f
    bne $t0, $t1, fail
    ld.b $t0, $sp, 321
    li.d $t1, 0x45
    bne $t0, $t1, fail
    ld.b $t0, $sp, 322
    li.d $t1, 0x4c
    bne $t0, $t1, fail
    ld.b $t0, $sp, 323
    li.d $t1, 0x46
    bne $t0, $t1, fail

    move $a0, $s3
    li.d $a1, 0
    li.d $a2, 0
    li.d $a7, 62
    syscall 0
    bnez $a0, fail

    move $a0, $s3
    li.d $a7, 57
    syscall 0
    bnez $a0, fail

    li.d $a0, -100
    la.local $a1, dir_path
    li.d $a2, 0
    li.d $a3, 0
    li.d $a7, 56
    syscall 0
    blt $a0, $zero, fail
    move $s3, $a0

    move $a0, $s3
    addi.d $a1, $sp, 320
    li.d $a2, 128
    li.d $a7, 61
    syscall 0
    bge $zero, $a0, fail

    move $a0, $s3
    li.d $a7, 57
    syscall 0
    bnez $a0, fail

    li.d $a0, 1
    la.local $a1, fs_ok_msg
    li.d $a2, 25
    li.d $a7, 64
    syscall 0
fs_skip:

    li.d $a0, 1
    la.local $a1, ok_msg
    li.d $a2, 30
    li.d $a7, 64
    syscall 0

    li.d $a0, 0
    li.d $a7, 93
    syscall 0

fail:
    li.d $a0, 1
    la.local $a1, fail_msg
    li.d $a2, 18
    li.d $a7, 64
    syscall 0

    li.d $a0, 1
    li.d $a7, 93
    syscall 0

1:
    b 1b

    .section .rodata, "a"
start_msg:
    .ascii "local-basic: start\n"
start_msg_end:
ok_msg:
    .ascii "local-basic: syscall smoke ok\n"
ok_msg_end:
brk_msg:
    .ascii "check: brk()\n"
brk0_msg:
    .ascii "brk(0): ok\n"
brk1_msg:
    .ascii "brk grow: ok\n"
mmap_msg:
    .ascii "check: mmap()\n"
time_msg:
    .ascii "check: time()\n"
uname_msg:
    .ascii "check: uname()\n"
yield_msg:
    .ascii "check: yield()\n"
sleep_msg:
    .ascii "check: sleep()\n"
fstat_msg:
    .ascii "check: fstat()\n"
fd_msg:
    .ascii "check: fd\n"
proc_msg:
    .ascii "check: proc()\n"
proc_child_msg:
    .ascii "proc child\n"
proc_ok_msg:
    .ascii "local-basic: proc smoke ok\n"
fs_msg:
    .ascii "check: fs()\n"
fs_ok_msg:
    .ascii "local-basic: fs smoke ok\n"
file_path:
    .asciz "/musl/basic/write"
dir_path:
    .asciz "/musl/basic"
root_path:
    .asciz "/"
fail_msg:
    .ascii "local-basic: FAIL\n"
fail_msg_end:
"#
);

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
