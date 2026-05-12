# Syscall 支持表

来源：`../test/oscomp_syscalls.md`

状态说明：

- `todo`：未实现。
- `stub`：有占位实现，但语义不完整。
- `partial`：能通过部分测例。
- `done`：语义基本满足当前测例。

优先级说明：

- `P0`：最小用户态、busybox 和基础测例必须。
- `P1`：基础测例常用，影响文件系统、进程、时间和内存。
- `P2`：可后续补强。

| 编号 | 名称 | 模块 | 优先级 | 状态 | 备注 |
| --- | --- | --- | --- | --- | --- |
| 17 | getcwd | fs/path | P1 | todo | 当前工作目录 |
| 23 | dup | fs/fd | P1 | todo | fd table |
| 24 | dup3 | fs/fd | P1 | todo | fd table |
| 34 | mkdirat | fs/path | P1 | todo | 目录创建 |
| 35 | unlinkat | fs/path | P1 | todo | 删除文件/目录项 |
| 37 | linkat | fs/path | P2 | todo | 硬链接 |
| 39 | umount2 | fs/mount | P2 | todo | 可先 stub |
| 40 | mount | fs/mount | P2 | todo | 可先支持伪挂载 |
| 49 | chdir | fs/path | P1 | todo | 当前工作目录 |
| 56 | openat | fs/fd | P0 | todo | 文件打开核心 |
| 57 | close | fs/fd | P0 | todo | fd table |
| 59 | pipe2 | fs/pipe | P1 | todo | shell/IPC 常用 |
| 61 | getdents64 | fs/dir | P1 | todo | ls/目录遍历 |
| 63 | read | fs/fd | P0 | todo | 最小 IO |
| 64 | write | fs/fd | P0 | todo | hello 输出 |
| 80 | fstat | fs/fd | P1 | todo | libc/busybox 常用 |
| 93 | exit | process | P0 | todo | 进程退出 |
| 101 | nanosleep | time | P1 | todo | 时间相关测例 |
| 124 | sched_yield | sched | P1 | todo | 可先让出 CPU |
| 153 | times | time | P1 | todo | 进程时间 |
| 160 | uname | sys | P1 | todo | busybox/libc 常用 |
| 169 | gettimeofday | time | P1 | todo | 时间相关测例 |
| 172 | getpid | process | P0 | todo | 进程 ID |
| 173 | getppid | process | P1 | todo | 父进程 ID |
| 214 | brk | mm | P0 | todo | libc 堆 |
| 215 | munmap | mm | P1 | todo | mmap 释放 |
| 220 | clone | process | P0 | todo | fork/thread 基础 |
| 221 | execve | process/elf | P0 | todo | 执行测试程序 |
| 222 | mmap | mm | P0 | todo | ELF/libc 映射 |
| 260 | wait4 | process | P0 | todo | runner 等待子进程 |

## 第一阶段实现顺序

1. `write`、`exit`
2. `brk`、`mmap`、`munmap`
3. `openat`、`read`、`close`、`fstat`
4. `execve`、`clone`、`wait4`、`getpid`
5. `getcwd`、`chdir`、`getdents64`
6. `pipe2`、`dup`、`dup3`
7. `gettimeofday`、`nanosleep`、`times`、`uname`
