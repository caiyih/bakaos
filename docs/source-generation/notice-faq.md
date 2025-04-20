## 注意事项

在使用该框架进行代码生成时，建议遵循以下规范以提高兼容性、生成效率与未来可维护性：

### 应当使用 FQN（Fully Qualified Name）

在生成代码中，尽量使用完整路径引用符号，例如：

```rust
pub const GENERATED_TEXT: &::core::primitive::str = "Hello, world!";
```

这样可以避免在导入环境中产生歧义，尤其是在没有 `use` 语句的情况下。

---

### 倾向使用 `core` 与 `alloc` 代替 `std`

由于 `std` 依赖于运行时，建议在生成代码中使用 `core` / `alloc` 提供的功能，确保代码在 `no_std` 环境下也可编译。

示例：

```rust
use ::core::fmt::Write;  // 推荐
```

而非：

```rust
use std::fmt::Write;     // 避免
```

---

### 避免覆盖手写代码

当调用 `add_source()` 添加文件时，建议将 `overwrite_existing` 设置为 `false`，防止意外覆盖已有手写逻辑文件：

```rust
context.add_source("my_file.rs", &text, false, true)?;
```

---

### 模块名与符号名需唯一

由于符号注册用于 `mod.rs` 生成，请确保模块名不会重复，且导出的符号具有唯一性（或设置 `as_name` 避免冲突）。

---

### 生成文件应避免格式化错误

由于生成代码可能自动导出并在主项目中被引用，请确保生成文件中语法正确且符合格式要求，可使用 `rustfmt` 预先格式化或测试验证。

## 常见问题 FAQ

### 为什么文件没有写入？

可能原因：

- 未调用 `context.add_source()`。
- 指定的文件路径无效（非生成目录下）。
- 没有设置 `overwrite_existing_gen = true`，导致已有文件未被覆盖。
- `driver.execute()` 未被执行，或 panic 中断了流程。

建议确认 `add_source` 调用逻辑、路径是否正确，并查看运行时日志是否提示跳过或失败。

---

### 为什么任务没有生效？

检查：

- 是否将任务添加到 `SourceGenerationDriver` 中？
- 是否正确实现了 `ISourceGenerator::execute()`？
- 是否手动执行了 `driver.execute(...)`？
- 任务是否由于错误中止？（建议设置 `fail_fast = false` 观察所有任务）

---

### 如何避免覆盖手写代码？

在 `add_source()` 时设置 `overwrite_existing = false`：

```rust
context.add_source("my_code.rs", content, false, true)?;
```

这样如果目标文件已存在且不是框架生成的，将跳过写入，避免覆盖。

也可以手动检查文件是否存在，并改用 append 或 merge 模式自行处理。

---

### 支持 async 吗？

当前框架 **不支持 async 或并发执行**，也不会在内部进行任何多线程调度。  
其设计目标是清晰、确定、顺序的代码生成流程。  
你可以在自己的 `ISourceGenerator` 中手动读取文件或使用阻塞 I/O，但需避免混入异步调用。
