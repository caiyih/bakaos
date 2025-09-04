use core::cell::RefCell;

use alloc::{
    string::String,
    sync::{Arc, Weak},
    vec,
    vec::Vec,
};

use abstractions::operations::IUsizeAlias;
use filesystem_abstractions::FileDescriptorTable;
use hermit_sync::SpinMutex;
use linux_loader::LinuxLoader;
use linux_task_abstractions::ILinuxProcess;
use memory_space::MemorySpace;
use mmu_abstractions::IMMU;
use platform_specific::{ITaskContext, TaskTrapContext};
use task_abstractions::{IProcess, ITask, ITaskIdAllocator, TaskId};

use crate::{id_allocator::TaskIdAllocator, LinuxTask};

pub struct LinuxProcess {
    pid: TaskId,
    pgid: u32,
    id_allocator: Arc<dyn ITaskIdAllocator>,
    parent: Option<Arc<dyn IProcess>>,
    threads: SpinMutex<Vec<Arc<dyn ITask>>>,
    children: SpinMutex<Vec<Weak<dyn IProcess>>>,
    memory_space: SpinMutex<MemorySpace>,
    mmu: RefCell<Arc<SpinMutex<dyn IMMU>>>,
    fd_table: SpinMutex<FileDescriptorTable>,
    working_directory: SpinMutex<String>,
    exit_code: SpinMutex<Option<u8>>,
}

unsafe impl Send for LinuxProcess {}
unsafe impl Sync for LinuxProcess {}

impl LinuxProcess {
    /// Create a new Linux process and its initial (main) thread.
    ///
    /// This consumes the provided `LinuxLoader`, allocates process and thread IDs,
    /// builds the main thread's trap context from the loader, registers the kernel
    /// address space for the process page table, clones the loader's MMU into the
    /// process, and returns the handle to the main `LinuxTask`. The loader's
    /// `memory_space` is moved into the new process.
    ///
    /// # Parameters
    ///
    /// - `builder`: the `LinuxLoader` whose entry, stack, argv/env and memory space
    ///   are used to initialize the main thread and process. It is consumed.
    /// - `tid`: an initial task id used to seed the internal `TaskIdAllocator`.
    ///
    /// # Returns
    ///
    /// An `Arc<LinuxTask>` pointing to the newly created main thread for the process.
    ///
    /// # Notes
    ///
    /// - The returned `LinuxTask`'s `process` field is populated to reference the
    ///   newly created process.
    /// - This function performs platform-specific kernel-area registration for the
    ///   process page table as a side effect.
    ///
    /// # Examples
    ///
    /// ```
    /// // Given a prepared `loader: LinuxLoader` and an initial tid seed:
    /// // let main_thread = LinuxProcess::new(loader, 1);
    /// // assert_eq!(main_thread.process().pid(), /* some pid value */);
    /// ```
    #[allow(clippy::new_ret_no_self)]
    pub fn new(builder: LinuxLoader, tid: u32) -> Arc<LinuxTask> {
        let id_allocator = TaskIdAllocator::new(tid);

        let tid = id_allocator.clone().alloc();
        let trap_ctx = create_task_context(&builder);
        let main_thread = LinuxTask::new(tid, trap_ctx);

        let pid = id_allocator.clone().alloc();

        Self::register_kernel_area_for_pt(&builder.memory_space);

        let mmu = builder.memory_space.mmu().clone();

        let process = Arc::new(Self {
            pgid: *pid,
            pid,
            id_allocator,
            parent: None,
            threads: SpinMutex::new(Vec::new()),
            children: SpinMutex::new(Vec::new()),
            mmu: RefCell::new(mmu),
            memory_space: SpinMutex::new(builder.memory_space),
            fd_table: SpinMutex::new(FileDescriptorTable::new()),
            working_directory: SpinMutex::new(String::new()),
            exit_code: SpinMutex::new(None),
        });

        unsafe { *main_thread.process.get().as_mut().unwrap() = Some(process) };

        main_thread
    }

    fn register_kernel_area_for_pt(space: &MemorySpace) {
        let _pt = space.mmu().lock();

        #[cfg_accessible(platform_specific::register_kernel_area_for_pt)]
        platform_specific::register_kernel_area_for_pt(_pt.platform_payload());
    }
}

impl IProcess for LinuxProcess {
    fn pid(&self) -> u32 {
        *self.pid
    }

    fn pgid(&self) -> u32 {
        self.pgid
    }

    fn parent(&self) -> Option<Arc<dyn IProcess>> {
        self.parent.clone()
    }

    fn threads(&self) -> Vec<Arc<dyn ITask>> {
        self.threads.lock().clone()
    }

    fn children(&self) -> Vec<Arc<dyn IProcess>> {
        self.children
            .lock()
            .iter()
            .filter_map(|w| w.upgrade())
            .collect::<Vec<_>>()
    }

    fn memory_space(&self) -> &SpinMutex<MemorySpace> {
        &self.memory_space
    }

    fn fd_table(&self) -> &SpinMutex<FileDescriptorTable> {
        &self.fd_table
    }

    fn working_directory(&self) -> String {
        self.working_directory.lock().clone()
    }

    fn exit_code(&self) -> &SpinMutex<Option<u8>> {
        &self.exit_code
    }

    fn alloc_id(&self) -> TaskId {
        self.id_allocator.clone().alloc()
    }

    fn push_thread(&self, task: Arc<dyn ITask>) {
        self.threads.lock().push(task);
    }
}

impl ILinuxProcess for LinuxProcess {
    /// Replace the process memory image with `mem`, keep only the calling thread, and clear exec-related FD state.
    ///
    /// This updates the process's MMU to `mem.mmu()` and replaces the process's owned `MemorySpace` with `mem`.
    /// It then locates the thread whose `tid()` equals `calling` and replaces the process's thread list with a
    /// single clone of that thread. Finally, it clears any exec-specific state in the process's file descriptor table.
    ///
    /// Panics:
    /// - Panics if no thread with the given `calling` tid exists in the process (the function uses `unwrap`).
    ///
    /// Parameters:
    /// - `mem`: the new memory space that becomes this process's address space.
    /// - `calling`: the tid of the thread that should remain as the sole thread after exec.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Replace the process image with `new_space`, keeping thread 42 as the only thread:
    /// // process.execve(new_space, 42);
    /// ```
    fn execve(&self, mem: MemorySpace, calling: u32) {
        *self.mmu.borrow_mut() = mem.mmu().clone();
        *self.memory_space.lock() = mem;

        let mut threads = self.threads.lock();

        let calling = threads.iter().find(|t| t.tid() == calling).unwrap();

        *threads = vec![calling.clone()];

        self.fd_table.lock().clear_exec();
    }
}

/// Build a TaskTrapContext from a LinuxLoader's entry, stack, and argument/environment layout.
///
/// The returned TaskTrapContext is initialized using:
/// - `entry_pc` as the initial program counter,
/// - `stack_top` as the initial stack pointer,
/// - the loader's `ctx.argv.len()` as `argc`,
/// - `argv_base` and `envp_base` as the base addresses for `argv` and `envp`.
///
/// # Examples
///
/// ```no_run
/// // Given a prepared `loader: LinuxLoader`, create the initial trap context:
/// let ctx = create_task_context(&loader);
/// // `ctx` is ready to be used for a new task's trap frame / bootstrap.
/// ```
fn create_task_context(loader: &LinuxLoader) -> TaskTrapContext {
    TaskTrapContext::new(
        loader.entry_pc.as_usize(),
        loader.stack_top.as_usize(),
        loader.ctx.argv.len(),
        loader.argv_base.as_usize(),
        loader.envp_base.as_usize(),
    )
}
