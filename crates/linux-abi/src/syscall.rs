#[cfg(feature = "axstd")]
use axstd::println;

pub const SYS_WRITE: usize = 64;
pub const SYS_EXIT: usize = 93;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyscallStatus {
    TraceOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SyscallInfo {
    pub nr: usize,
    pub name: &'static str,
    pub status: SyscallStatus,
}

pub const BOOTSTRAP_SYSCALLS: &[SyscallInfo] = &[
    SyscallInfo {
        nr: SYS_WRITE,
        name: "write",
        status: SyscallStatus::TraceOnly,
    },
    SyscallInfo {
        nr: SYS_EXIT,
        name: "exit",
        status: SyscallStatus::TraceOnly,
    },
];

pub fn init_trace() {
    #[cfg(feature = "axstd")]
    {
        println!("[syscall] bootstrap table:");
        for syscall in BOOTSTRAP_SYSCALLS {
            println!(
                "[syscall] nr={} name={} status={:?}",
                syscall.nr, syscall.name, syscall.status
            );
        }
    }
}
