use alloc::{rc::Weak, str, sync::Arc, vec::Vec};
use core::{cell::UnsafeCell, mem::MaybeUninit, sync::atomic::AtomicI32, task::Waker};

use address::VirtualAddress;
use hermit_sync::SpinMutex;
use paging::{MemorySpace, MemorySpaceBuilder};
use riscv::register::sstatus::{self, Sstatus, SPP};

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
                                      // reserved for float point registers
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
}

unsafe impl Sync for TaskControlBlock {}
unsafe impl Send for TaskControlBlock {}

impl TaskControlBlock {
    pub fn new(memory_space_builder: MemorySpaceBuilder) -> Arc<TaskControlBlock> {
        let trap_context = TaskTrapContext::new(&memory_space_builder);
        Arc::new(TaskControlBlock {
            task_id: tid::allocate_tid(),
            task_status: SpinMutex::new(TaskStatus::Uninitialized),
            exit_code: AtomicI32::new(0),
            memory_space: Arc::new(SpinMutex::new(memory_space_builder.memory_space)),
            parent: None,
            children: SpinMutex::new(Vec::new()),
            trap_context: UnsafeCell::new(trap_context),
            waker: UnsafeCell::new(MaybeUninit::uninit()),
        })
    }

    pub fn init(&self) {
        *self.task_status.lock() = TaskStatus::Ready;
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
