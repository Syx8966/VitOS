# 阶段 2：Loader 与 Syscall 骨架

## 当前目标

本阶段先跑通“第一个 Linux 用户态 ELF”，不直接实现 busybox。当前目标是：

- 在内核入口中初始化 loader 模块。
- 在内核入口中初始化 syscall 模块。
- 实现 ELF64 header/program header 的最小解析。
- 构建并嵌入一个真实的最小用户态 hello ELF。
- 启动时解析内置 hello ELF 并打印 entry/LOAD 段。
- 明确首批 syscall：`write` 和 `exit`。
- 将 hello ELF 映射到用户地址空间并从内核态切到用户态。
- 通过 syscall trap 实现最小 `write`/`exit`。
- 每次改动后继续保持 RISC-V64 和 LoongArch64 都能启动。

## 已新增模块

```text
apps/oscomp-kernel/src/main.rs       # ArceOS app 入口，只负责启动胶水

crates/kernel/
├── Cargo.toml
├── build.rs
└── src/
    ├── elf.rs
    ├── lib.rs
    ├── loader.rs
    └── runtime.rs

crates/linux-abi/
├── Cargo.toml
└── src/
    ├── lib.rs
    └── syscall.rs

user/hello/
├── README.md
├── linker.ld
└── src/main.rs
```

`crates/kernel/src/elf.rs` 当前支持：

- 校验 ELF magic。
- 校验 ELF64。
- 校验 little-endian。
- 支持 `ET_EXEC`。
- 支持 `EM_RISCV` / `EM_LOONGARCH`。
- 遍历 program header。
- 收集最多 8 个 `PT_LOAD` 段。
- 解析由 `crates/kernel/build.rs` 生成并嵌入的真实用户态 hello ELF。

`user/hello/src/main.rs` 当前是无 libc 的最小用户程序，入口 `_start` 只做：

```text
write(1, "hello from user\n", 16)
exit(0)
```

当前阶段已经能把这个 ELF 映射到用户地址空间执行，并通过 `write` 输出：

```text
hello from user
```

`crates/kernel/src/loader.rs` 当前维护状态：

```text
ReadyForStaticElf
ParsedStaticElf
```

`crates/kernel/src/runtime.rs` 当前实现：

- 使用 `axmm::new_user_aspace` 创建用户地址空间。
- 按 `PT_LOAD` 段映射 ELF。
- 建立最小用户栈。
- 使用 `axhal::context::UspaceContext` trap 返回用户态。
- 注册 `SYSCALL` handler。
- 支持最小 `write`/`exit`。

`crates/linux-abi/src/syscall.rs` 当前定义：

```text
SYS_WRITE = 64
SYS_EXIT  = 93
```

状态是最小可用实现，当前只覆盖内置 hello ELF 的 `write`/`exit`。

## 验证方式

每次修改后执行：

```bash
cargo test --manifest-path crates/kernel/Cargo.toml
make all
bash scripts/run-qemu-riscv64.sh
bash scripts/run-qemu-loongarch64.sh
```

预期输出中应包含：

RISC-V64：

```text
[loader] status = ReadyForStaticElf
[loader] next = parse static ELF headers
[elf] entry=0x400000 phnum=3 load_segments=2
[elf] load[0] off=0x1000 vaddr=0x400000 filesz=0x20 memsz=0x20 flags=0x5
[elf] load[1] off=0x2000 vaddr=0x401000 filesz=0x10 memsz=0x10 flags=0x4
[loader] status = ParsedStaticElf entry=0x400000 load_segments=2
[syscall] bootstrap table:
[syscall] nr=64 name=write status=Implemented
[syscall] nr=93 name=exit status=Implemented
stage2 = loader/syscall scaffold ready
[runtime] user image ready entry=0x400000
[runtime] enter user
hello from user
[syscall] exit(0)
```

LoongArch64：

```text
[loader] status = ReadyForStaticElf
[loader] next = parse static ELF headers
[elf] entry=0x400000 phnum=2 load_segments=2
[elf] load[0] off=0x1000 vaddr=0x400000 filesz=0x28 memsz=0x28 flags=0x5
[elf] load[1] off=0x2000 vaddr=0x401000 filesz=0x10 memsz=0x10 flags=0x4
[loader] status = ParsedStaticElf entry=0x400000 load_segments=2
[syscall] bootstrap table:
[syscall] nr=64 name=write status=Implemented
[syscall] nr=93 name=exit status=Implemented
stage2 = loader/syscall scaffold ready
[runtime] user image ready entry=0x400000
[runtime] enter user
hello from user
[syscall] exit(0)
```

## 下一步

阶段 2 已完成最小闭环。下一步进入 busybox 前，建议先把单程序运行能力补到“可扩展”：

1. 把 `runtime` 拆成 `process` / `fd` / `syscall` 子模块。
2. 实现 `brk`、`mmap`、`munmap`。
3. 实现最小 fd table。
4. 准备 rootfs/EXT4 入口。
5. 再接 `openat`、`read`、`close`、`fstat`。
