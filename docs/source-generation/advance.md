## 3. 高级功能

### 3.1 自动生成 `mod.rs`

本框架内置对 `mod.rs` 文件的自动生成支持，旨在简化模块组织与公共导出，避免手动维护 `mod` / `pub use` 声明。

#### 默认行为

在启用符号导出（通过 `register_export_symbol()`）后，框架会自动在输出目录生成 `mod.rs`，并包含所有设置为 `export = true` 的符号：

```rust
context.register_export_symbol(
    "generated::Foo",
    SymbolExportType::Use { as_name: None },
    true, // 将该符号纳入 mod.rs 自动管理
)?;
```

生成示例（`src/generated/mod.rs`）：

```rust
#![allow(unused_imports)]
pub use self::generated::Foo;
```

#### 指定根目录

你可以在 `SourceGenerationContext` 构造时指定一个输出根目录：

```rust
let context = SourceGenerationContext::new("src/generated".into(), true);
```

该目录将作为所有生成文件和 `mod.rs` 的存放位置。当前采用“扁平结构”，即不递归生成嵌套模块的 `mod.rs`。

#### 自定义导出格式

你可以通过不同的 `SymbolExportType` 配置导出的具体行为：

| 类型                    | 示例代码                         | 用途                         |
| ----------------------- | -------------------------------- | ---------------------------- |
| `Use { as_name: None }` | `pub use generated::Foo;`        | 普通符号导出（默认）         |
| `Use { as_name: Some }` | `pub use generated::Foo as Bar;` | 自定义别名导出               |
| `Mod`                   | `mod my_mod;`                    | 导出一个子模块（用于子文件） |

你还可以在生成器内部控制是否插入文档注释，是否使用 pub use 等（可通过生成的代码自行决定，框架不会自动插入 doc 注释）。
