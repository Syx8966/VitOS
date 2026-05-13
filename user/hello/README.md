# 用户态 hello ELF

这是阶段 2 使用的最小 Linux 用户态程序。

它不依赖 libc，入口是 `_start`，只执行两个 Linux syscall：

- `write(1, "hello from user\n", 16)`
- `exit(0)`

内核构建 `crates/kernel` 时会通过 `build.rs` 将它分别编译为：

- RISC-V64 ELF
- LoongArch64 ELF

当前阶段只解析 ELF header 和 `PT_LOAD` 段；真正映射到用户地址空间并执行，是后续步骤。
