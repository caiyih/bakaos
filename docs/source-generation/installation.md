## 快速上手

本节将展示如何在项目中快速集成并使用该 Source Generation 框架，以实现简单的编译期代码生成。

### 安装方式

在你的 `Cargo.toml` 中添加以下依赖项，将框架作为 `build-dependencies` 引入：

```toml
[build-dependencies]
source-generation = { git = "https://github.com/caiyih/bakaos" }
```

该框架设计用于在 `build.rs` 中使用，因此无需在运行时代码中引用它。

### 示例代码（生成一个常量）

以下是一个使用该框架在编译期生成 Rust 源码的简单示例。它将创建一个 `generated.rs` 文件，并在其中定义一个常量 `GENERATED_TEXT`。

#### build.rs

```rust
use source_generation::{
    ISourceGenerator, SourceGenerationContext, SourceGenerationDriver, SourceGenerationError,
    SymbolExportType,
};

fn main() {
    // 配置 generated 目录（输出目录）和 lint 抑制（建议开启）
    let context = SourceGenerationContext::new("src/generated".into(), true);

    let driver = SourceGenerationDriver::new(vec![
        // 添加自定义生成器
        Box::new(BuildInfoGenerator)
    ]);

    // 执行所有生成器；true 表示遇到错误立即返回
    driver.execute(context, false).unwrap();
}

// 一个简单的代码生成器实现
struct BuildInfoGenerator;

impl ISourceGenerator for BuildInfoGenerator {
    fn execute(
        &mut self,
        context: &mut SourceGenerationContext,
    ) -> Result<(), SourceGenerationError> {
        let source_text = format!(
            "pub const GENERATED_TEXT: &::core::primitive::str = \"Hello, world!\";"
        );

        // 添加生成文件，支持覆盖策略控制
        context.add_source(
            "generated.rs",
            &source_text,
            false, // 不允许覆盖非生成文件（建议）
            true   // 允许覆盖已生成文件（建议）
        )?;

        // 注册导出符号，用于后续自动生成 mod.rs 文件
        context.register_export_symbol(
            "generated::GENERATED_TEXT",
            SymbolExportType::Use { as_name: None },
            true,
        )?;

        Ok(())
    }

    fn init(&mut self) {}

    fn name(&self) -> &'static str {
        "BuildInfoGenerator"
    }
}
```

#### src/lib.rs

```rust
// 避免 rustfmt 破坏生成的 mod.rs 排版
#[rustfmt::skip]
mod generated;

// 可选：导出所有自动生成的符号
#[allow(unused_imports)]
pub use generated::*;
```

运行 `cargo build` 后，你将会在 `src/generated/` 目录下看到生成的 `generated.rs` 文件，并能在运行时代码中访问 `GENERATED_TEXT` 常量。

如果你有更多想要生成的结构体、函数或模块，可以实现更多的 `ISourceGenerator` 实例并注册到 `SourceGenerationDriver` 中。
