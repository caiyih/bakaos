# 内核栈展开

当用户程序抛出未捕获异常或 panic 时，通常会获得非常详细的堆栈跟踪信息，这些信息包括函数调用链、函数参数和局部变量等。这些信息是通过展开调用栈来获取的。

但是对于内核这样的裸机程序来说，即使我们尝试进行栈回溯，也很难获得像用户程序一样的详细的栈跟踪信息。这是因为内核的栈通常是裸机程序的栈，不包含任何元数据，如函数调用链、函数参数和局部变量等。而用户程序通常有运行时的帮助，因而可以获得这些信息。

但是我们仍然希望能通过某种方式来获取内核的栈跟踪信息，以便更好地调试内核程序。本项目实现了源码级别的栈展开功能，可以在**任意时刻**保存调用栈信息，并在需要时展开调用栈。并且查看源码文件，行号、函数名、相关指令以及PC值等信息。

## 实现

我们的 `crates/unwinding` 库实现了栈回溯功能，可以在任意时刻保存调用栈信息，并在需要时展开调用栈。当生成栈回溯时，我们会根据 `fp` 遍历调用栈，获取每个函数的返回地址以及栈指针。然后保存在 `StackFrame` 数据结构中，并储存为整个调用栈的帧信息`StackTrace`。

栈回溯可以在任意时刻生成，并非仅在 panic 时。例如，我们可以在 debug 构建的获取锁时保存栈回溯，这样当死锁发生时，我们可以通过展开调用栈来查找死锁的原因。

## 使用

生成栈回溯使用`StackTrace::begin_unwind(skip_frames: usize) -> StackTrace`函数，其中`skip_frames`参数表示跳过的帧数。例如，如果我们希望跳过当前函数，可以传入`1`。例如，在 panic 时，我们希望跳过 `rust_begin_unwind` 函数，可以传入`1`。

对于每一个`StackTrace`对象，我们可以使用`pub fn stack_frames(&self) -> &[StackFrame];`函数获取栈帧信息。每一个`StackFrame`对象包含了`fp`和`ra`信息。

但是我们想要的并不是`ra`，而是`PC`。`unwinding`提供了`find_previous_instruction(ra: usize) -> Result<usize, u64>`函数，可以根据`ra`获取`PC`值。该函数支持可变长度指令，可以正确地获取`PC`值。对于 RISC-V 指令集，该函数能够正确解析`16`位，`32`位，`48`位和`64`位指令，支持了 RISC-V 标准指令集的所有指令以及未实现的扩展指令。当解析失败时，会返回`ra`前64位的值。

在内核代码中，每一个`StackTrace`对象都可以直接通过`print_trace`方法打印调用栈信息。这样我们就可以在任意时刻生成调用栈信息，并在需要时展开调用栈。这可以获取到类似下面的信息：

```text
[BAKA-OS]     Stack trace:
[BAKA-OS]        0 at: 0xffffffc0802043aa Frame pointer: 0xffffffc080a930e0
[BAKA-OS]        1 at: 0xffffffc080205444 Frame pointer: 0xffffffc080a938c0
[BAKA-OS]        2 at: 0xffffffc08020ad98 Frame pointer: 0xffffffc080a93d20
[BAKA-OS]        3 at: 0xffffffc080205852 Frame pointer: 0xffffffc080a93da0
[BAKA-OS]        4 at: 0xffffffc08020548c Frame pointer: 0xffffffc080a93fd0
[BAKA-OS]        5 at: 0xffffffc080200058 Frame pointer: 0xffffffc080a93ff0
[BAKA-OS]     Note: Higher traces are deeper. You can check symbol files for detailed info.
```

尽管如此，我们仍然只能获取 fp 和 PC，并不能获取函数名、源码文件和行号等信息。那我们是如何获取到这些信息的呢？事实证明，在内核中通过 DWARF 调试信息来获取这些信息是非常困难的。因此我们的解决方案是借助外部工具来获取这些信息。在构建内核时，我们会默认包含所有的 DWARF 调试信息。当需要进行栈展开时，我们会懒式地使用`objdump`生成符号表。

接着，我们编写了一个辅助栈展开器`unwinder.py`。当它检测到内核输出栈回溯信息后，它会懒式构建符号表，然后根据栈回溯信息和符号表，生成详细的栈展开信息。以下是栈展开器输出的样例：

![unwinded](https://github.com/user-attachments/assets/2def908d-2a22-4202-8f8b-7e953cbc9b9e)

```text
Unwinding stack trace:
  pc: 0xffffffc0802038b0
    at: bakaos::panic_handling::stack_trace() in /home/runner/work/bakaos/bakaos/kernel/src/panic_handling.rs:36
    disassembly of the line:
      ffffffc0802038a4 <bakaos::panic_handling::stack_trace+0x4> sd	ra,2008(sp)
      ffffffc0802038a8 <bakaos::panic_handling::stack_trace+0x8> sd	s0,2000(sp)
      ffffffc0802038ac <bakaos::panic_handling::stack_trace+0xc> addi	s0,sp,2016
      ffffffc0802038b0 <bakaos::panic_handling::stack_trace+0x10> auipc	ra,0x0
      ffffffc0802038b4 <bakaos::panic_handling::stack_trace+0x14> jalr	-42(ra) # ffffffc080203886 <bakaos::panic_handling::lr>
      ffffffc0802038b8 <bakaos::panic_handling::stack_trace+0x18> sd	a0,-1840(s0)
  pc: 0xffffffc0802048b2
    at: rust_begin_unwind() in /home/runner/work/bakaos/bakaos/kernel/src/panic_handling.rs:99
    disassembly of the line:
      ffffffc0802048b2 <.Lpcrel_hi100+0x30> auipc	ra,0xfffff
      ffffffc0802048b6 <.Lpcrel_hi100+0x34> jalr	-18(ra) # ffffffc0802038a0 <bakaos::panic_handling::stack_trace>
      ffffffc0802048ba <.Lpcrel_hi100+0x38> j	ffffffc0802048aa <.Lpcrel_hi100+0x28>
  pc: 0xffffffc08020acea
    at: core::panicking::panic_fmt() in /rustc/bf3c6c5bed498f41ad815641319a1ad9bcecb8e8/library/core/src/panicking.rs:72
    disassembly of the line:
      ffffffc08020ace8 <.Lpcrel_hi416+0x14> mv	a0,sp
      ffffffc08020acea <.Lpcrel_hi416+0x16> auipc	ra,0xffff9
      ffffffc08020acee <.Lpcrel_hi416+0x1a> jalr	1394(ra) # ffffffc08020425c <rust_begin_unwind>
  pc: 0xffffffc0802035b0
    at: bakaos::clear_bss() in /home/runner/work/bakaos/bakaos/kernel/src/main.rs:177
    disassembly of the line:
      ffffffc08020359a <bakaos::clear_bss+0x2> sd	ra,8(sp)
      ffffffc08020359c <bakaos::clear_bss+0x4> sd	s0,0(sp)
      ffffffc08020359e <bakaos::clear_bss+0x6> addi	s0,sp,16
      ffffffc0802035a0 <.Lpcrel_hi43> auipc	a0,0x891
      ffffffc0802035a4 <.Lpcrel_hi43+0x4> addi	a0,a0,-1440 # ffffffc080a94000 <bakaos::memory::global_heap::GLOBAL_ALLOCATOR>
      ffffffc0802035a8 <.Lpcrel_hi44> auipc	a1,0x892
      ffffffc0802035ac <.Lpcrel_hi44+0x4> addi	a1,a1,-1448 # ffffffc080a95000 <ebss>
      ffffffc0802035b0 <.Lpcrel_hi44+0x8> auipc	ra,0x0
      ffffffc0802035b4 <.Lpcrel_hi44+0xc> jalr	16(ra) # ffffffc0802035c0 <bakaos::clear_bss_fast>
  pc: 0xffffffc08020311a
    at: __kernel_init() in /home/runner/work/bakaos/bakaos/kernel/src/main.rs:125
    disassembly of the line:
      ffffffc08020311a <.Lpcrel_hi3+0x14> auipc	ra,0x0
      ffffffc08020311e <.Lpcrel_hi3+0x18> jalr	1150(ra) # ffffffc080203598 <bakaos::clear_bss>
  pc: 0xffffffc080200058
    at: __kernel_start_main() in /home/runner/work/bakaos/bakaos/kernel/src/main.rs:139
    disassembly of the line:
      ffffffc080200052 <__kernel_start_main+0x2> sd	ra,8(sp)
      ffffffc080200054 <__kernel_start_main+0x4> sd	s0,0(sp)
      ffffffc080200056 <__kernel_start_main+0x6> addi	s0,sp,16
      ffffffc080200058 <__kernel_start_main+0x8> auipc	ra,0x3
      ffffffc08020005c <__kernel_start_main+0xc> jalr	156(ra) # ffffffc0802030f4 <__kernel_init>
```
