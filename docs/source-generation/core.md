## 架构概览

本节将简要介绍 Source Generation 框架的核心组件以及其协作方式，并说明当前支持的文件结构设计。

### 1.1 核心组件介绍

本框架采用模块化架构设计，核心组件各司其职，方便扩展与维护。

#### `SourceGenerationContext`

表示生成过程中的上下文对象，由框架在运行时创建并传递给每个生成任务（`ISourceGenerator`）使用。

功能包括：

- 写入源文件（带有文件缓存与覆盖检测）
- 注册符号（用于后续自动导出，如 `mod.rs`）
- 配置输出路径、格式选项等
- 记录是否启用 Lint Suppression

```rust
let mut context = SourceGenerationContext::new("src/generated".into(), true);
```

常用方法：

- `add_source(...)`：添加生成文件
- `register_export_symbol(...)`：注册要导出的模块路径或符号
- `get_symbol_registry()`：访问符号注册表

#### `ISourceGenerator`

代码生成器任务的抽象接口，所有代码生成逻辑应通过实现该 trait 完成。

核心方法：

```rust
trait ISourceGenerator {
    fn init(&mut self) { ... }               // 可选初始化逻辑
    fn name(&self) -> &'static str;          // 用于日志或调试的唯一名称
    fn execute(&mut self, context: &mut SourceGenerationContext) -> Result<(), SourceGenerationError>;
}
```

每一个生成器通常会：

- 读取输入文件（可选）
- 生成字符串形式的 Rust 源码
- 调用 `context.add_source()` 写入目标文件
- 调用 `context.register_export_symbol()` 注册自动导出的符号（如 `mod::name`）

#### `SourceGenerationDriver`

用于统一驱动和执行多个 `ISourceGenerator` 实例的调度器。

构造时接收生成器列表，并在执行时依次调用每个任务。支持配置是否在遇到错误时立即中断（fail-fast）。

```rust
let driver = SourceGenerationDriver::new(vec![
    Box::new(MyGenerator),
    Box::new(OtherGenerator),
]);

driver.execute(context, false /* fail_fast */)?;
```

该组件不会并行执行任务（当前版本不关注并行性），执行顺序即为注册顺序。

#### `SymbolRegistry`

符号注册表，用于记录需要导出的路径（模块名或对象名）。一般通过 `SourceGenerationContext` 间接使用。

符号主要用于自动生成 `mod.rs` 文件，提升模块可用性与导出管理效率。

每条符号包括：

- 完整路径（如 `my_mod::MyStruct`）
- 导出类型（例如 `use`、`mod`）
- 是否包含到自动生成的 mod.rs 中

### 1.2 文件结构示意

当前版本的框架仅支持“扁平化文件结构”，即所有生成的文件会被统一写入到指定输出目录（如 `src/generated`）下，而不会创建子目录或模块嵌套。

优点：

- 简单直观
- 易于管理和导入（通常配合自动生成的 `mod.rs`）

示例结构如下：

```
src/
├── lib.rs
└── generated/
    ├── generated.rs
    ├── build_info.rs
    ├── schema.rs
    └── mod.rs  // 自动生成，导出上述文件中的符号
```

你可以在 `lib.rs` 中通过以下方式将所有自动导出的对象统一暴露：

```rust
#[rustfmt::skip]
mod generated;

#[allow(unused_imports)]
pub use generated::*;
```

如需支持嵌套结构，建议你自行在任务中实现相关逻辑，目前框架本身不会递归处理目录结构。
