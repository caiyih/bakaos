# 项目结构

```ascii
├── crates
│   ├── Cargo.toml                  # Rust 工作区配置文件
│   ├── abstractions                # 通用抽象，用于代码复用
│   ├── address                     # 内存地址空间相关的抽象
│   ├── allocation                  # 页帧分配器
│   ├── constants                   # 内核无关常量，例如页大小，构建时间等
│   ├── drivers                     # 驱动程序及驱动程序抽象层
│   ├── filesystem-abstractions     # 文件系统抽象层
│   ├── filesystem                  # 与驱动程序抽象层直接交互的文件系统实现
│   ├── paging                      # 分页机制与内存管理
│   ├── path                        # 路径处理
│   ├── tasks                       # 任务进程管理
│   ├── threading                   # 线程协程管理，任务调度
│   ├── timing                      # 时间处理
│   └── unwinding                   # 栈回溯展开
│   ├── template                    # 模板库
├── docs                # 文档
├── InspectCode.sh      # 代码质量检查
├── kernel              # 内核相关
│   ├── binary         # SBI 等二进制文件
│   ├── lds            # 链接器脚本
│   ├── Makefile       # 内核构建脚本
│   ├── src            # 内核源代码
│   │   ├── firmwares
│   │   │   ├── console.rs      # 控制台输出
│   │   ├── memory
│   │   │   ├── global_heap.rs  # 内核全局堆
│   │   ├── platform             # 平台相关代码
│   │   │   ├── machine.rs      # 设备抽象层
│   │   │   └── virt.rs         # virt QEMU 平台相关代码
│   │   ├── syscalls             # 系统调用实现
│   │   │   ├── mod.rs          # syscall dispatcher
│   │   │   ├── file_async.rs   # 异步文件系统相关 syscall
│   │   │   ├── file.rs         # 文件系统相关syscall
│   │   │   ├── task_async.rs   # 异步任相关 syscall
│   │   │   └── task.rs         # 任务相关 syscall
│   │   └── trap                 # 中断处理
│   │       ├── kernel.rs        # 内核中断处理
│   │       └── user.rs          # 用户态中断处理
│   │   ├── ci_helper.rs         # CI 测试辅助
│   │   ├── kernel.rs            # 内核抽象
│   │   ├── logging.rs           # 日志
│   │   ├── main.rs              # 内核入口及主循环
│   │   ├── panic_handling.rs    # panic 处理即栈回溯
│   │   ├── processor.rs         # 处理器相关，多核处理器支持
│   │   ├── scheduling.rs        # 进程主循环
│   │   ├── serial.rs            # 串口输出
│   │   ├── statistics.rs        # 内核统计信息
│   │   ├── timing.rs            # 时间处理
│   └── unwinder.py     # 辅助栈回溯展开脚本
├── LICENSE              # MIT 许可证
├── Makefile             # 初赛测试构建脚本
├── README.md            # 项目说明
└── test_preliminary            # 初赛测试相关 artifacts
    ├── grading_scripts         # 评测脚本
    ├── README.md               # 评测环境文档
    ├── sdcard.img              # 初赛测试镜像
    └── visualize_result.py     # 评测结果可视化脚本
```