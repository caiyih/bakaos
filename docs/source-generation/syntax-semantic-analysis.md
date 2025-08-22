## 基于语法语义分析的代码生成

本部分将介绍 **Source Generation Framework** 如何与 Rust 的 **过程宏（proc macro）** 或 **语法/语义分析器** 协同工作，形成一个完整的“从源代码解析 → 编译期分析 → 自动生成代码”的工作流。

## 与过程宏 / 语法语义分析协同工作的工作流

### 场景描述

当你希望根据现有 Rust 代码（例如结构体、函数、属性宏等）生成额外代码（如注册表、默认实现、DSL 映射），可以通过：

1. **Rust 分析工具**（如 `syn`, `quote`）提取原始结构。
2. 使用本框架作为 **代码生成执行器**，统一输出 `.rs` 文件并导出符号。
3. 自动导入生成模块供主程序使用。

### 工作流步骤

#### 1. 源代码作为输入（已存在的 Rust 文件）

你可能已有如下结构体定义：

```rust
// src/components/foo.rs
#[derive(Component)]
pub struct Foo {
    pub value: u32,
}
```

#### 2. 分析文件内容（使用 syn）

在 `ISourceGenerator::execute()` 中读取并分析已有文件：

```rust
let content = std::fs::read_to_string("src/components/foo.rs")?;
let syntax_tree = syn::parse_file(&content)?;
```

你可以使用 `syn`, `quote`, `proc_macro2` 对语法树进行遍历、匹配等操作。

#### 3. 提取信息 / 构造输出（语义分析）

从语法树中提取目标对象（如带某种属性的结构体、函数等）：

```rust
for item in syntax_tree.items {
    if let syn::Item::Struct(s) = item {
        // 判断是否带有 #[derive(Component)]
        let is_component = s.attrs.iter().any(|attr| {
            attr.path().is_ident("derive") && attr.tokens.to_string().contains("Component")
        });

        if is_component {
            // 构建对应的注册语句或 impl 块
        }
    }
}
```

#### 4. 使用框架生成代码

将分析结果转化为代码字符串，并写入生成目录：

```rust
context.add_source("components.rs", &generated_code, false, true)?;
context.register_export_symbol(
    "generated::FooComponent",
    SymbolExportType::Use { as_name: None },
    true,
)?;
```

生成的内容将写入 `src/generated/components.rs`，并被自动导出到 `mod.rs`。

#### 5. 在主库中引用导出模块

```rust
mod generated;
pub use generated::*;
```

### 优势

- 可清晰拆分为“语法提取 / 分析”和“代码生成 / 输出”两个阶段。
- 生成的代码可以明确控制路径、内容和导出方式。
- 可与手写代码、过程宏生成代码共存，不存在污染。
- 支持多文件、多任务组织，便于拆分 DSL、注册器、导出器等任务。

### 对比过程宏 / build.rs 的职责边界

| 功能            | 过程宏               | Source Generation Framework    |
| --------------- | -------------------- | ------------------------------ |
| 编译期代码修改  | ✅（宏注入）         | ✅（build.rs 中生成源码）      |
| 原始源码访问    | ❌（需配合宏）       | ✅（可读取任意 .rs/.dsl 文件） |
| 多模块生成      | ❌（宏受限于单对象） | ✅（多任务、多文件输出）       |
| 文件输出        | ❌                   | ✅（写入磁盘）                 |
| 自动导出 mod.rs | ❌                   | ✅                             |
| Lint 控制       | ❌                   | ✅                             |

### 推荐组合

- 使用 `syn` + `quote` 做 AST 分析
- 使用本框架做生成器驱动、输出管理、符号导出
- 手动控制哪些文件作为输入源、哪些作为输出目标
- 适合用于大型代码注册、DSL 编译、自动文档生成等
