# ArceOS BaseOS 参赛路线

## 目标定位

本项目将 ArceOS 设为 BaseOS，目标不是实现一个普通 unikernel，而是在 ArceOS 组件化能力之上构建面向系统能力大赛 OS 内核实现赛道的 Linux 兼容宏内核。

核心目标：

- 复用 ArceOS 的 HAL、任务、内存、文件系统、驱动和组件化组织方式。
- 自行实现比赛需要的 Linux ABI、进程模型、用户地址空间、ELF 加载、VFS/Ext4、信号和 futex。
- 优先通过赛方基础测例和 LTP 高价值测例。
- 支持 RISC-V64 和 LoongArch64，先 QEMU，后开发板。

## 推荐目录结构

采用 `third_party/arceos` 作为上游底座，比赛代码放在独立 workspace crates 中。这样能明确区分“BaseOS 复用部分”和“参赛实现部分”，也方便后续同步 ArceOS 上游与撰写答辩文档。

```text
oskernel-arceos/
├── README.md
├── Cargo.toml
├── Makefile                     # 赛方入口：make all 生成 kernel-rv 和 kernel-la
├── rust-toolchain.toml
├── .cargo/
│   └── config.toml
│
├── third_party/
│   └── arceos/                 # ArceOS BaseOS，上游代码，建议用 git submodule
│
├── crates/
│   ├── kernel/                 # 比赛内核入口与全局初始化
│   ├── linux-abi/              # Linux syscall、errno、auxv、信号编号、用户 ABI
│   ├── test-runner/            # 扫描评测盘并串行执行 *_testcode.sh
│   ├── process/                # 进程、线程、地址空间、exec、wait、clone/fork
│   ├── mm/                     # 用户内存、mmap、brk、页权限、copy_from_user
│   ├── vfs/                    # VFS、fd table、path lookup、mount
│   ├── fs-ext4/                # Ext4 适配层
│   ├── fs-pseudo/              # devfs、procfs、sysfs、tmpfs
│   ├── signal/                 # Linux 信号语义
│   ├── futex/                  # futex 与用户态同步
│   ├── arch/                   # RISC-V64 / LoongArch64 差异封装
│   └── drivers/                # 比赛需要的块设备、串口、网卡、板级适配
│
├── apps/
│   └── init/                   # 第一个用户态 init 程序或启动脚本
│
├── rootfs/
│   ├── busybox/                # busybox 构建产物或配置
│   ├── ltp/                    # LTP 根文件系统内容
│   └── image/                  # 生成的 rootfs 镜像，不建议提交大文件
│
├── tests/
│   ├── smoke/                  # 最小启动、hello、busybox 等冒烟测试
│   ├── syscall/                # 单 syscall 回归测试
│   ├── ltp/                    # LTP 运行配置和结果解析
│   └── results/                # 本地跑分日志，必要时只提交摘要
│
├── scripts/
│   ├── setup-toolchain.sh
│   ├── build.sh
│   ├── make-rootfs.sh
│   ├── run-qemu-riscv64.sh
│   ├── run-qemu-loongarch64.sh
│   ├── run-ltp.sh
│   ├── gdb-riscv64.sh
│   ├── gdb-loongarch64.sh
│   └── collect-results.sh
│
├── configs/
│   ├── riscv64-qemu.toml
│   ├── loongarch64-qemu.toml
│   ├── riscv64-board.toml
│   ├── loongarch64-board.toml
│   └── features.toml
│
├── docs/
│   ├── arceos-baseos-roadmap.md
│   ├── design.md
│   ├── syscall-table.md
│   ├── ltp-report.md
│   ├── riscv64-porting.md
│   ├── loongarch64-porting.md
│   └── final-report.md
│
└── tools/
    ├── syscall-trace/
    ├── log-analyzer/
    └── rootfs-builder/
```

## 为什么选这个结构

### 1. ArceOS 放在 `third_party/`

ArceOS 是 BaseOS，不应和参赛业务代码混在同一层。放在 `third_party/arceos` 有三个好处：

- 后续可以用 submodule 固定版本。
- 可以清楚说明哪些能力来自 ArceOS，哪些是团队实现。
- 同步上游或替换组件时更容易控制影响范围。

### 2. 比赛功能按 crate 拆分

比赛真正的工作量集中在 Linux 兼容层，而不是启动一个内核。`linux-abi`、`process`、`mm`、`vfs`、`signal`、`futex` 分开后，每个成员可以独立负责一个方向。

建议团队分工：

- 成员 A：构建系统、QEMU、rootfs、LTP、日志分析。
- 成员 B：Linux syscall、进程线程、ELF、wait/exec/clone。
- 成员 C：VFS、Ext4、伪文件系统、fd table、路径解析。
- 成员 D：RISC-V64/LoongArch64 HAL 差异、trap、上下文切换、开发板。

### 3. `tests/` 和 `scripts/` 一开始就建立

OS 内核赛不是“功能写完再测试”。LTP 和真实 Linux 程序兼容性会反向决定 syscall、VFS 和内存语义的优先级。

从第一周开始就应该让每次提交能回答三个问题：

- 现在能启动到哪里？
- 新增通过了哪些测例？
- 失败测例卡在哪个 syscall 或内核模块？

### 4. 根目录必须保留赛方评测入口

赛方评测会在项目根目录执行 `make all`，并期望生成两个 ELF 内核文件：

- `kernel-rv`：RISC-V64 QEMU 使用。
- `kernel-la`：LoongArch64 QEMU 使用。

如果需要附加磁盘镜像，可以生成：

- `disk.img`：RISC-V64 附加镜像。
- `disk-la.img`：LoongArch64 附加镜像。

系统启动后需要主动扫描评测 EXT4 磁盘，找到并串行执行根目录下的 `*_testcode.sh` 脚本。脚本前后的输出格式会影响评分，因此建议单独设置 `crates/test-runner`，不要把这部分逻辑散落在 init 或 syscall 实现里。

## 开发路线

### 阶段 0：准备与选型确认

目标：确定 ArceOS 版本、工具链和测试入口。

任务：

- 拉取 ArceOS 到 `third_party/arceos`。
- 拉取或参考 StarryOS/starry-next 的 Linux 兼容实现。
- 跑通 ArceOS 官方 RISC-V64 QEMU 示例。
- 阅读赛方 `test/` 目录，确认构建方式、测试输入和提交格式。
- 实现根目录 `Makefile` 的空壳目标，先固定 `kernel-rv` / `kernel-la` 输出路径。
- 建立 `docs/syscall-table.md`，记录每个 syscall 的状态。

验收标准：

- 一条命令能构建并启动 ArceOS 示例。
- `make all` 的输出路径符合赛方要求。
- 明确当前比赛基础测例需要哪些 syscall。
- 文档中写明 BaseOS 版本、commit id 和本队扩展范围。

### 阶段 1：最小 Linux 用户程序

目标：在 ArceOS 上跑第一个 Linux 用户态 ELF。

任务：

- 实现用户地址空间。
- 实现 ELF 加载。
- 实现用户态 trap 返回。
- 实现最小 syscall：`write`、`exit`、`brk`、`mmap`。
- 支持静态链接 hello 程序输出。

验收标准：

- `run-qemu-riscv64.sh` 能启动内核并执行用户态 hello。
- syscall trace 能打印 syscall 编号、参数和返回值。

### 阶段 2：busybox 与基础文件系统

目标：跑起 busybox shell。

任务：

- 实现 fd table。
- 实现 `openat/read/write/close/fstat/lseek/getdents64`。
- 接入 VFS 和 rootfs。
- 实现 `execve/wait4/getpid/getppid/clone` 的基础语义。
- 补 `/dev/null`、`/dev/zero`、`/proc` 的最小实现。

验收标准：

- 能进入 busybox `sh`。
- 能运行 `ls`、`cat`、`echo`、`mkdir`、`rm` 等基础命令。

### 阶段 3：赛方基础测例

目标：稳定通过赛方基础测例，形成自动化回归。

任务：

- 把赛方测试集接入 `scripts/run-ltp.sh` 或单独 runner。
- 按失败频率补 syscall。
- 完善路径解析、权限位、时间、目录项、pipe、dup。
- 建立失败用例分类表。

验收标准：

- 每次运行能生成测试摘要。
- `tests/results/` 中保存最近一次通过数、失败数和失败原因。

### 阶段 4：LTP 高价值测例

目标：围绕 LTP 形成高分突破。

任务：

- 先在 Linux 上跑一遍同版本 LTP，记录正常行为。
- 在本内核上跑 LTP，按模块归类失败原因。
- 优先突破文件系统、信号、futex、mmap、wait/clone 相关测例。
- 实现或修正 `rt_sigaction`、`rt_sigprocmask`、`kill`、`nanosleep`、`clock_gettime`、`poll/select`。

验收标准：

- `docs/ltp-report.md` 中能看到分阶段通过率变化。
- 每个新增 syscall 都有至少一个回归测试或 LTP 对应用例。

### 阶段 5：LoongArch64 和硬件

目标：双架构可运行，决赛前具备开发板适配能力。

任务：

- 将 `arch` crate 中的 trap、context、page table、timer 抽象稳定下来。
- 先跑 LoongArch64 QEMU。
- 再适配龙芯板相关启动、中断、块设备和内存差异。
- 保留 RISC-V64 与 LoongArch64 的统一 syscall 行为。

验收标准：

- 同一套用户程序能在 RISC-V64 QEMU 和 LoongArch64 QEMU 运行。
- 双架构测试日志能被同一套 `collect-results.sh` 收集。

## 下一步该做什么

按照优先级执行：

1. 新建 `oskernel-arceos/` 仓库骨架。
2. 将 ArceOS 添加为 `third_party/arceos`。
3. 写根目录 `Makefile`，让 `make all` 最终产出 `kernel-rv` 和 `kernel-la`。
4. 跑通 ArceOS 的 RISC-V64 QEMU 示例。
5. 梳理 `test/oscomp_syscalls.md`，生成第一版 `docs/syscall-table.md`。
6. 写 `scripts/run-qemu-riscv64.sh`，保证启动命令固定下来。
7. 实现最小用户态 ELF 加载和 `write/exit`。
8. 实现 `test-runner`：扫描 EXT4 测试盘，串行执行 `*_testcode.sh`。
9. 接入 busybox rootfs。
10. 从基础测例开始跑自动化，之后进入 LTP。

第一周不要做这些事：

- 不要重写 ArceOS 已有 HAL、调度器或内存分配器。
- 不要先追求复杂应用如 gcc/rustc。
- 不要先做开发板。
- 不要等 syscall 全部写完再跑测试。

第一周唯一核心目标：

```text
ArceOS BaseOS + RISC-V64 QEMU + 第一个 Linux 用户态 ELF + syscall trace
```

只要这个目标完成，后续所有工作都能围绕测试失败日志持续推进。
