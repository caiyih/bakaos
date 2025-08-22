## 2. 使用教程（快速上手）

本节将介绍如何从零开始使用本框架完成一个简单的代码生成任务，包括定义任务、配置驱动器、文件管理策略、符号导出与 Lint 抑制。

### 2.1 定义一个任务

每一个代码生成器任务需实现 `ISourceGenerator` trait，它定义了最基本的生命周期和执行接口。

示例：创建一个简单的构建信息生成器，输出静态字符串内容：

```rust
use source_generation::{
    ISourceGenerator, SourceGenerationContext, SourceGenerationError, SymbolExportType,
};

struct BuildInfoGenerator;

impl ISourceGenerator for BuildInfoGenerator {
    fn name(&self) -> &'static str {
        "BuildInfoGenerator"
    }

    fn init(&mut self) {
        // 可选初始化逻辑
    }

    fn execute(&mut self, context: &mut SourceGenerationContext) -> Result<(), SourceGenerationError> {
        let source_text = r#"
            pub const GENERATED_TEXT: &::core::primitive::str = \"Hello, world!\";
        "#;

        context.add_source(
            "generated.rs",
            source_text,
            false, // 不覆盖已有的非生成文件
            true,  // 允许覆盖旧的生成文件
        )?;

        context.register_export_symbol(
            "generated::GENERATED_TEXT",
            SymbolExportType::Use { as_name: None },
            true,
        )?;

        Ok(())
    }
}
```

生成器中可读取文件、处理模板、构建结构体等，只需最终通过 `context.add_source()` 写入目标路径。

### 2.2 注册任务到 SourceGenerationDriver

定义完任务后，需要将其注册到生成器驱动器中进行统一调度。

完整示例：

```rust
use source_generation::{SourceGenerationContext, SourceGenerationDriver};

fn main() {
    let context = SourceGenerationContext::new("src/generated".into(), true);

    let driver = SourceGenerationDriver::new(vec![
        Box::new(BuildInfoGenerator),
        // 可添加更多任务
    ]);

    // 执行任务，设置为 false 表示遇错不中断
    driver.execute(context, false).unwrap();
}
```

其中：

- `new("src/generated".into(), true)`：配置输出目录和 Lint 抑制（见 2.5）
- `execute(..., false)`：是否启用 fail-fast 模式（true 表示一旦失败立即终止）

### 2.3 使用管理文件

每个生成器可以选择性地控制生成文件的覆盖策略：

```rust
context.add_source(
    "my_module.rs",
    &source_code,
    false, // 不覆盖已有的非生成文件
    true   // 允许覆盖之前生成的同名文件
)?;
```

推荐设置：

| 参数                     | 含义                         | 建议值                 |
| ------------------------ | ---------------------------- | ---------------------- |
| `overwrite_existing`     | 是否允许覆盖非生成的手写代码 | `false`（防止误覆盖）  |
| `overwrite_existing_gen` | 是否允许覆盖旧的自动生成代码 | `true`（更新生成文件） |

### 2.4 使用 SourceGenerationContext 注册符号

为了支持自动生成 `mod.rs` 和统一导出，建议将生成的对象注册到符号系统中。

调用示例：

```rust
context.register_export_symbol(
    "generated::MyStruct",                        // 完整路径
    SymbolExportType::Use { as_name: None },      // 导出方式
    true,                                         // 是否公开（public）导出
)?;
```

支持的导出类型包括：

- `Use { as_name: Option<String> }`：生成 `pub use path [as name];`
- `Mod`：生成 `mod xxx;`，用于模块结构导出

该功能不会影响实际运行逻辑，但会影响 `mod.rs` 的生成内容，从而影响公共导出（`pub use`）。

### 2.5 Lint Suppression

为了避免 IDE 或编译器对生成文件提出不必要的警告，你可以在 `SourceGenerationContext` 中启用 Lint 抑制：

```rust
let context = SourceGenerationContext::new("src/generated".into(), true);
```

其中第二个参数 `true` 表示启用 Lint Suppression，会在注册各个文件时添加如下代码：

```rust
#[rustfmt::skip]
```

这对于生成的代码非常重要，特别是结构体/函数暂未被调用的阶段。
