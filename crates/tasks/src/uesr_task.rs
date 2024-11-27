use abstractions::operations::IUsizeAlias;
use alloc::{string::String, sync::Arc, sync::Weak, vec::Vec};
use core::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{AtomicI32, AtomicUsize, Ordering},
    task::Waker,
};
use lock_api::MappedMutexGuard;
use timing::{TimeSpan, TimeSpec};

use address::VirtualAddress;
use hermit_sync::{RawSpinMutex, SpinMutex, SpinMutexGuard};
use paging::{MemorySpace, MemorySpaceBuilder, PageTable};
use riscv::register::sstatus::{self, Sstatus, FS, SPP};

use crate::{
    tid::{self, TrackedTaskId},
    TaskStatus,
};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GeneralRegisterContext {
    // Not saving x0 for simplicity
    pub ra: VirtualAddress,
    pub sp: VirtualAddress,
    pub gp: usize,
    pub tp: usize,
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub fp: usize, // s0
    pub s1: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FloatRegisterContext {
    pub f: [f64; 32], // 0 - 31
    pub fcsr: u32,
    pub dirty: bool,
    pub activated: bool,
}

impl FloatRegisterContext {
    pub fn snapshot(&mut self) {
        #[cfg(not(target_arch = "riscv64"))]
        panic!("Only riscv64 is supported");

        #[cfg(target_arch = "riscv64")]
        if self.dirty {
            self.dirty = false;

            unsafe {
                core::arch::asm!(
                    "fsd    f0,     0*8(a0)",
                    "fsd    f1,     1*8(a0)",
                    "fsd    f2,     2*8(a0)",
                    "fsd    f3,     3*8(a0)",
                    "fsd    f4,     4*8(a0)",
                    "fsd    f5,     5*8(a0)",
                    "fsd    f6,     6*8(a0)",
                    "fsd    f7,     7*8(a0)",
                    "fsd    f8,     8*8(a0)",
                    "fsd    f9,     9*8(a0)",
                    "fsd    f10,   10*8(a0)",
                    "fsd    f11,   11*8(a0)",
                    "fsd    f12,   12*8(a0)",
                    "fsd    f13,   13*8(a0)",
                    "fsd    f14,   14*8(a0)",
                    "fsd    f15,   15*8(a0)",
                    "fsd    f16,   16*8(a0)",
                    "fsd    f17,   17*8(a0)",
                    "fsd    f18,   18*8(a0)",
                    "fsd    f19,   19*8(a0)",
                    "fsd    f20,   20*8(a0)",
                    "fsd    f21,   21*8(a0)",
                    "fsd    f22,   22*8(a0)",
                    "fsd    f23,   23*8(a0)",
                    "fsd    f24,   24*8(a0)",
                    "fsd    f25,   25*8(a0)",
                    "fsd    f26,   26*8(a0)",
                    "fsd    f27,   27*8(a0)",
                    "fsd    f28,   28*8(a0)",
                    "fsd    f29,   29*8(a0)",
                    "fsd    f30,   30*8(a0)",
                    "fsd    f31,   31*8(a0)",
                    "csrr   t0,    fcsr",
                    "sw     t0,    31*8(a0)",
                    in("a0") self,
                    options(nostack, nomem)
                );
            }
        }
    }

    pub fn restore(&mut self) {
        #[cfg(not(target_arch = "riscv64"))]
        panic!("Only riscv64 is supported");

        #[cfg(target_arch = "riscv64")]
        if !self.activated {
            self.activated = true;

            unsafe {
                core::arch::asm!(
                    "fld    f0,     0*8(a0)",
                    "fld    f1,     1*8(a0)",
                    "fld    f2,     2*8(a0)",
                    "fld    f3,     3*8(a0)",
                    "fld    f4,     4*8(a0)",
                    "fld    f5,     5*8(a0)",
                    "fld    f6,     6*8(a0)",
                    "fld    f7,     7*8(a0)",
                    "fld    f8,     8*8(a0)",
                    "fld    f9,     9*8(a0)",
                    "fld    f10,   10*8(a0)",
                    "fld    f11,   11*8(a0)",
                    "fld    f12,   12*8(a0)",
                    "fld    f13,   13*8(a0)",
                    "fld    f14,   14*8(a0)",
                    "fld    f15,   15*8(a0)",
                    "fld    f16,   16*8(a0)",
                    "fld    f17,   17*8(a0)",
                    "fld    f18,   18*8(a0)",
                    "fld    f19,   19*8(a0)",
                    "fld    f20,   20*8(a0)",
                    "fld    f21,   21*8(a0)",
                    "fld    f22,   22*8(a0)",
                    "fld    f23,   23*8(a0)",
                    "fld    f24,   24*8(a0)",
                    "fld    f25,   25*8(a0)",
                    "fld    f26,   26*8(a0)",
                    "fld    f27,   27*8(a0)",
                    "fld    f28,   28*8(a0)",
                    "fld    f29,   29*8(a0)",
                    "fld    f30,   30*8(a0)",
                    "fld    f31,   31*8(a0)",
                    "lw     t0,    32*8(a0)",
                    "csrw   fcsr,  t0",
                    in("a0") self,
                    options(nostack, nomem)
                );
            }
        }
    }

    pub fn on_trap(&mut self, sstatus: Sstatus) {
        self.dirty |= sstatus.fs() == FS::Dirty;
    }

    pub fn activate_restore(&mut self) {
        self.activated = true;
        self.restore();
    }

    pub fn deactivate(&mut self) {
        self.snapshot();
        self.activated = false;
    }
}

// Saved context for coroutine
// Following calling convention that only caller-saved registers are saved
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CoroutineSavedContext {
    pub saved: [usize; 12], // 36 - 47
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TaskTrapContext {
    pub regs: GeneralRegisterContext, // 0 - 30
    pub sstatus: usize,               // 31
    pub sepc: VirtualAddress,         // 32
    pub ksp: VirtualAddress,          // kernel stack pointer, 33
    pub kra: VirtualAddress,          // kernel return address, 34
    pub ktp: usize,                   // kernel tp, 35
    pub kregs: CoroutineSavedContext, // 36 - 47
    pub fregs: FloatRegisterContext,
}

impl TaskTrapContext {
    pub fn new(memory_space_builder: &MemorySpaceBuilder) -> Self {
        const SIZE: usize = core::mem::size_of::<TaskTrapContext>();
        let mut ctx = unsafe { core::mem::transmute::<[u8; SIZE], TaskTrapContext>([0; SIZE]) };
        ctx.sepc = memory_space_builder.entry_pc;
        ctx.regs.sp = memory_space_builder.stack_top;

        let sstatus = sstatus::read();
        unsafe {
            sstatus::set_spp(SPP::User);
            sstatus::set_sum();
        }

        ctx.sstatus = unsafe { core::mem::transmute::<Sstatus, usize>(sstatus) };

        ctx
    }
}

pub struct TaskControlBlock {
    pub task_id: TrackedTaskId,
    pub task_status: SpinMutex<TaskStatus>,
    pub exit_code: AtomicI32,
    pub memory_space: Arc<SpinMutex<MemorySpace>>,
    pub parent: Option<Arc<Weak<TaskControlBlock>>>,
    pub children: SpinMutex<Vec<Arc<TaskControlBlock>>>,
    pub trap_context: UnsafeCell<TaskTrapContext>,
    pub waker: UnsafeCell<MaybeUninit<Waker>>,
    pub stats: SpinMutex<UserTaskStatistics>,
    pub start_time: UnsafeCell<MaybeUninit<TimeSpec>>,
    pub timer: SpinMutex<UserTaskTimer>,
    pub kernel_timer: SpinMutex<UserTaskTimer>,
    pub brk_pos: AtomicUsize,
    pub cwd: UnsafeCell<String>,
}

unsafe impl Sync for TaskControlBlock {}
unsafe impl Send for TaskControlBlock {}

impl TaskControlBlock {
    pub fn new(memory_space_builder: MemorySpaceBuilder) -> Arc<TaskControlBlock> {
        let trap_context = TaskTrapContext::new(&memory_space_builder);
        let brk_pos = memory_space_builder.memory_space.brk_start().as_usize();
        Arc::new(TaskControlBlock {
            task_id: tid::allocate_tid(),
            task_status: SpinMutex::new(TaskStatus::Uninitialized),
            exit_code: AtomicI32::new(0),
            memory_space: Arc::new(SpinMutex::new(memory_space_builder.memory_space)),
            parent: None,
            children: SpinMutex::new(Vec::new()),
            trap_context: UnsafeCell::new(trap_context),
            waker: UnsafeCell::new(MaybeUninit::uninit()),
            stats: SpinMutex::new(UserTaskStatistics::default()),
            start_time: UnsafeCell::new(MaybeUninit::uninit()),
            timer: SpinMutex::new(UserTaskTimer::default()),
            kernel_timer: SpinMutex::new(UserTaskTimer::default()),
            brk_pos: AtomicUsize::new(brk_pos),
            cwd: UnsafeCell::new(String::new()),
        })
    }

    pub fn init(&self) {
        *self.task_status.lock() = TaskStatus::Ready;
    }

    pub fn borrow_page_table(&self) -> MappedMutexGuard<RawSpinMutex, PageTable> {
        let memsapce = unsafe { self.memory_space.make_guard_unchecked() };
        SpinMutexGuard::map(memsapce, |m| m.page_table_mut())
    }
}

impl TaskControlBlock {
    pub fn mut_trap_ctx(&self) -> &'static mut TaskTrapContext {
        unsafe { &mut *self.trap_context.get() }
    }
}

impl TaskControlBlock {
    pub fn is_uninitialized(&self) -> bool {
        *self.task_status.lock() == TaskStatus::Uninitialized
    }

    pub fn is_ready(&self) -> bool {
        *self.task_status.lock() == TaskStatus::Ready
    }

    pub fn is_running(&self) -> bool {
        *self.task_status.lock() == TaskStatus::Running
    }

    pub fn is_exited(&self) -> bool {
        *self.task_status.lock() >= TaskStatus::Exited
    }
}

impl TaskControlBlock {
    pub fn fork_process(self: &Arc<TaskControlBlock>) -> Arc<TaskControlBlock> {
        let this_trap_ctx = self.mut_trap_ctx().clone();
        let this_brk_pos = self.brk_pos.load(Ordering::Relaxed);
        let memory_space = MemorySpace::clone_existing(&self.memory_space.lock());

        Arc::new(TaskControlBlock {
            task_id: tid::allocate_tid(),
            task_status: SpinMutex::new(self.task_status.lock().clone()),
            exit_code: AtomicI32::new(self.exit_code.load(Ordering::Relaxed)),
            memory_space: Arc::new(SpinMutex::new(memory_space)),
            parent: Some(Arc::new(Arc::downgrade(self))),
            children: SpinMutex::new(Vec::new()),
            trap_context: UnsafeCell::new(this_trap_ctx),
            waker: UnsafeCell::new(MaybeUninit::uninit()),
            stats: SpinMutex::new(self.stats.lock().clone()),
            start_time: UnsafeCell::new(unsafe { self.start_time.get().as_ref().unwrap().clone() }),
            timer: SpinMutex::new(UserTaskTimer::default()),
            kernel_timer: SpinMutex::new(UserTaskTimer::default()),
            brk_pos: AtomicUsize::new(this_brk_pos),
            cwd: UnsafeCell::new(unsafe { self.cwd.get().as_ref().unwrap().clone() }),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct UserTaskStatistics {
    pub external_interrupts: usize,
    pub timer_interrupts: usize,
    pub software_interrupts: usize,
    pub exceptions: usize,
    pub syscalls: usize,
}

#[derive(Debug, Clone)]
pub struct UserTaskTimer {
    pub total: TimeSpan,
    pub start: Option<TimeSpec>,
}

impl Default for UserTaskTimer {
    fn default() -> Self {
        UserTaskTimer {
            total: TimeSpan::zero(),
            start: None,
        }
    }
}
