#[cfg(feature = "axstd")]
use axstd::println;

pub const SYS_GETCWD: usize = 17;
pub const SYS_DUP: usize = 23;
pub const SYS_DUP3: usize = 24;
pub const SYS_FCNTL: usize = 25;
pub const SYS_IOCTL: usize = 29;
pub const SYS_MKDIRAT: usize = 34;
pub const SYS_UNLINKAT: usize = 35;
pub const SYS_MOUNT: usize = 40;
pub const SYS_UMOUNT2: usize = 39;
pub const SYS_CHDIR: usize = 49;
pub const SYS_READLINKAT: usize = 78;
pub const SYS_OPENAT: usize = 56;
pub const SYS_CLOSE: usize = 57;
pub const SYS_PIPE2: usize = 59;
pub const SYS_GETDENTS64: usize = 61;
pub const SYS_LSEEK: usize = 62;
pub const SYS_READ: usize = 63;
pub const SYS_WRITE: usize = 64;
pub const SYS_FSTAT: usize = 80;
pub const SYS_NEWFSTATAT: usize = 79;
pub const SYS_FACCESSAT: usize = 48;
pub const SYS_STATFS: usize = 43;
pub const SYS_FSTATFS: usize = 44;
pub const SYS_NANOSLEEP: usize = 101;
pub const SYS_SCHED_YIELD: usize = 124;
pub const SYS_TIMES: usize = 153;
pub const SYS_UNAME: usize = 160;
pub const SYS_GETTIMEOFDAY: usize = 169;
pub const SYS_GETPID: usize = 172;
pub const SYS_GETPPID: usize = 173;
pub const SYS_GETTID: usize = 178;
pub const SYS_EXIT: usize = 93;
pub const SYS_BRK: usize = 214;
pub const SYS_MUNMAP: usize = 215;
pub const SYS_CLONE: usize = 220;
pub const SYS_EXECVE: usize = 221;
pub const SYS_MMAP: usize = 222;
pub const SYS_WAIT4: usize = 260;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyscallStatus {
    TraceOnly,
    Implemented,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SyscallInfo {
    pub nr: usize,
    pub name: &'static str,
    pub status: SyscallStatus,
}

pub const BOOTSTRAP_SYSCALLS: &[SyscallInfo] = &[
    SyscallInfo {
        nr: SYS_READ,
        name: "read",
        status: SyscallStatus::Implemented,
    },
    SyscallInfo {
        nr: SYS_OPENAT,
        name: "openat",
        status: SyscallStatus::Implemented,
    },
    SyscallInfo {
        nr: SYS_WRITE,
        name: "write",
        status: SyscallStatus::Implemented,
    },
    SyscallInfo {
        nr: SYS_CLOSE,
        name: "close",
        status: SyscallStatus::Implemented,
    },
    SyscallInfo {
        nr: SYS_DUP,
        name: "dup",
        status: SyscallStatus::Implemented,
    },
    SyscallInfo {
        nr: SYS_DUP3,
        name: "dup3",
        status: SyscallStatus::Implemented,
    },
    SyscallInfo {
        nr: SYS_FCNTL,
        name: "fcntl",
        status: SyscallStatus::Implemented,
    },
    SyscallInfo {
        nr: SYS_IOCTL,
        name: "ioctl",
        status: SyscallStatus::Implemented,
    },
    SyscallInfo {
        nr: SYS_LSEEK,
        name: "lseek",
        status: SyscallStatus::Implemented,
    },
    SyscallInfo {
        nr: SYS_FSTAT,
        name: "fstat",
        status: SyscallStatus::Implemented,
    },
    SyscallInfo {
        nr: SYS_NEWFSTATAT,
        name: "newfstatat",
        status: SyscallStatus::Implemented,
    },
    SyscallInfo {
        nr: SYS_READLINKAT,
        name: "readlinkat",
        status: SyscallStatus::Implemented,
    },
    SyscallInfo {
        nr: SYS_FACCESSAT,
        name: "faccessat",
        status: SyscallStatus::Implemented,
    },
    SyscallInfo {
        nr: SYS_STATFS,
        name: "statfs",
        status: SyscallStatus::Implemented,
    },
    SyscallInfo {
        nr: SYS_FSTATFS,
        name: "fstatfs",
        status: SyscallStatus::Implemented,
    },
    SyscallInfo {
        nr: SYS_GETDENTS64,
        name: "getdents64",
        status: SyscallStatus::Implemented,
    },
    SyscallInfo {
        nr: SYS_EXIT,
        name: "exit",
        status: SyscallStatus::Implemented,
    },
    SyscallInfo {
        nr: SYS_BRK,
        name: "brk",
        status: SyscallStatus::Implemented,
    },
    SyscallInfo {
        nr: SYS_MMAP,
        name: "mmap",
        status: SyscallStatus::Implemented,
    },
    SyscallInfo {
        nr: SYS_MUNMAP,
        name: "munmap",
        status: SyscallStatus::Implemented,
    },
    SyscallInfo {
        nr: SYS_GETTIMEOFDAY,
        name: "gettimeofday",
        status: SyscallStatus::Implemented,
    },
    SyscallInfo {
        nr: SYS_NANOSLEEP,
        name: "nanosleep",
        status: SyscallStatus::Implemented,
    },
    SyscallInfo {
        nr: SYS_UNAME,
        name: "uname",
        status: SyscallStatus::Implemented,
    },
    SyscallInfo {
        nr: SYS_GETPID,
        name: "getpid",
        status: SyscallStatus::Implemented,
    },
    SyscallInfo {
        nr: SYS_GETPPID,
        name: "getppid",
        status: SyscallStatus::Implemented,
    },
    SyscallInfo {
        nr: SYS_SCHED_YIELD,
        name: "sched_yield",
        status: SyscallStatus::Implemented,
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
