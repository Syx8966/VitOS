# 阶段 0：ArceOS BaseOS 拉取与启动准备

## 阶段目标

阶段 0 不修改内核逻辑，目标是把参赛工程的来源、目录、构建入口和第一批测试目标固定下来。

完成阶段 0 后应具备：

- ArceOS 上游源码已固定在 `third_party/arceos`。
- StarryOS/starry-next 参考源码已固定在 `third_party/reference/starry-next`。
- 明确 `make all` 最终必须生成 `kernel-rv` 和 `kernel-la`。
- 能在本地尝试运行 ArceOS RISC-V64 QEMU 示例。
- 能从 `../test/oscomp_syscalls.md` 梳理第一版 syscall 支持表。

## 拉取哪些部分

### 必须拉取：ArceOS 完整仓库

路径：

```text
third_party/arceos/
```

来源：

```text
https://github.com/arceos-org/arceos.git
```

原因：

- `modules/`、`crates/`、`api/`、`ulib/` 是组件代码。
- `Makefile`、`scripts/`、`configs/` 参与配置生成、平台选择和 QEMU 启动。
- `examples/` 提供最小可运行应用，用于验证工具链。

不要只拷贝 `modules/` 或某几个 crate。阶段 0 应保持 ArceOS 上游完整，后续再通过依赖和适配层复用组件。

当前已拉取版本：

```text
8d2e6f97efc4a67359b99db5e519a55b7067b524
```

### 参考拉取：StarryOS/starry-next

路径：

```text
third_party/reference/starry-next/
```

来源：

```text
https://www.gitlink.org.cn/oscomp/starry-next.git
```

用途：

- 参考 OSComp 场景下的 Linux syscall 组织方式。
- 参考用户程序加载、测试入口、文件系统和 runner 设计。
- 不作为主构建依赖，不直接复制大段代码。

## 已执行的拉取命令

```bash
mkdir -p oskernel-arceos/third_party/reference
cd oskernel-arceos
git clone --depth 1 https://github.com/arceos-org/arceos.git third_party/arceos
git clone --depth 1 https://www.gitlink.org.cn/oscomp/starry-next.git third_party/reference/starry-next
```

如果后续希望固定为 submodule，可在干净仓库中改用：

```bash
git submodule add https://github.com/arceos-org/arceos.git third_party/arceos
git submodule add https://www.gitlink.org.cn/oscomp/starry-next.git third_party/reference/starry-next
git submodule update --init --recursive
```

## 下一步具体命令

### 1. 检查 ArceOS 示例

```bash
cd oskernel-arceos/third_party/arceos
make A=examples/helloworld ARCH=riscv64 LOG=info
```

也可以从项目根目录运行包装目标：

```bash
cd oskernel-arceos
make setup-deps
make arceos-helloworld-rv
```

如果本机已有 QEMU，可以继续运行：

```bash
make A=examples/helloworld ARCH=riscv64 LOG=info run
```

或：

```bash
cd oskernel-arceos
make arceos-helloworld-rv-run
```

预期结果：

- 能生成 RISC-V64 镜像。
- QEMU 中能看到 helloworld 输出。

如果缺少依赖，按报错补齐。常见依赖包括：

- Rust nightly 与 `riscv64gc-unknown-none-elf`
- `cargo-binutils`
- `axconfig-gen`
- `cargo-axplat`
- `qemu-system-riscv64`

### 2. 确认赛方输出入口

根目录 `Makefile` 后续必须支持：

```bash
make all
```

最终产物必须是：

```text
kernel-rv
kernel-la
```

可选产物：

```text
disk.img
disk-la.img
```

阶段 0 先只保留占位 Makefile，等最小内核应用确定后再接入真实构建。

### 3. 梳理 syscall 表

输入文档：

```text
../test/oscomp_syscalls.md
```

输出文档：

```text
docs/syscall-table.md
```

第一版只需要记录：

```text
syscall 编号 | 名称 | 模块 | 优先级 | 当前状态 | 备注
```

优先级建议：

- P0：启动 busybox、基础测例、hello/init 必须。
- P1：文件系统、进程、mmap、信号、futex 的常见测例。
- P2：网络、性能测试、复杂 LTP 场景。

### 4. 先固定 RISC-V64 QEMU 命令

后续创建：

```text
scripts/run-qemu-riscv64.sh
```

它应该尽量贴近赛方 README 中的 QEMU 形态：

```text
qemu-system-riscv64 -machine virt -kernel kernel-rv -m <mem> -nographic -smp <smp> -bios default ...
```

阶段 0 可以先调用 ArceOS 自带 `make ... run`，但阶段 1 开始必须逐步切换到赛方 QEMU 参数。

## 阶段 0 验收清单

- [x] 创建 `oskernel-arceos/` 工程目录。
- [x] 拉取 `third_party/arceos`。
- [x] 拉取 `third_party/reference/starry-next`。
- [x] 记录 ArceOS commit id。
- [x] 创建根目录阶段 0 Makefile 占位目标。
- [x] 跑通 `make A=examples/helloworld ARCH=riscv64 LOG=info`。
- [x] 跑通 `make A=examples/helloworld ARCH=riscv64 LOG=info run`。
- [x] 生成第一版 `docs/syscall-table.md`。
- [ ] 根目录 `Makefile` 接入真实 `kernel-rv` / `kernel-la` 构建。

## 阶段 0 之后进入阶段 1

阶段 1 的唯一目标：

```text
ArceOS BaseOS + RISC-V64 QEMU + 第一个 Linux 用户态 ELF + write/exit syscall trace
```

在这个目标完成前，不建议投入 LoongArch64、开发板、gcc/rustc、复杂网络或完整 LTP。
