# 阶段 2：Loader 与 Syscall 骨架

## 当前目标

本阶段先建立“第一个 Linux 用户态 ELF”的前置结构，不直接实现 busybox。当前目标是：

- 在内核入口中初始化 loader 模块。
- 在内核入口中初始化 syscall 模块。
- 明确首批 syscall：`write` 和 `exit`。
- 每次改动后继续保持 RISC-V64 和 LoongArch64 都能启动。

## 已新增模块

```text
apps/oscomp-kernel/src/main.rs       # ArceOS app 入口，只负责启动胶水

crates/kernel/
├── Cargo.toml
└── src/
    ├── lib.rs
    └── loader.rs

crates/linux-abi/
├── Cargo.toml
└── src/
    ├── lib.rs
    └── syscall.rs
```

`crates/kernel/src/loader.rs` 当前只维护占位状态：

```text
ReadyForStaticElf
```

后续会从这里继续实现：

- ELF header 解析
- program header 遍历
- 用户入口地址记录
- 用户栈规划
- 用户地址空间映射

`crates/linux-abi/src/syscall.rs` 当前定义：

```text
SYS_WRITE = 64
SYS_EXIT  = 93
```

状态暂时是 `TraceOnly`，后续会接入 trap/syscall 分发。

## 验证方式

每次修改后执行：

```bash
make all
bash scripts/run-qemu-riscv64.sh
bash scripts/run-qemu-loongarch64.sh
```

预期输出中应包含：

```text
[loader] status = ReadyForStaticElf
[loader] next = parse static ELF headers
[syscall] bootstrap table:
[syscall] nr=64 name=write status=TraceOnly
[syscall] nr=93 name=exit status=TraceOnly
stage2 = loader/syscall scaffold ready
```

## 下一步

下一步不要先接 busybox。先实现一个内置静态 ELF 样例的解析流程：

1. 准备一个最小用户态 hello ELF。
2. 将 ELF 作为 bytes 嵌入或放入 rootfs。
3. 在 `loader` 中解析 ELF header。
4. 打印 entry 和 load segment 信息。
5. 再开始做用户地址空间和 trap 返回。
