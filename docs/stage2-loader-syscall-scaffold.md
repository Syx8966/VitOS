# 阶段 2：Loader 与 Syscall 骨架

## 当前目标

本阶段先建立“第一个 Linux 用户态 ELF”的前置结构，不直接实现 busybox。当前目标是：

- 在内核入口中初始化 loader 模块。
- 在内核入口中初始化 syscall 模块。
- 实现 ELF64 header/program header 的最小解析。
- 构建并嵌入一个真实的最小用户态 hello ELF。
- 启动时解析内置 hello ELF 并打印 entry/LOAD 段。
- 明确首批 syscall：`write` 和 `exit`。
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
    └── loader.rs

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

当前阶段只是构建和解析这个 ELF，尚未映射到用户地址空间执行。

`crates/kernel/src/loader.rs` 当前维护状态：

```text
ReadyForStaticElf
ParsedStaticElf
```

后续会从这里继续实现：

- 真实 ELF bytes 来源
- LOAD 段映射
- 用户栈规划
- trap 返回用户态

`crates/linux-abi/src/syscall.rs` 当前定义：

```text
SYS_WRITE = 64
SYS_EXIT  = 93
```

状态暂时是 `TraceOnly`，后续会接入 trap/syscall 分发。

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
[syscall] nr=64 name=write status=TraceOnly
[syscall] nr=93 name=exit status=TraceOnly
stage2 = loader/syscall scaffold ready
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
[syscall] nr=64 name=write status=TraceOnly
[syscall] nr=93 name=exit status=TraceOnly
stage2 = loader/syscall scaffold ready
```

## 下一步

下一步不要先接 busybox。先把真实 hello ELF 映射到用户地址空间：

1. 使用 `axmm::new_user_aspace` 创建用户地址空间。
2. 按 `PT_LOAD` 段把 ELF 内容复制到用户页。
3. 建立用户栈。
4. 准备 trap 返回上下文。
5. 接入 `write`/`exit` syscall 分发。
