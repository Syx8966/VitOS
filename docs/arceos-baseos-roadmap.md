# ArceOS BaseOS 参赛路线

## 当前定位

本项目以 ArceOS 作为 BaseOS，目标是在 ArceOS 的启动、HAL、任务调度、内存管理、设备驱动和组件体系之上，实现面向系统能力大赛 OS 内核实现赛道的 Linux ABI 兼容内核。

当前重点不是重写 ArceOS，而是补齐比赛测例需要的用户态运行环境：

- Linux ELF 加载和用户态执行。
- Linux syscall 兼容层。
- 用户地址空间、`brk`、`mmap`、用户内存访问。
- EXT4 测试盘读取。
- 从测试盘读取并执行 basic 用户程序。
- RISC-V64 与 LoongArch64 双架构构建和运行。

## 当前目录结构

当前仓库采用“ArceOS 作为 third_party，比赛代码放在 crates”的结构：

```text
oskernel-arceos/
├── Makefile                         # 赛方入口：make all 生成 kernel-rv / kernel-la
├── apps/
│   └── oscomp-kernel/               # ArceOS app 入口，只做启动胶水
├── crates/
│   ├── kernel/                      # 比赛内核核心：ELF、runtime、runner、EXT4 smoke
│   └── linux-abi/                   # syscall 编号、errno、ABI 常量
├── docs/
│   └── arceos-baseos-roadmap.md
├── scripts/
│   ├── run-qemu-riscv64.sh
│   └── run-qemu-loongarch64.sh
├── third_party/
│   ├── arceos/                      # ArceOS BaseOS
│   └── reference/                   # 参考实现
├── user/
│   ├── hello/                       # 最小用户态 hello
│   └── local-basic/                 # 本地 syscall smoke 用户程序
├── kernel-rv                        # RISC-V64 赛方产物
├── kernel-rv.bin                    # RISC-V64 QEMU binary
├── kernel-la                        # LoongArch64 赛方产物
└── kernel-la.bin                    # LoongArch64 QEMU binary
```

后续仍建议坚持这个边界：

- `apps/oscomp-kernel` 只保留 ArceOS 应用入口和初始化胶水。
- 真正比赛逻辑写在 `crates/kernel` 和后续拆出的子 crate 中。
- syscall 编号、errno、结构体布局等 ABI 内容放在 `crates/linux-abi`。
- 不直接大改 `third_party/arceos`，除非遇到 BaseOS 无法绕开的架构问题。

## 已完成进度

### 阶段 0：构建入口和 BaseOS 选型

状态：已完成。

已完成内容：

- ArceOS 已作为 BaseOS 放入 `third_party/arceos`。
- 根目录 `Makefile` 已接入赛方入口。
- `make all` 能生成：
  - `kernel-rv`
  - `kernel-rv.bin`
  - `kernel-la`
  - `kernel-la.bin`
- RISC-V64 和 LoongArch64 两个架构都能完成构建。

验证方式：

```bash
make all
```

成功标准：根目录出现 `kernel-rv` 和 `kernel-la`，构建过程无错误。

### 阶段 1：最小用户态 ELF

状态：已完成。

已完成内容：

- 能加载静态用户 ELF。
- 能创建用户地址空间。
- 能映射 ELF `PT_LOAD` 段。
- 能进入用户态执行。
- 能处理最小 syscall：`write`、`exit`。
- RISC-V64 和 LoongArch64 都能运行 embedded hello。

验证方式：

```bash
bash scripts/run-qemu-riscv64.sh
bash scripts/run-qemu-loongarch64.sh
```

成功标准：日志中出现用户态 hello 输出，并打印 `embedded-hello exit_code=0`。

### 阶段 2：基础 syscall smoke

状态：已完成本地 smoke，尚未覆盖真实赛方全部 syscall。

已完成内容：

- 实现了基础用户 runtime。
- 增加 `user/local-basic` 本地用户程序。
- 当前已支持或具备最小实现的 syscall 包括：
  - `write`
  - `read`：stdin EOF 语义
  - `close`
  - `fstat`
  - `exit`
  - `brk`
  - `mmap`
  - `munmap`
  - `gettimeofday`
  - `nanosleep`
  - `times`
  - `uname`
  - `getpid`
  - `getppid`
  - `sched_yield`
  - `mount` / `umount2` stub
- `local-basic` 可以在两个架构上跑完：
  - `brk`
  - `mmap/munmap`
  - `gettimeofday`
  - `uname`
  - `sched_yield`
  - `nanosleep`
  - `fstat`
  - `exit`

验证方式：

```bash
make all
bash scripts/run-qemu-riscv64.sh
bash scripts/run-qemu-loongarch64.sh
```

成功标准：日志中出现：

```text
local-basic: syscall smoke ok
[runner] local-basic exit_code=0
```

### 阶段 3：EXT4 读取和从盘执行 ELF

状态：本地闭环已完成。

已完成内容：

- QEMU 脚本支持通过 `TEST_IMG` 挂载测试镜像：
  - RISC-V64 使用 virtio-blk-mmio。
  - LoongArch64 使用 virtio-blk-pci。
- `crates/kernel/src/testdisk.rs` 能读取无分区 EXT4 镜像。
- 已支持：
  - EXT4 superblock 解析。
  - group descriptor 解析。
  - inode 读取。
  - extent tree 递归读取。
  - 目录项查找。
  - 小型普通文件读取。
- 能查找并读取：
  - `/musl/basic/write`
  - `/musl/basic/brk`
  - `/glibc/basic/write`
- runner 能把 `/musl/basic/write` 读出的 ELF 加载并执行。
- RISC-V64 和 LoongArch64 都已用本地 EXT4 镜像验证成功。

本地 EXT4 镜像位置：

```text
/tmp/vitos-rv-ext4.img
/tmp/vitos-la-ext4.img
```

验证方式：

```bash
make all
TEST_IMG=/tmp/vitos-rv-ext4.img bash scripts/run-qemu-riscv64.sh
TEST_IMG=/tmp/vitos-la-ext4.img bash scripts/run-qemu-loongarch64.sh
```

成功标准：

```text
[testdisk] EXT4 detected block_size=4096 ...
[testdisk] read /musl/basic/write size=...
[runner] running ext4-musl-basic-write
local-basic: syscall smoke ok
[runner] ext4-musl-basic-write exit_code=0
```

阶段 3 的边界：

- 已证明“读取 EXT4 文件 -> 解析 ELF -> 进入用户态执行”链路可用。
- 尚未证明能通过赛方真实 basic 全量测试。
- 当前 EXT4 读取器仍是早期实现，不是完整 VFS/ext4 文件系统。

## 当前结论

阶段 0、1、2、3 的基础目标已经完成。

接下来应进入阶段 4，但阶段 4 不建议先做复杂调度、信号或 LTP。当前最合理的方向是先完善 syscall，因为真实 basic 测试是否能跑起来，主要取决于文件、进程、目录和内存相关 syscall 是否具备 Linux 兼容语义。

## 阶段 4：真实 basic 测试与 syscall 完善

### 阶段目标

阶段 4 的目标是：接入赛方真实 EXT4 basic 测试盘，能够枚举并执行 `/musl/basic/*`，按失败日志补 syscall，最终让 musl basic 测例稳定通过，再扩展到 glibc basic。

优先级：

1. 先跑真实 basic。
2. 先补 syscall。
3. 先通过 musl basic。
4. 再处理 glibc basic。
5. 最后再进入 LTP 和高阶功能。

## 阶段 4.1：接入真实 basic 测试盘

要做的事：

- 获取赛方真实 EXT4 测试镜像。
- 用 `TEST_IMG=...` 挂到 QEMU。
- 在当前 runner 中先固定执行少量路径：
  - `/musl/basic/write`
  - `/musl/basic/brk`
  - `/musl/basic/mmap`
  - `/musl/basic/open`
  - `/musl/basic/read`
- 将当前 `testdisk` 的路径读取能力扩展为目录枚举能力，为后续自动扫描 `/musl/basic` 做准备。

验证方式：

```bash
make all
TEST_IMG=/path/to/real-basic-rv.img bash scripts/run-qemu-riscv64.sh
TEST_IMG=/path/to/real-basic-la.img bash scripts/run-qemu-loongarch64.sh
```

成功标准：

- 能识别真实 EXT4 镜像。
- 能读出 `/musl/basic` 目录。
- 至少能执行一个真实 basic ELF。
- 失败时能打印清楚是哪个 syscall 未实现或哪个语义不兼容。

## 阶段 4.2：优先完善文件相关 syscall

这是下一步最应该做的部分。

当前 `read` 只有 stdin EOF 语义，不足以跑真实测试。真实 basic 很快会需要 fd table 和文件读写。

优先实现：

- `openat`
- `read`
- `write`
- `close`
- `fstat`
- `lseek`
- `getdents64`
- `fcntl`
- `ioctl` 的常见 stub
- `readlinkat`
- `newfstatat` / `fstatat`

推荐实现方式：

- 在 `crates/kernel` 中先做最小 fd table。
- fd `0/1/2` 映射到 console。
- 普通文件 fd 映射到从 EXT4 读取出的内存文件。
- 目录 fd 支持 `getdents64` 返回目录项。
- 暂时只支持只读 EXT4，先不做写回。

第一版目标不是完整 VFS，而是能满足 basic：

```text
openat(path) -> fd
read(fd) -> file bytes
fstat(fd) -> regular file / directory metadata
getdents64(fd) -> directory entries
close(fd) -> release fd
```

验证标准：

- 真实 `/musl/basic/open` 能执行。
- 真实 `/musl/basic/read` 能执行。
- 用户程序中访问普通文件不会返回 `-ENOSYS`。

## 阶段 4.3：完善进程相关 syscall

basic 中经常会出现进程和等待相关测试。当前 runtime 还不是完整 Linux 进程模型，需要先实现最小语义。

优先实现：

- `execve`
- `wait4`
- `clone`
- `exit_group`
- `set_tid_address`
- `gettid`
- `tgkill` stub

推荐最小策略：

- `execve` 第一版只支持从 EXT4 读取目标 ELF，并在当前任务中替换用户地址空间。
- `wait4` 第一版支持等待由 `clone` 或 runner 创建的子任务退出。
- `clone` 第一版先支持 basic 常用 flags，不追求完整线程语义。
- `exit_group` 可以先等价于当前进程退出。

验证标准：

- 能跑涉及 fork/clone/wait 的 basic 测例。
- 子进程退出码能被父进程取到。
- 不因为 `execve` 直接返回 `-ENOSYS` 卡住测试脚本。

## 阶段 4.4：完善内存和时间相关 syscall

当前已有 `brk`、`mmap`、`munmap`、`gettimeofday`、`nanosleep` 的基础实现，但真实测试可能会检查边界。

需要补强：

- `mmap` flags/prot 检查。
- 匿名映射和固定地址映射。
- `munmap` 局部释放。
- `mprotect`
- `clock_gettime`
- `times`
- `getrusage`

验证标准：

- `/musl/basic/brk`
- `/musl/basic/mmap`
- `/musl/basic/time`
- 相关本地 regression 都能通过。

## 阶段 4.5：runner 自动化

当前 runner 仍偏手工，只固定执行 `ext4-musl-basic-write`。

后续要做：

- 枚举 `/musl/basic` 下的 ELF。
- 按固定顺序串行执行。
- 对每个测试打印开始、结束和退出码。
- 单个测试失败不能影响后续测试继续运行。
- 最后输出 summary。

推荐输出格式：

```text
#### OS COMP TEST GROUP START basic ####
[runner] START /musl/basic/write
[runner] PASS  /musl/basic/write exit_code=0
[runner] START /musl/basic/brk
[runner] PASS  /musl/basic/brk exit_code=0
[runner] SUMMARY pass=... fail=...
#### OS COMP TEST GROUP END basic ####
```

验证标准：

- 挂真实 basic 镜像后，不需要手动改路径就能跑 `/musl/basic`。
- 失败用例能从日志直接定位。

## 阶段 5：glibc basic 和 LTP

阶段 5 不建议现在立刻进入。

进入条件：

- musl basic 中大部分文件、内存、进程测试已经通过。
- syscall 失败日志可分类。
- runner 已能自动扫描测试目录。

阶段 5 目标：

- 扩展到 `/glibc/basic`。
- 补齐 glibc 更常用的 TLS、线程、futex、signal 相关 syscall。
- 开始跑 LTP 高价值子集。

重点 syscall：

- `futex`
- `rt_sigaction`
- `rt_sigprocmask`
- `rt_sigreturn`
- `kill`
- `clock_nanosleep`
- `poll`
- `ppoll`
- `select`
- `epoll_create1`
- `epoll_ctl`
- `epoll_wait`

## 下一步执行清单

当前建议立即执行下面顺序。

### 1. 建立 fd table

位置建议：

```text
crates/kernel/src/fd.rs
```

第一版支持：

- console fd：`0/1/2`
- regular file fd
- directory fd
- fd 分配、查询、关闭

### 2. 把 EXT4 读取器接到 fd table

第一版可以先做只读内存文件：

- `openat("/musl/basic/write")` 读取整个文件到内存。
- `read(fd)` 从内存 buffer 按 offset 返回。
- `lseek(fd)` 修改 offset。
- `close(fd)` 释放 fd。

这比一开始写完整 page cache/VFS 更适合当前阶段。

### 3. 实现 `openat/read/lseek/getdents64`

优先让 basic 文件类测试不再卡在 `-ENOSYS`。

最小验收：

```text
/musl/basic/open
/musl/basic/read
/musl/basic/write
```

### 4. 增强 runner

先不要直接跑全目录，建议先固定路径数组：

```text
/musl/basic/write
/musl/basic/brk
/musl/basic/mmap
/musl/basic/open
/musl/basic/read
```

等这几个稳定后，再做目录枚举和自动扫描。

### 5. 再补进程 syscall

文件 syscall 打通后，再做：

- `execve`
- `wait4`
- `clone`
- `exit_group`

这样能避免在文件路径还不可用时调试进程模型，降低复杂度。

## 每次编写后的验证方式

每次改 syscall 后都按三层验证。

### 第一层：编译和单测

```bash
cargo test --manifest-path crates/kernel/Cargo.toml
make all
```

成功标准：

- Rust 单测通过。
- `kernel-rv` 和 `kernel-la` 都生成。

### 第二层：无盘启动

```bash
bash scripts/run-qemu-riscv64.sh
bash scripts/run-qemu-loongarch64.sh
```

成功标准：

```text
[runner] embedded-hello exit_code=0
[runner] local-basic exit_code=0
```

### 第三层：EXT4 盘启动

```bash
TEST_IMG=/tmp/vitos-rv-ext4.img bash scripts/run-qemu-riscv64.sh
TEST_IMG=/tmp/vitos-la-ext4.img bash scripts/run-qemu-loongarch64.sh
```

成功标准：

```text
[testdisk] EXT4 detected ...
[testdisk] read /musl/basic/write size=...
[runner] ext4-musl-basic-write exit_code=0
```

接入真实 basic 镜像后，把 `/tmp/vitos-*.img` 替换成真实镜像路径。

## 当前最重要的判断

现在不应该急着进入 LTP，也不应该先写复杂文件系统。

最有效的下一步是：

```text
完善 syscall，尤其是 openat/read/lseek/getdents64/fstat 这一组文件相关 syscall。
```

原因：

- EXT4 读取和从盘执行 ELF 已经打通。
- 真实 basic 测试接下来最容易卡在文件 fd、目录枚举和进程 syscall。
- 先补文件 syscall，可以最快扩大可运行 basic 测例数量。
- 有真实失败日志后，再补 `execve/wait4/clone` 会更稳。
