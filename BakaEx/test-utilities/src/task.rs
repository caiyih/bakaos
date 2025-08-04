use std::sync::{Arc, Weak};

use filesystem_abstractions::FileDescriptorTable;
use hermit_sync::SpinMutex;
use memory_space_abstractions::MemorySpace;
use mmu_abstractions::IMMU;
use task_abstractions::{status::TaskStatus, IProcess, ITask, UserTaskStatistics};
use trap_abstractions::ITaskTrapContext;

pub struct TestTask {
    tid: u32,
    tgid: u32,
    process: Option<Arc<dyn IProcess>>,
    status: SpinMutex<TaskStatus>,
    stats: UserTaskStatistics,
}

impl TestTask {
    pub fn new() -> TestTask {
        TestTask {
            tid: 0,
            tgid: 0,
            process: None,
            status: SpinMutex::new(TaskStatus::Running),
            stats: UserTaskStatistics::default(),
        }
    }

    pub fn build(self) -> Arc<dyn ITask> {
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

    pub fn with_process(mut self, process: Option<Arc<dyn IProcess>>) -> Self {
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

    fn process(&self) -> &std::sync::Arc<dyn task_abstractions::IProcess> {
        self.process.as_ref().unwrap()
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
        todo!()
    }

    fn trap_context_mut(&self) -> &mut dyn ITaskTrapContext {
        todo!()
    }

    fn execve(&self, _: &mut MemorySpace, _: &dyn ITaskTrapContext) {
        unimplemented!("TestTask is intended for light-weight mock testing. Use task::Task instead, which also supports unit test")
    }

    fn fork_thread(&self) -> Arc<dyn ITask> {
        unimplemented!("TestTask is intended for light-weight mock testing. Use task::Task instead, which also supports unit test")
    }

    fn fork_process(&self) -> Arc<dyn ITask> {
        unimplemented!("TestTask is intended for light-weight mock testing. Use task::Task instead, which also supports unit test")
    }
}

pub struct TestProcess {
    pub pid: u32,
    pub pgid: u32,
    pub parent: Option<Arc<dyn IProcess>>,
    pub threads: Vec<Arc<dyn ITask>>,
    pub children: Vec<Weak<dyn IProcess>>,
    pub memory_space: Option<SpinMutex<MemorySpace>>,
    pub fd_table: Option<SpinMutex<FileDescriptorTable>>,
    pub working_directory: String,
    pub main_thread: Option<TestTask>,
    pub exit_code: SpinMutex<Option<u8>>,
}

impl TestProcess {
    pub fn new() -> TestProcess {
        Self {
            pid: 0,
            pgid: 0,
            parent: None,
            threads: Vec::new(),
            children: Vec::new(),
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

    pub fn build(mut self) -> (Arc<dyn IProcess>, Arc<dyn ITask>) {
        let main_thread = self.main_thread.take().unwrap();

        let process = Arc::new(self);

        let main_thread = main_thread.with_process(Some(process.clone())).build();

        // TODO: Add main thread to threads.

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
        self.threads = threads;
        self
    }

    pub fn with_children(mut self, children: Vec<Weak<dyn IProcess>>) -> Self {
        self.children = children;
        self
    }

    pub fn with_memory_space(mut self, memory_space: Option<MemorySpace>) -> Self {
        self.memory_space = memory_space.map(|m| SpinMutex::new(m));
        self
    }

    pub fn with_fd_table(mut self, fd_table: Option<FileDescriptorTable>) -> Self {
        self.fd_table = fd_table.map(|f| SpinMutex::new(f));
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
        self.threads.clone()
    }

    fn children(&self) -> Vec<Arc<dyn IProcess>> {
        self.children
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

    fn mmu(&self) -> &SpinMutex<dyn IMMU> {
        unsafe { &self.memory_space().data_ptr().as_ref().unwrap().mmu() }
    }
}
