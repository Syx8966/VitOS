# 阶段 1：最小内核应用

## 当前状态

阶段 1 已将根目录构建入口从 ArceOS `examples/helloworld` 切换到本队应用：

```text
apps/oscomp-kernel/
├── Cargo.toml
└── src/main.rs
```

`make all` 当前会生成：

```text
kernel-rv      # 赛方 RISC-V64 ELF 产物
kernel-la      # 赛方 LoongArch64 ELF 产物
kernel-rv.bin  # 本地 RISC-V64 QEMU 验证用
kernel-la.bin  # 本地备用二进制
```

注意：赛方要求的是 `kernel-rv` 和 `kernel-la`。`.bin` 文件只是为了本地启动验证。

## 每次改代码后的验证流程

在项目根目录执行：

```bash
make all
file kernel-rv kernel-la
bash scripts/run-qemu-riscv64.sh
```

RISC-V64 QEMU 中应看到：

```text
OSKernel-ArceOS starting
stage = 1
base_os = ArceOS
arch = riscv64
status = minimal app booted
```

LoongArch64 构建验证：

```bash
file kernel-la
```

应看到：

```text
kernel-la: ELF 64-bit LSB executable, LoongArch
```

本地 LoongArch64 QEMU 脚本：

```bash
bash scripts/run-qemu-loongarch64.sh
```

LoongArch64 启动验证要求 `qemu-system-loongarch64` 为 9.2.1。Ubuntu apt 安装的 8.2.2 会出现启动无输出等问题，不适合作为比赛环境验证依据。

LoongArch64 QEMU 中应看到：

```text
OSKernel-ArceOS starting
stage = 1
base_os = ArceOS
arch = loongarch64
status = minimal app booted
```

## 合格标准

每次提交前至少满足：

- `make all` 成功。
- `kernel-rv` 是 RISC-V64 ELF。
- `kernel-la` 是 LoongArch64 ELF。
- `scripts/run-qemu-riscv64.sh` 能启动并输出阶段 1 banner。
- `scripts/run-qemu-loongarch64.sh` 在 QEMU 9.2.1 下能启动并输出阶段 1 banner。

## 下一步

下一步开始实现“第一个 Linux 用户态 ELF”的加载前置能力：

1. 在 `apps/oscomp-kernel` 中建立模块结构。
2. 增加 `loader` 占位模块，用于后续 ELF 解析。
3. 增加 `syscall` 占位模块，先定义 `write/exit` 编号和 trace 输出格式。
4. 保持每次修改后都能通过本文件的验证流程。
