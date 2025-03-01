use abstractions::operations::IUsizeAlias;
use alloc::collections::BTreeMap;
use alloc::{string::String, sync::Arc, sync::Weak, vec::Vec};
use core::mem;
use core::sync::atomic::AtomicI32;
use core::{cell::UnsafeCell, mem::MaybeUninit, task::Waker};
use filesystem_abstractions::FileDescriptorTable;
use timing::{TimeSpan, TimeSpec};

use address::VirtualAddress;
use hermit_sync::SpinMutex;
use paging::{
    MemoryMapFlags, MemoryMapProt, MemorySpace, MemorySpaceBuilder, PageTable, TaskMemoryMap,
};
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

        ctx.regs.a0 = memory_space_builder.argc;
        ctx.regs.a1 = memory_space_builder.argv_base.as_usize();
        ctx.regs.a2 = memory_space_builder.envp_base.as_usize();

        ctx
    }
}

pub struct ProcessControlBlock {
    pub parent: Option<Weak<TaskControlBlock>>,
    pub id: usize,
    pub brk_pos: usize,
    pub exit_code: i32,
    pub status: TaskStatus,
    pub stats: UserTaskStatistics,
    pub memory_space: MemorySpace,
    pub cwd: String,
    pub fd_table: FileDescriptorTable,
    pub mmaps: TaskMemoryMap,
    pub futex_queue: FutexQueue,
    pub tasks: BTreeMap<usize, Weak<TaskControlBlock>>,
    pub executable: Arc<String>,
    pub command_line: Arc<Vec<String>>,
}

impl ProcessControlBlock {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(memory_space_builder: MemorySpaceBuilder) -> Arc<TaskControlBlock> {
        let trap_context = TaskTrapContext::new(&memory_space_builder);
        let task_id = tid::allocate_tid();
        let tid = task_id.id();

        let pcb = ProcessControlBlock {
            parent: None,
            id: tid,
            brk_pos: memory_space_builder.memory_space.brk_start().as_usize(),
            exit_code: 0,
            status: TaskStatus::Uninitialized,
            stats: UserTaskStatistics::default(),
            cwd: String::new(),
            fd_table: FileDescriptorTable::new(tid),
            mmaps: TaskMemoryMap::default(),
            memory_space: memory_space_builder.memory_space,
            futex_queue: FutexQueue::default(),
            tasks: BTreeMap::new(),
            executable: Arc::new(memory_space_builder.executable),
            command_line: Arc::new(memory_space_builder.command_line),
        };

        let tcb = Arc::new(TaskControlBlock {
            task_id,
            task_status: SpinMutex::new(TaskStatus::Uninitialized),
            children: SpinMutex::new(Vec::new()),
            trap_context: UnsafeCell::new(trap_context),
            waker: UnsafeCell::new(MaybeUninit::uninit()),
            start_time: UnsafeCell::new(MaybeUninit::uninit()),
            timer: SpinMutex::new(UserTaskTimer::default()),
            kernel_timer: SpinMutex::new(UserTaskTimer::default()),
            exit_code: AtomicI32::new(0),
            pcb: Arc::new(SpinMutex::new(pcb)),
        });

        tcb.pcb.lock().tasks.insert(tid, Arc::downgrade(&tcb));

        tcb
    }
}

pub struct TaskControlBlock {
    pub task_id: TrackedTaskId,
    pub task_status: SpinMutex<TaskStatus>,
    pub children: SpinMutex<Vec<Arc<TaskControlBlock>>>,
    pub trap_context: UnsafeCell<TaskTrapContext>,
    pub waker: UnsafeCell<MaybeUninit<Waker>>,
    pub start_time: UnsafeCell<MaybeUninit<TimeSpec>>,
    pub timer: SpinMutex<UserTaskTimer>,
    pub kernel_timer: SpinMutex<UserTaskTimer>,
    pub exit_code: AtomicI32,
    pub pcb: Arc<SpinMutex<ProcessControlBlock>>,
}

unsafe impl Sync for TaskControlBlock {}
unsafe impl Send for TaskControlBlock {}

impl TaskControlBlock {
    pub fn init(&self) {
        let mut task_status = self.task_status.lock();
        if let core::cmp::Ordering::Less = task_status.cmp(&TaskStatus::Ready) {
            // For process that is execve'd, it is already in Running state
            // So we don't need to change it to Ready
            *task_status = TaskStatus::Ready;
        }
    }

    pub fn borrow_page_table(&self) -> &PageTable {
        unsafe {
            self.pcb
                .data_ptr()
                .as_ref()
                .unwrap()
                .memory_space
                .page_table()
        }
    }

    pub fn waker(&self) -> &Waker {
        unsafe { self.waker.get().as_ref().unwrap().assume_init_ref() }
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
    pub fn execve(
        self: &Arc<TaskControlBlock>,
        elf: &[u8],
        path: &str,
        args: &[&str],
        envp: &[&str],
    ) -> Result<(), &'static str> {
        let mut memory_space_builder = MemorySpaceBuilder::from_elf(elf, path)?;

        memory_space_builder.init_stack(args, envp);

        *self.mut_trap_ctx() = TaskTrapContext::new(&memory_space_builder);

        *self.timer.lock() = UserTaskTimer::default();
        *self.kernel_timer.lock() = UserTaskTimer::default();

        let mut pcb = self.pcb.lock();
        pcb.brk_pos = memory_space_builder.memory_space.brk_start().as_usize();

        pcb.stats = UserTaskStatistics::default();

        pcb.memory_space = memory_space_builder.memory_space;

        let self_tid = self.task_id.id();
        for (tid, weak) in pcb.tasks.iter() {
            if *tid == self_tid {
                continue;
            }

            if let Some(thread) = weak.upgrade() {
                *thread.task_status.lock() = TaskStatus::Exited;
            }
        }

        pcb.tasks.clear();
        pcb.tasks.insert(self_tid, Arc::downgrade(self));
        pcb.futex_queue.clear();

        unsafe { self.borrow_page_table().activate() };
        self.init();

        // TODO: Handle file descriptor table with FD_CLOEXEC flag

        Ok(())
    }

    pub fn fork_process(self: &Arc<TaskControlBlock>) -> Arc<TaskControlBlock> {
        let this_trap_ctx = *self.mut_trap_ctx();
        let this_pcb = self.pcb.lock();
        let memory_space = MemorySpace::clone_existing(&this_pcb.memory_space);
        let task_id = tid::allocate_tid();
        let tid = task_id.id();

        let forked = Arc::new(TaskControlBlock {
            task_id,
            task_status: SpinMutex::new(TaskStatus::Ready),
            children: SpinMutex::new(Vec::new()),
            trap_context: UnsafeCell::new(this_trap_ctx),
            waker: UnsafeCell::new(MaybeUninit::uninit()),
            // stats: SpinMutex::new(self.stats.lock().clone()),
            start_time: UnsafeCell::new(unsafe { *self.start_time.get().as_ref().unwrap() }),
            timer: SpinMutex::new(UserTaskTimer::default()),
            kernel_timer: SpinMutex::new(UserTaskTimer::default()),
            exit_code: AtomicI32::new(self.exit_code.load(core::sync::atomic::Ordering::Relaxed)),
            pcb: Arc::new(SpinMutex::new(ProcessControlBlock {
                parent: Some(Arc::downgrade(self)),
                id: tid,
                brk_pos: this_pcb.brk_pos,
                exit_code: this_pcb.exit_code,
                status: this_pcb.status,
                stats: this_pcb.stats.clone(),
                memory_space,
                cwd: this_pcb.cwd.clone(),
                fd_table: this_pcb.fd_table.clone_for(tid),
                mmaps: TaskMemoryMap::default(),
                futex_queue: FutexQueue::default(),
                tasks: BTreeMap::new(),
                executable: this_pcb.executable.clone(),
                command_line: this_pcb.command_line.clone(),
            })),
        });

        let mut forked_pcb = forked.pcb.lock();

        forked_pcb.tasks.insert(tid, Arc::downgrade(&forked));

        // FIXME: Spawn other threads and inserts

        drop(forked_pcb);

        forked
    }
}

impl TaskControlBlock {
    pub fn mmap(
        self: &Arc<TaskControlBlock>,
        fd: usize,
        flags: MemoryMapFlags,
        prot: MemoryMapProt,
        offset: usize,
        length: usize,
    ) -> Option<VirtualAddress> {
        let mut pcb = self.pcb.lock();

        let fd = pcb.fd_table.get(fd).cloned();

        let mut pcb_ = unsafe { self.pcb.make_guard_unchecked() };
        let page_table = pcb_.memory_space.page_table_mut();

        let ret = pcb.mmaps.mmap(
            fd.as_ref(),
            flags,
            prot,
            offset,
            length,
            |vpn, ppn, flags| {
                page_table.map_single(vpn, ppn, flags);
            },
        );

        mem::forget(pcb);

        page_table.flush_tlb();

        ret
    }

    pub fn munmap(self: &Arc<TaskControlBlock>, addr: VirtualAddress, length: usize) -> bool {
        let mut pcb = self.pcb.lock();

        let mut pcb_ = unsafe { self.pcb.make_guard_unchecked() };
        let page_table = pcb_.memory_space.page_table_mut();

        let ret = pcb.mmaps.munmap(addr, length, |vpn| {
            page_table.unmap_single(vpn);
        });

        mem::forget(pcb);

        page_table.flush_tlb();

        ret
    }
}

impl Drop for TaskControlBlock {
    fn drop(&mut self) {
        unsafe {
            self.waker.get().as_mut().unwrap().assume_init_drop();
        }
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

#[derive(Default)]
pub struct FutexQueue {
    inner: BTreeMap<VirtualAddress, BTreeMap<usize, Waker>>,
}

impl FutexQueue {
    fn clear(&mut self) {
        self.inner.clear();
    }
}

impl FutexQueue {
    pub fn enqueue(&mut self, addr: VirtualAddress, tid: usize, waker: &Waker) {
        self.inner
            .entry(addr)
            .or_default()
            .insert(tid, waker.clone());
    }

    pub fn notify_woken(&mut self, addr: VirtualAddress, tid: usize) {
        self.inner.entry(addr).and_modify(|map| {
            map.remove(&tid);
        });
    }

    pub fn wake(&mut self, addr: VirtualAddress, n: usize) -> usize {
        if n == 0 {
            return 0;
        }

        match self.inner.get_mut(&addr) {
            Some(map) => {
                let mut count = 0;

                while let Some((_tid, waker)) = map.pop_first() {
                    #[cfg(debug_assertions)]
                    log::debug!("[FutexQueue] waking task {}", _tid);

                    waker.wake();
                    count += 1;

                    if count == n {
                        break;
                    }
                }

                count
            }
            None => 0,
        }
    }

    pub fn requeue(
        &mut self,
        prev_addr: VirtualAddress,
        new_addr: VirtualAddress,
        n_wake: usize,
        n_requeue: usize,
    ) -> usize {
        if prev_addr == new_addr {
            return 0;
        }

        let woken = self.wake(prev_addr, n_wake);

        let mut requeued = 0;

        if let Some(mut prev_map) = self.inner.remove(&prev_addr) {
            let new_map = self.inner.entry(new_addr).or_default();

            while requeued < n_requeue {
                match prev_map.pop_first() {
                    Some((tid, waker)) => {
                        #[cfg(debug_assertions)]
                        log::debug!("[FutexQueue] requeuing task {}", tid);

                        new_map.insert(tid, waker);
                    }
                    None => break,
                }

                requeued += 1;
            }

            if !prev_map.is_empty() {
                self.inner.insert(prev_addr, prev_map);
            }
        }

        woken + requeued
    }
}
