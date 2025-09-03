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
    fn execve(&self, mem: MemorySpace, calling: u32) {
        *self.mmu.borrow_mut() = mem.mmu().clone();
        *self.memory_space.lock() = mem;

        let mut threads = self.threads.lock();

        let calling = threads.iter().find(|t| t.tid() == calling).unwrap();

        *threads = vec![calling.clone()];

        self.fd_table.lock().clear_exec();
    }
}

fn create_task_context(builder: &LinuxLoader) -> TaskTrapContext {
    TaskTrapContext::new(
        builder.entry_pc.as_usize(),
        builder.stack_top.as_usize(),
        builder.argc,
        builder.argv_base.as_usize(),
        builder.envp_base.as_usize(),
    )
}
