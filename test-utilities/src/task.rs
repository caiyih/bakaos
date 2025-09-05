use core::cell::UnsafeCell;
use std::sync::{Arc, Weak};

use filesystem_abstractions::FileDescriptorTable;
use hermit_sync::SpinMutex;
use linux_task_abstractions::{ILinuxProcess, ILinuxTask};
use memory_space::MemorySpace;
use platform_specific::TaskTrapContext;
use task_abstractions::{status::TaskStatus, IProcess, ITask, UserTaskStatistics};
use trap_abstractions::ITaskTrapContext;

pub struct TestTask {
    tid: u32,
    tgid: u32,
    process: Option<Arc<dyn ILinuxProcess>>,
    status: SpinMutex<TaskStatus>,
    stats: UserTaskStatistics,
    trap_ctx: UnsafeCell<TaskTrapContext>,
}

unsafe impl Send for TestTask {}
unsafe impl Sync for TestTask {}

impl Default for TestTask {
    fn default() -> Self {
        Self::new()
    }
}

impl TestTask {
    pub fn new() -> TestTask {
        TestTask {
            tid: 0,
            tgid: 0,
            process: None,
            status: SpinMutex::new(TaskStatus::Running),
            stats: UserTaskStatistics::default(),
            trap_ctx: UnsafeCell::new(TaskTrapContext::default()),
        }
    }

    pub fn build(self) -> Arc<dyn ILinuxTask> {
        Arc::new(self)
    }

    pub fn with_tid(mut self, tid: u32) -> Self {
        self.tid = tid;
        self
    }

    pub fn with_tgid(mut self, tgid: u32) -> Self {
        self.tgid = tgid;
        self
    }

    pub fn with_linux_process(mut self, process: Option<Arc<dyn ILinuxProcess>>) -> Self {
        self.process = process;
        self
    }

    pub fn with_status(self, status: TaskStatus) -> Self {
        *self.status.lock() = status;
        self
    }

    pub fn with_stats(mut self, stats: UserTaskStatistics) -> Self {
        self.stats = stats;
        self
    }
}

impl ITask for TestTask {
    fn tid(&self) -> u32 {
        self.tid
    }

    fn tgid(&self) -> u32 {
        self.tgid
    }

    fn process(&self) -> std::sync::Arc<dyn task_abstractions::IProcess> {
        self.process.as_ref().unwrap().clone()
    }

    fn status(&self) -> TaskStatus {
        *self.status.lock()
    }

    fn stats(&self) -> UserTaskStatistics {
        self.stats.clone()
    }

    fn update_status(&self, status: TaskStatus) -> TaskStatus {
        let mut locked = self.status.lock();

        let prev = *locked;
        *locked = status;

        prev
    }

    fn trap_context(&self) -> &dyn ITaskTrapContext {
        unsafe { self.trap_ctx.get().as_ref().unwrap() }
    }

    fn trap_context_mut(&self) -> &mut dyn ITaskTrapContext {
        unsafe { self.trap_ctx.get().as_mut().unwrap() }
    }

    fn fork_thread(&self) -> Arc<dyn ITask> {
        let mut trap_ctx = TaskTrapContext::default();
        trap_ctx.copy_from(self.trap_context());

        Arc::new(TestTask {
            tid: self.tid, // TODO: allocate a new one
            tgid: self.tgid,
            process: self.process.clone(),
            status: SpinMutex::new(*self.status.lock()),
            stats: self.stats.clone(),
            trap_ctx: UnsafeCell::new(trap_ctx),
        })
    }

    fn fork_process(&self) -> Arc<dyn ITask> {
        unimplemented!(
            "TestTask is intended for light-weight mock testing. Use task::Task instead, which also supports unit test"
        )
    }
}

impl ILinuxTask for TestTask {
    fn linux_process(&self) -> Arc<dyn ILinuxProcess> {
        self.process.clone().unwrap()
    }
}

pub struct TestProcess {
    pub pid: u32,
    pub pgid: u32,
    pub parent: Option<Arc<dyn IProcess>>,
    pub threads: SpinMutex<Vec<Arc<dyn ITask>>>,
    pub children: SpinMutex<Vec<Weak<dyn IProcess>>>,
    pub memory_space: Option<SpinMutex<MemorySpace>>,
    pub fd_table: Option<SpinMutex<FileDescriptorTable>>,
    pub working_directory: String,
    pub main_thread: Option<TestTask>,
    pub exit_code: SpinMutex<Option<u8>>,
}

unsafe impl Send for TestProcess {}
unsafe impl Sync for TestProcess {}

impl Default for TestProcess {
    fn default() -> Self {
        Self::new()
    }
}

impl TestProcess {
    pub fn new() -> TestProcess {
        Self {
            pid: 0,
            pgid: 0,
            parent: None,
            threads: SpinMutex::new(Vec::new()),
            children: SpinMutex::new(Vec::new()),
            memory_space: None,
            fd_table: None,
            working_directory: String::new(),
            main_thread: Some(TestTask::new()),
            exit_code: SpinMutex::new(None),
        }
    }

    pub fn configure_main_thread(&mut self, mut callback: impl FnMut(&mut TestTask)) {
        callback(self.main_thread.as_mut().unwrap())
    }

    pub fn build(mut self) -> (Arc<dyn ILinuxProcess>, Arc<dyn ILinuxTask>) {
        let main_thread = self.main_thread.take().unwrap();

        let process = Arc::new(self);

        let main_thread = main_thread
            .with_linux_process(Some(process.clone()))
            .build();

        process.push_thread(main_thread.clone());

        (process, main_thread)
    }

    pub fn with_pid(mut self, pid: u32) -> Self {
        self.pid = pid;
        self
    }

    pub fn with_pgid(mut self, pgid: u32) -> Self {
        self.pgid = pgid;
        self
    }

    pub fn with_parent(mut self, parent: Option<Arc<dyn IProcess>>) -> Self {
        self.parent = parent;
        self
    }

    pub fn with_threads(mut self, threads: Vec<Arc<dyn ITask>>) -> Self {
        self.threads = SpinMutex::new(threads);
        self
    }

    pub fn with_children(mut self, children: Vec<Weak<dyn IProcess>>) -> Self {
        self.children = SpinMutex::new(children);
        self
    }

    pub fn with_memory_space(mut self, memory_space: Option<MemorySpace>) -> Self {
        self.memory_space = memory_space.map(SpinMutex::new);
        self
    }

    pub fn with_fd_table(mut self, fd_table: Option<FileDescriptorTable>) -> Self {
        self.fd_table = fd_table.map(SpinMutex::new);
        self
    }

    pub fn with_cwd(mut self, cwd: String) -> Self {
        self.working_directory = cwd;
        self
    }
}

impl IProcess for TestProcess {
    fn pid(&self) -> u32 {
        self.pid
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
        self.memory_space.as_ref().unwrap()
    }

    fn fd_table(&self) -> &SpinMutex<FileDescriptorTable> {
        self.fd_table.as_ref().unwrap()
    }

    fn working_directory(&self) -> String {
        self.working_directory.clone()
    }

    fn exit_code(&self) -> &SpinMutex<Option<u8>> {
        &self.exit_code
    }

    fn alloc_id(&self) -> task_abstractions::TaskId {
        unimplemented!(
            "TestProcess is intended for light-weight mock testing. Use task::Process instead, which also supports unit test"
        )
    }

    fn push_thread(&self, task: Arc<dyn ITask>) {
        self.threads.lock().push(task);
    }
}

impl ILinuxProcess for TestProcess {
    fn execve(&self, _: MemorySpace, _: u32) {
        unimplemented!(
            "TestProcess is intended for light-weight mock testing. Use task::Process instead, which also supports unit test"
        )
    }
}
