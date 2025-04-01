use abstractions::operations::IUsizeAlias;
use alloc::collections::BTreeMap;
use alloc::{string::String, sync::Arc, sync::Weak, vec::Vec};
use core::mem;
use core::sync::atomic::AtomicI32;
use core::{cell::UnsafeCell, mem::MaybeUninit, task::Waker};
use drivers::UserTaskTimer;
use filesystem_abstractions::FileDescriptorTable;
use platform_specific::{ITaskContext, TaskTrapContext};
use timing::TimeSpec;

use address::{IPageNum, VirtualAddress};
use hermit_sync::SpinMutex;
use paging::{
    MemoryMapFlags, MemoryMapProt, MemorySpace, MemorySpaceBuilder, PageTable, TaskMemoryMap,
};

use crate::{
    tid::{self, TrackedTaskId},
    TaskStatus,
};

fn create_task_context(builder: &MemorySpaceBuilder) -> TaskTrapContext {
    TaskTrapContext::new(
        builder.entry_pc.as_usize(),
        builder.stack_top.as_usize(),
        builder.argc,
        builder.argv_base.as_usize(),
        builder.envp_base.as_usize(),
    )
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
        let trap_context = create_task_context(&memory_space_builder);
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
            waker: UnsafeCell::new(MaybeUninit::new(Waker::noop().clone())),
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

pub type SyscallContext<'a> = platform_specific::SyscallContext<'a, Arc<TaskControlBlock>>;

impl TaskControlBlock {
    /// # Safety
    /// This function is only intended to use for `return_to_user`
    pub unsafe fn mut_trap_ctx(&self) -> &'static mut TaskTrapContext {
        &mut *self.trap_context.get()
    }

    pub fn to_syscall_context<'a>(self: &'a Arc<Self>) -> SyscallContext<'a> {
        SyscallContext::new(unsafe { self.mut_trap_ctx() }, self.clone())
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
        let memory_space_builder = MemorySpaceBuilder::from_raw(elf, path, args, envp)?;

        *unsafe { self.mut_trap_ctx() } = create_task_context(&memory_space_builder);

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
        pcb.fd_table.clear_exec();

        pcb.executable = Arc::new(String::from(path));

        Ok(())
    }

    pub fn fork_process(self: &Arc<TaskControlBlock>, share_vm: bool) -> Arc<TaskControlBlock> {
        let this_trap_ctx = *unsafe { self.mut_trap_ctx() };
        let this_pcb = self.pcb.lock();
        let mut memory_space = MemorySpace::clone_existing(&this_pcb.memory_space);

        // Handle memory mapping clone
        if share_vm {
            let this_pt = this_pcb.memory_space.page_table();
            let new_pt = memory_space.page_table_mut();

            for record in this_pcb.mmaps.records() {
                for vpn in record.page_area.iter() {
                    let vaddr = vpn.start_addr();

                    if let Ok((this_entry, size)) = this_pt.get_entry(vaddr) {
                        // Map to new page table

                        if let Ok(new_entry) = new_pt.get_create_entry(vaddr, size) {
                            *new_entry = *this_entry;
                        } else {
                            log::warn!(
                                "Can not creat pte when trying to clone memory mapping page"
                            );
                        }
                    } else {
                        log::warn!("Memory mapping record does not existing in page table");
                    }
                }
            }
        }

        let task_id = tid::allocate_tid();
        let tid = task_id.id();

        let forked = Arc::new(TaskControlBlock {
            task_id,
            task_status: SpinMutex::new(TaskStatus::Ready),
            children: SpinMutex::new(Vec::new()),
            trap_context: UnsafeCell::new(this_trap_ctx),
            waker: UnsafeCell::new(MaybeUninit::new(Waker::noop().clone())),
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
                mmaps: this_pcb.mmaps.clone(),
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

    pub fn fork_thread(self: &Arc<TaskControlBlock>) -> Arc<TaskControlBlock> {
        let this_trap_ctx = *unsafe { self.mut_trap_ctx() };
        let task_id = tid::allocate_tid();

        let forked = Arc::new(TaskControlBlock {
            task_id,
            task_status: SpinMutex::new(TaskStatus::Ready),
            children: SpinMutex::new(Vec::new()),
            trap_context: UnsafeCell::new(this_trap_ctx),
            waker: UnsafeCell::new(MaybeUninit::new(Waker::noop().clone())),
            start_time: UnsafeCell::new(unsafe { *self.start_time.get().as_ref().unwrap() }),
            timer: SpinMutex::new(UserTaskTimer::default()),
            kernel_timer: SpinMutex::new(UserTaskTimer::default()),
            exit_code: AtomicI32::new(0),
            pcb: self.pcb.clone(),
        });

        let mut pcb = self.pcb.lock();

        pcb.tasks
            .insert(forked.task_id.id(), Arc::downgrade(&forked));

        drop(pcb);

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
                page_table
                    .map_single(
                        vpn.start_addr(),
                        ppn.start_addr(),
                        page_table::PageSize::_4K,
                        flags,
                    )
                    .unwrap();
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
            page_table.unmap_single(vpn.start_addr()).unwrap();
        });

        mem::forget(pcb);

        page_table.flush_tlb();

        ret
    }
}

impl Drop for TaskControlBlock {
    fn drop(&mut self) {
        let p_waker = self.waker.get();

        unsafe {
            if let Some(waker) = p_waker.as_mut() {
                // Currently not checking vtable for simplicity
                if waker.assume_init_ref().data().is_null() {
                    waker.assume_init_drop();
                }
            }
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
