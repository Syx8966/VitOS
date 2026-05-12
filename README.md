# oskernel-arceos

本仓库以 ArceOS 作为 BaseOS，目标是在其组件化内核能力之上实现面向系统能力大赛 OS 内核实现赛道的 Linux 兼容宏内核。

当前阶段：阶段 0，完成上游代码拉取、目录骨架、构建入口确认和 syscall 清单梳理。

## 关键目录

- `third_party/arceos/`：ArceOS BaseOS，上游源码。
- `third_party/reference/starry-next/`：StarryOS/starry-next 参考实现，只读参考，不进入主构建。
- `crates/`：后续放置本队实现的内核模块。
- `scripts/`：后续放置构建、QEMU、测试脚本。
- `docs/`：设计文档、阶段路线、syscall 表和测试报告。

## 阶段 0 文档

见 [docs/stage0-bootstrap.md](docs/stage0-bootstrap.md)。
