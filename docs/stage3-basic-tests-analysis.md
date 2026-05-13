# 阶段 3：basic 测试分析

## 测试镜像如何生成

`../test/Makefile` 会生成两套架构镜像：

- `sdcard-rv.img.xz`
- `sdcard-la.img.xz`

每个镜像是无分区表 EXT4 文件系统。根目录下会有：

```text
musl/
├── basic/
│   ├── brk
│   ├── chdir
│   ├── ...
│   ├── write
│   ├── yield
│   ├── run-all.sh
│   └── text.txt
├── basic_testcode.sh
├── busybox
└── lib/

glibc/
├── basic/
├── basic_testcode.sh
├── busybox
└── lib/
```

`Makefile.sub` 的 `basic` 目标执行：

```bash
make -C basic/user all CHAPTER=7 ARCH=<arch>
cp -r basic/user/build/<arch>/* <DESTDIR>/basic/
cp scripts/basic/basic_testcode.sh <DESTDIR>
```

`basic/user/src/oscomp/run-all.sh` 固定执行 32 个测试程序：

```text
brk chdir clone close dup2 dup execve exit fork fstat getcwd getdents getpid
getppid gettimeofday mkdir_ mmap mount munmap openat open pipe read sleep
times umount uname unlink wait waitpid write yield
```

## 评测时如何运行

赛方 QEMU 会额外挂载一个 EXT4 测试盘：

- RISC-V：`virtio-blk-device`
- LoongArch64：`virtio-blk-pci`

内核启动后必须主动扫描测试盘根目录，找到 `*_testcode.sh`，串行执行。basic 脚本内容是：

```sh
./busybox echo "#### OS COMP TEST GROUP START basic ####"
cd ./basic
./run-all.sh
cd ..
./busybox echo "#### OS COMP TEST GROUP END basic ####"
```

也就是说，完整运行 basic 需要：

- 能挂载并读 EXT4 测试盘。
- 能执行 `musl/basic_testcode.sh` 和 `glibc/basic_testcode.sh`。
- 能运行 busybox 的 `echo`、`cd` 或者由内核测试 runner 模拟等价脚本语义。
- 能进入 `basic/` 目录并逐个执行 32 个 ELF。
- 每个程序退出后继续执行下一个程序。

评分主要依赖串口输出。至少要保留测试组起止标记：

```text
#### OS COMP TEST GROUP START basic ####
...
#### OS COMP TEST GROUP END basic ####
```

## basic 程序形态

basic 测试程序来自 `basic/user/src/oscomp/*.c`，不是 libc/glibc 常规程序，而是链接了自带的轻量 `ulib`。

它们直接使用 Linux syscall 编号，例如：

```text
write=64
exit=93
brk=214
mmap=222
openat=56
read=63
clone=220
execve=221
wait4=260
```

程序内部用 `TEST_START` / `TEST_END` / `assert` 打印结果。`*_test.py` 是离线解析输出的参考，不在目标 OS 内运行。

## 当前内核差距

阶段 2 已完成：

- 内置 hello ELF。
- ELF `PT_LOAD` 映射。
- 单用户程序进入用户态。
- 最小 `write(fd=1/2)`。
- 最小 `exit` 后关机。

basic 还需要：

1. 从外部 EXT4 测试盘读取 ELF，而不是只运行内置 ELF。
2. 支持多个用户进程串行运行，`exit` 后返回内核 runner，而不是直接关机。
3. 支持当前工作目录和路径解析。
4. 支持 fd table。
5. 支持文件读写和目录操作。
6. 支持堆与匿名/file mmap。
7. 支持 fork/clone/wait/execve。
8. 支持时间、uname、yield 等基础 syscall。

## basic syscall 分组

### 第一批：先跑单进程文件/内存基础

这些是进入 basic 的最低门槛：

| syscall | 编号 | 用途 |
| --- | ---: | --- |
| `exit` | 93 | 用户程序退出，返回 runner |
| `write` | 64 | stdout/stderr 输出 |
| `openat` | 56 | 打开 `text.txt` 和创建测试文件 |
| `read` | 63 | 读取 `text.txt` |
| `close` | 57 | 关闭 fd |
| `fstat` | 80 | 文件大小，mmap 测试依赖 |
| `brk` | 214 | 堆位置增长 |
| `mmap` | 222 | 匿名/file 映射 |
| `munmap` | 215 | 释放映射 |

建议先让这些程序能单独跑：

```text
write open read close fstat brk mmap munmap
```

### 第二批：目录和路径

| syscall | 编号 | 用途 |
| --- | ---: | --- |
| `getcwd` | 17 | 当前目录 |
| `chdir` | 49 | 切换目录 |
| `mkdirat` | 34 | 创建目录 |
| `unlinkat` | 35 | 删除文件 |
| `getdents64` | 61 | 遍历目录 |
| `mount` | 40 | basic 里可先 stub 成成功 |
| `umount2` | 39 | basic 里可先 stub 成成功 |

### 第三批：进程模型

| syscall | 编号 | 用途 |
| --- | ---: | --- |
| `getpid` | 172 | 返回当前 pid |
| `getppid` | 173 | 返回父 pid |
| `clone` | 220 | `fork()` 和 `clone()` 都依赖它 |
| `wait4` | 260 | 等待子进程 |
| `execve` | 221 | 执行 `test_echo` |
| `sched_yield` | 124 | yield 测试 |

basic 里的 `fork()` 实际是：

```c
syscall(SYS_clone, SIGCHLD, 0)
```

因此 `clone(flags=SIGCHLD, stack=0)` 要按 fork 语义复制进程；`clone(fn, stack, SIGCHLD)` 需要创建子执行流。

### 第四批：时间和系统信息

| syscall | 编号 | 用途 |
| --- | ---: | --- |
| `gettimeofday` | 169 | `get_time()`、sleep 测试 |
| `nanosleep` | 101 | sleep 测试 |
| `times` | 153 | 进程时间 |
| `uname` | 160 | 返回系统信息 |

## 阶段 3 建议实现顺序

1. **测试盘和 runner**
   - QEMU 脚本加入测试盘参数。
   - 接入 ArceOS block driver + EXT4/VFS。
   - 扫描根目录 `musl` / `glibc` 下的 `*_testcode.sh`。
   - 先模拟 basic 脚本，输出 START/END，进入 `basic/` 执行 `run-all.sh` 列表。

2. **执行外部 ELF**
   - 把阶段 2 的内置 ELF loader 改成从 VFS 读取 bytes。
   - 支持运行一个指定路径 ELF。
   - `exit` 后返回 runner，继续执行下一个程序。

3. **fd table + 文件**
   - fd 0/1/2 绑定 stdin/stdout/stderr。
   - `openat/read/write/close/fstat` 接入 VFS。
   - 路径相对当前工作目录解析。

4. **内存管理**
   - 每进程维护 heap end。
   - 实现 `brk`。
   - 实现匿名 `mmap`、文件 `mmap`、`munmap`。

5. **进程模型**
   - 进程结构：pid、ppid、地址空间、fd table、cwd、exit_code。
   - `clone/fork/wait4/execve/getpid/getppid/sched_yield`。

6. **补齐 basic 剩余 syscall**
   - 目录：`mkdirat/getcwd/chdir/getdents64/unlinkat`。
   - 时间：`gettimeofday/nanosleep/times`。
   - 系统信息：`uname`。
   - `mount/umount2` 在 basic 阶段可以先做受控 stub。

## 立即下一步

不要直接补所有 syscall。阶段 3 的第一步应是建立“测试盘可见 + basic runner 可打印标记”的骨架：

```text
kernel boots
  -> find EXT4 test disk
  -> list root directory
  -> find musl/basic_testcode.sh and glibc/basic_testcode.sh
  -> print START basic
  -> select first external ELF, e.g. basic/write
  -> load and run it
  -> return to runner on exit
```

只有 runner 和外部 ELF 执行闭环成立后，再按 syscall 表补功能。
